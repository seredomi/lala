use crate::audio_io::{load_wav_to_ndarray, save_ndarray_to_wav};
use crate::separator::Separator;
use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
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

lazy_static! {
    static ref SEPARATION_STATE: Arc<Mutex<Option<SeparationHandle>>> = Arc::new(Mutex::new(None));
}

struct SeparationHandle {
    abort_flag: Arc<AtomicBool>,
}

#[tauri::command(async)]
pub async fn start_separation(
    app: AppHandle,
    file_path: String,
) -> Result<HashMap<String, String>, String> {
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

    let input_path = Path::new(&file_path);
    let file_stem = input_path.file_stem().unwrap().to_str().unwrap();
    let parent_dir = input_path.parent().unwrap();
    let output_dir = parent_dir.join(format!("{}_stems", file_stem));
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    let app_clone = app.clone();
    let output_dir_clone = output_dir.clone();

    let separation_task = task::spawn_blocking(move || {
        perform_separation_task(&app_clone, &file_path, &output_dir_clone, abort_flag)
    });

    let result = match separation_task.await {
        Ok(Ok(paths)) => {
            let final_state = LoadingState {
                title: "Complete!".to_string(),
                description: "Separation finished successfully.".to_string(),
                progress: Some(100),
            };
            app.emit("separation_progress", &final_state).unwrap();
            Ok(paths)
        }
        Ok(Err(e)) => {
            let err_msg = e.to_string();
            let error_state = LoadingState {
                title: "Error".to_string(),
                description: err_msg.clone(),
                progress: None,
            };
            app.emit("separation_error", &error_state).unwrap();
            Err(err_msg)
        }
        Err(e) => {
            let err_msg = format!("Task panicked: {}", e);
            let error_state = LoadingState {
                title: "Fatal Error".to_string(),
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
    output_dir: &PathBuf,
    abort_flag: Arc<AtomicBool>,
) -> Result<HashMap<String, String>> {
    app.emit(
        "separation_progress",
        &LoadingState {
            title: "initializing...".to_string(),
            description: "loading separation model.".to_string(),
            progress: Some(0),
        },
    )?;
    let mut separator = Separator::new()?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "loading audio...".to_string(),
            description: "reading and decoding the audio file.".to_string(),
            progress: Some(5),
        },
    )?;
    let (mixture_array, sample_rate) = load_wav_to_ndarray(Path::new(file_path))?;

    let mut separated_stems = separator.separate(mixture_array, app, abort_flag)?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "saving files...".to_string(),
            description: "writing separated stems to disk.".to_string(),
            progress: Some(95),
        },
    )?;

    let instrumental = separated_stems.get("drums").unwrap().clone()
        + separated_stems.get("bass").unwrap()
        + separated_stems.get("other").unwrap();
    separated_stems.insert("instrumental".to_string(), instrumental);

    let mut output_paths = HashMap::new();
    for (stem_name, stem_data) in separated_stems.iter() {
        if stem_name == "vocals" || stem_name == "instrumental" {
            let output_path_str = output_dir
                .join(format!("{}.wav", stem_name))
                .to_str()
                .unwrap()
                .to_string();
            save_ndarray_to_wav(&output_path_str, stem_data, sample_rate)?;
            output_paths.insert(stem_name.clone(), output_path_str);
        }
    }

    Ok(output_paths)
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
