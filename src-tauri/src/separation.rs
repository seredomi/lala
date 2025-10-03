use crate::audio_io::{load_wav_to_tensor, save_tensor_to_wav};
use crate::demucs_model::DemucsModel;
use anyhow::Result;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{
    Mutex, // atomic::{AtomicBool, Ordering},
           // Arc, Mutex,
};
use tauri::{command, AppHandle, Emitter};
use tch::{Device, Tensor};

#[derive(serde::Serialize, Clone)]
pub struct LoadingState {
    pub title: String,
    pub description: String,
    pub progress: Option<u32>,
}

// In-memory storage for stems and sample rate
lazy_static! {
    static ref SEPARATED_STEMS: Mutex<Option<HashMap<String, Tensor>>> = Mutex::new(None);
    static ref SAMPLE_RATE: Mutex<Option<u32>> = Mutex::new(None);
}

#[command]
pub async fn start_separation(app: AppHandle, file_path: String) -> Result<Vec<String>, String> {
    // Emit progress
    app.emit(
        "separation_progress",
        &LoadingState {
            title: "initializing...".to_string(),
            description: "loading model and audio".to_string(),
            progress: Some(0),
        },
    )
    .map_err(|e| format!("{:?}", e))?;

    // Load audio
    let device = if tch::Cuda::is_available() {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };
    let (audio_tensor, sample_rate) = load_wav_to_tensor(Path::new(&file_path), device)
        .map_err(|e| format!("Failed to load audio: {:?}", e))?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "running Demucs model...".to_string(),
            description: "initiating".to_string(),
            progress: Some(10),
        },
    )
    .map_err(|e| format!("{:?}", e))?;

    // Load model and run separation
    // let path = env::current_dir();
    println!("The current directory is {:?}", env::current_dir());
    let model_path = Path::new("models/hdemucs.pt");
    let demucs =
        DemucsModel::new(model_path).map_err(|e| format!("failed to load model: {:?}", e))?;
    let stems = demucs
        .separate(&audio_tensor, |current_chunk, total_chunks| {
            let _ = app.emit(
                "separation_progress",
                &LoadingState {
                    title: "running Demucs model...".to_string(),
                    description: format!("processing chunk {}/{}", current_chunk, total_chunks),
                    progress: Some(10 + 80 * current_chunk / total_chunks),
                },
            );
        })
        .map_err(|e| e.to_string())?;

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "finalizing...".to_string(),
            description: "preparing tracks for download".to_string(),
            progress: Some(95),
        },
    )
    .map_err(|e| format!("{:?}", e))?;

    let available_stems: Vec<String> = stems.keys().cloned().collect();

    // store stems and sample rate for download
    *SEPARATED_STEMS.lock().unwrap() = Some(stems);
    *SAMPLE_RATE.lock().unwrap() = Some(sample_rate);

    app.emit(
        "separation_progress",
        &LoadingState {
            title: "done".to_string(),
            description: "separation complete".to_string(),
            progress: Some(100),
        },
    )
    .map_err(|e| format!("{:?}", e))?;

    Ok(available_stems)
}

#[command]
pub async fn download_stem(stem_name: String, output_path: String) -> Result<(), String> {
    let stems = SEPARATED_STEMS.lock().unwrap();
    let sample_rate = SAMPLE_RATE.lock().unwrap();

    let stems = stems.as_ref().ok_or("No stems available")?;
    let sample_rate = sample_rate.ok_or("No sample rate available")?;

    let tensor = stems.get(&stem_name).ok_or("Stem not found")?;

    save_tensor_to_wav(&output_path, tensor, sample_rate)
        .map_err(|e| format!("Failed to save stem: {:?}", e))?;

    Ok(())
}

#[command]
pub async fn abort_separation() -> Result<(), String> {
    // todo: implement
    Ok(())
}
