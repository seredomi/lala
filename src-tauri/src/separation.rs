use crate::audio_io::{load_wav_to_tensor, save_tensor_to_wav};
use crate::separator::Separator;
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tch::{Device, Tensor};
use tokio::task;

#[derive(Serialize, Deserialize, Clone)]
pub struct LoadingState {
    pub title: String,
    pub description: String,
    pub progress: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadProgress {
    pub title: String,
    pub description: String,
    pub progress: Option<u8>,
}

lazy_static! {
    static ref SEPARATION_STATE: Arc<Mutex<Option<SeparationHandle>>> = Arc::new(Mutex::new(None));
    // store stems as individual tensors to avoid cloning issues
    static ref SEPARATED_STEMS: Arc<Mutex<Option<StemStorage>>> =
        Arc::new(Mutex::new(None));
    static ref SAMPLE_RATE: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
}

struct SeparationHandle {
    abort_flag: Arc<AtomicBool>,
}

// custom storage to handle tensor ownership
struct StemStorage {
    drums: Option<Tensor>,
    bass: Option<Tensor>,
    other: Option<Tensor>,
    vocals: Option<Tensor>,
    instrumental: Option<Tensor>,
}

impl StemStorage {
    fn new() -> Self {
        Self {
            drums: None,
            bass: None,
            other: None,
            vocals: None,
            instrumental: None,
        }
    }

    fn insert(&mut self, name: &str, tensor: Tensor) {
        match name {
            "drums" => self.drums = Some(tensor),
            "bass" => self.bass = Some(tensor),
            "other" => self.other = Some(tensor),
            "vocals" => self.vocals = Some(tensor),
            "instrumental" => self.instrumental = Some(tensor),
            _ => {}
        }
    }

    fn get(&self, name: &str) -> Option<&Tensor> {
        match name {
            "drums" => self.drums.as_ref(),
            "bass" => self.bass.as_ref(),
            "other" => self.other.as_ref(),
            "vocals" => self.vocals.as_ref(),
            "instrumental" => self.instrumental.as_ref(),
            _ => None,
        }
    }

    fn available_stems(&self) -> Vec<String> {
        let mut stems = Vec::new();
        if self.vocals.is_some() {
            stems.push("vocals".to_string());
        }
        if self.instrumental.is_some() {
            stems.push("instrumental".to_string());
        }
        if self.drums.is_some() {
            stems.push("drums".to_string());
        }
        if self.bass.is_some() {
            stems.push("bass".to_string());
        }
        if self.other.is_some() {
            stems.push("other".to_string());
        }
        stems
    }
}

#[tauri::command(async)]
pub async fn start_separation(app: AppHandle, file_path: String) -> Result<Vec<String>, String> {
    let abort_flag = Arc::new(AtomicBool::new(false));

    {
        let mut state_guard = SEPARATION_STATE.lock().unwrap();
        if state_guard.is_some() {
            return Err("separation already in progress".into());
        }
        *state_guard = Some(SeparationHandle {
            abort_flag: abort_flag.clone(),
        });
    }

    let app_clone = app.clone();
    let separation_task =
        task::spawn_blocking(move || perform_separation_task(&app_clone, &file_path, abort_flag));

    let result = match separation_task.await {
        Ok(Ok(stem_names)) => {
            app.emit(
                "separation_progress",
                &LoadingState {
                    title: "complete".to_string(),
                    description: "separation finished successfully".to_string(),
                    progress: Some(100),
                },
            )
            .unwrap();
            Ok(stem_names)
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            app.emit(
                "separation_error",
                &LoadingState {
                    title: "error".to_string(),
                    description: err_msg.clone(),
                    progress: None,
                },
            )
            .unwrap();
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("task failed: {}", e);
            app.emit(
                "separation_error",
                &LoadingState {
                    title: "fatal error".to_string(),
                    description: err_msg.clone(),
                    progress: None,
                },
            )
            .unwrap();
            Err(err_msg)
        }
    };

    *SEPARATION_STATE.lock().unwrap() = None;
    result
}

fn perform_separation_task(
    app: &AppHandle,
    file_path: &str,
    abort_flag: Arc<AtomicBool>,
) -> Result<Vec<String>> {
    app.emit(
        "separation_progress",
        &LoadingState {
            title: "initializing...".to_string(),
            description: "loading demucs model".to_string(),
            progress: Some(0),
        },
    )?;

    let separator = Separator::new()?;

    if abort_flag.load(Ordering::SeqCst) {
        return Err(anyhow::anyhow!("separation aborted"));
    }

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "loading audio...".to_string(),
            description: "reading and decoding audio file".to_string(),
            progress: Some(5),
        },
    )?;

    let device = if tch::Cuda::is_available() {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };

    let (audio_tensor, sample_rate) = load_wav_to_tensor(Path::new(file_path), device)?;

    if abort_flag.load(Ordering::SeqCst) {
        return Err(anyhow::anyhow!("separation aborted"));
    }

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "analyzing audio...".to_string(),
            description: format!(
                "loaded {:.1} seconds of audio",
                audio_tensor.size()[1] as f64 / 44100.0
            ),
            progress: Some(15),
        },
    )?;

    let separated_stems = separator.separate(audio_tensor, app, abort_flag)?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "finalizing...".to_string(),
            description: "preparing tracks for download".to_string(),
            progress: Some(95),
        },
    )?;

    // store results for download using our custom storage
    let mut storage = StemStorage::new();
    for (name, tensor) in separated_stems {
        storage.insert(&name, tensor);
    }

    let available_stems = storage.available_stems();

    *SEPARATED_STEMS.lock().unwrap() = Some(storage);
    *SAMPLE_RATE.lock().unwrap() = Some(sample_rate);

    // return available stem names (focus on vocals and instrumental)
    let filtered_stems: Vec<String> = available_stems
        .into_iter()
        .filter(|name| name == "vocals" || name == "instrumental")
        .collect();

    println!("separation complete. available stems: {:?}", filtered_stems);
    Ok(filtered_stems)
}

#[tauri::command(async)]
pub async fn download_stem(
    app: AppHandle,
    stem_name: String,
    output_path: String,
) -> Result<(), String> {
    let app_clone = app.clone();
    let stem_name_clone = stem_name.clone();
    let output_path_clone = output_path.clone();

    let download_task = task::spawn_blocking(move || {
        perform_download_task(&app_clone, &stem_name_clone, &output_path_clone)
    });

    match download_task.await {
        Ok(Ok(_)) => {
            app.emit(
                "download_progress",
                &DownloadProgress {
                    title: "download complete".to_string(),
                    description: format!("{} saved successfully", stem_name),
                    progress: Some(100),
                },
            )
            .unwrap();
            Ok(())
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            app.emit(
                "download_error",
                &DownloadProgress {
                    title: "download error".to_string(),
                    description: err_msg.clone(),
                    progress: None,
                },
            )
            .unwrap();
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("download task failed: {}", e);
            app.emit(
                "download_error",
                &DownloadProgress {
                    title: "download fatal error".to_string(),
                    description: err_msg.clone(),
                    progress: None,
                },
            )
            .unwrap();
            Err(err_msg)
        }
    }
}

fn perform_download_task(app: &AppHandle, stem_name: &str, output_path: &str) -> Result<()> {
    app.emit(
        "download_progress",
        &DownloadProgress {
            title: "starting download...".to_string(),
            description: format!("preparing {} for download", stem_name),
            progress: Some(0),
        },
    )?;

    let stems_guard = SEPARATED_STEMS.lock().unwrap();
    let sample_rate_guard = SAMPLE_RATE.lock().unwrap();

    if let (Some(stems), Some(sample_rate)) = (stems_guard.as_ref(), sample_rate_guard.as_ref()) {
        if let Some(stem_tensor) = stems.get(stem_name) {
            app.emit(
                "download_progress",
                &DownloadProgress {
                    title: "writing file...".to_string(),
                    description: format!("saving {} to disk", stem_name),
                    progress: Some(50),
                },
            )?;

            save_tensor_to_wav(output_path, stem_tensor, *sample_rate)?;

            app.emit(
                "download_progress",
                &DownloadProgress {
                    title: "finalizing...".to_string(),
                    description: "file saved successfully".to_string(),
                    progress: Some(90),
                },
            )?;

            Ok(())
        } else {
            Err(anyhow::anyhow!("stem '{}' not found", stem_name))
        }
    } else {
        Err(anyhow::anyhow!("no separated stems available for download"))
    }
}

#[tauri::command]
pub fn abort_separation() -> Result<(), String> {
    let state_guard = SEPARATION_STATE.lock().unwrap();
    if let Some(handle) = &*state_guard {
        handle.abort_flag.store(true, Ordering::SeqCst);
        println!("separation abort requested");
        Ok(())
    } else {
        Err("no separation to cancel".into())
    }
}
