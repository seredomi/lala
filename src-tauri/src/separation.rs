use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Deserialize, Clone)]
pub struct LoadingState {
    pub title: String,
    pub description: String,
    pub progress: Option<u8>,
}

static SEPARATION_STATE: Mutex<Option<SeparationState>> = Mutex::new(None);

struct SeparationState {
    abort_flag: Arc<Mutex<bool>>,
}

#[tauri::command]
pub async fn start_separation(app: AppHandle, file_path: String) -> Result<(), String> {
    let mut state = SEPARATION_STATE
        .lock()
        .map_err(|_| "failed to acquire lock")?;

    // check if already separating
    if state.is_some() {
        return Err("song separation already in progress".to_string());
    }

    let abort_flag = Arc::new(Mutex::new(false));
    *state = Some(SeparationState {
        abort_flag: abort_flag.clone(),
    });

    // start separation in background thread
    thread::spawn(move || {
        for progress in (0..=100).step_by(10) {
            // check abort flag
            if let Ok(should_abort) = abort_flag.lock() {
                if *should_abort {
                    // emit cancellation event with current progress
                    let cancellation_state = LoadingState {
                        title: "cancelled".to_string(),
                        description: "separation has been cancelled".to_string(),
                        progress: Some(progress),
                    };

                    if let Err(e) = app.emit("separation_progress", &cancellation_state) {
                        eprintln!("failed to emit cancellation: {}", e);
                    }
                    break;
                }
            }

            let loading_state = LoadingState {
                title: "separating vocals from piano".to_string(),
                description: format!(
                    "processing {}...",
                    file_path.split('/').last().unwrap_or("file")
                ),
                progress: Some(progress),
            };

            // Emit progress event
            if let Err(e) = app.emit("separation_progress", &loading_state) {
                eprintln!("failed to emit progress: {}", e);
                break;
            }

            if progress >= 100 {
                break;
            }

            // wait 2 seconds before next update
            thread::sleep(Duration::from_secs(2));
        }

        // clear separation state when done
        if let Ok(mut state) = SEPARATION_STATE.lock() {
            *state = None;
        }

        // emit completion event (only if not aborted)
        if let Ok(should_abort) = abort_flag.lock() {
            if !*should_abort {
                let completion_state = LoadingState {
                    title: "complete".to_string(),
                    description: "vocals and piano have been separated successfully".to_string(),
                    progress: Some(100),
                };

                if let Err(e) = app.emit("separation_progress", &completion_state) {
                    eprintln!("failed to emit completion: {}", e);
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn abort_separation() -> Result<(), String> {
    let state = SEPARATION_STATE
        .lock()
        .map_err(|_| "Failed to acquire lock")?;

    if let Some(separation_state) = state.as_ref() {
        if let Ok(mut abort_flag) = separation_state.abort_flag.lock() {
            *abort_flag = true;
        }
        Ok(())
    } else {
        Err("no song separation in progress".to_string())
    }
}
