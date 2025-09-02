use crate::audio_io::{load_wav_to_ndarray, save_ndarray_to_wav};
use crate::separator::Separator;
use anyhow::Result;
use lazy_static::lazy_static;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
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
    static ref SEPARATED_STEMS: Arc<Mutex<Option<HashMap<String, Array2<f32>>>>> =
        Arc::new(Mutex::new(None));
    static ref SAMPLE_RATE: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
}

struct SeparationHandle {
    abort_flag: Arc<AtomicBool>,
}

#[tauri::command(async)]
pub async fn start_separation(app: AppHandle, file_path: String) -> Result<Vec<String>, String> {
    // Fixed return type
    let abort_flag = Arc::new(AtomicBool::new(false));
    {
        let mut state_guard = SEPARATION_STATE.lock().unwrap();
        if state_guard.is_some() {
            return Err("Separation already in progress.".into());
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
            let final_state = LoadingState {
                title: "complete".to_string(),
                description: "separation finished successfully".to_string(),
                progress: Some(100),
            };
            app.emit("separation_progress", &final_state).unwrap();
            Ok(stem_names)
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            let error_state = LoadingState {
                title: "error".to_string(),
                description: err_msg.clone(),
                progress: None,
            };
            app.emit("separation_error", &error_state).unwrap();
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("task panicked: {}", e);
            let error_state = LoadingState {
                title: "fatal error".to_string(),
                description: err_msg.clone(),
                progress: None,
            };
            app.emit("separation_error", &error_state).unwrap();
            Err(err_msg)
        }
    };

    // clear the state regardless of outcome
    *SEPARATION_STATE.lock().unwrap() = None;
    result
}

fn perform_separation_task(
    app: &AppHandle,
    file_path: &str,
    abort_flag: Arc<AtomicBool>,
) -> Result<Vec<String>> {
    // Fixed function signature and return type
    app.emit(
        "separation_progress",
        &LoadingState {
            title: "initializing...".to_string(),
            description: "loading model".to_string(),
            progress: Some(0),
        },
    )?;
    let mut separator = Separator::new()?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "loading audio...".to_string(),
            description: "reading and decoding the audio file".to_string(),
            progress: Some(5),
        },
    )?;
    let (mixture_array, sample_rate) = load_wav_to_ndarray(Path::new(file_path))?;

    let mut separated_stems = separator.separate(mixture_array, app, abort_flag)?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "finalizing...".to_string(),
            description: "preparing tracks for download".to_string(),
            progress: Some(95),
        },
    )?;

    let instrumental = if let (Some(drums), Some(bass), Some(other)) = (
        separated_stems.get("drums"),
        separated_stems.get("bass"),
        separated_stems.get("other"),
    ) {
        drums.clone() + bass + other
    } else {
        return Err(anyhow::anyhow!(
            "missing expected stems for instrumental creation"
        ));
    };
    separated_stems.insert("instrumental".to_string(), instrumental);

    // store in memory for later download
    *SEPARATED_STEMS.lock().unwrap() = Some(separated_stems.clone());
    *SAMPLE_RATE.lock().unwrap() = Some(sample_rate);

    // return available stem names
    let available_stems: Vec<String> = separated_stems
        .keys()
        .filter(|&name| name == "vocals" || name == "instrumental")
        .cloned()
        .collect();

    Ok(available_stems)
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
            let final_state = DownloadProgress {
                title: "download complete".to_string(),
                description: format!("{} saved successfully", stem_name),
                progress: Some(100),
            };
            app.emit("download_progress", &final_state).unwrap();
            Ok(())
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            let error_state = DownloadProgress {
                title: "download error".to_string(),
                description: err_msg.clone(),
                progress: None,
            };
            app.emit("download_error", &error_state).unwrap();
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("download task panicked: {}", e);
            let error_state = DownloadProgress {
                title: "download fatal error".to_string(),
                description: err_msg.clone(),
                progress: None,
            };
            app.emit("download_error", &error_state).unwrap();
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
        if let Some(stem_data) = stems.get(stem_name) {
            app.emit(
                "download_progress",
                &DownloadProgress {
                    title: "writing file...".to_string(),
                    description: format!("saving {} to disk", stem_name),
                    progress: Some(50),
                },
            )?;

            save_ndarray_to_wav(output_path, stem_data, *sample_rate)?;

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
        Ok(())
    } else {
        Err("no separation to cancel".into())
    }
}
