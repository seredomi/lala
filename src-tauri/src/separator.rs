use crate::dsp::{istft, stft};
use anyhow::{anyhow, Result};
use ndarray::{Array, Array2, Array3, Axis}; // Removed unused 's'
use ort::session::Session;
use ort::value::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::separation::LoadingState;

const STEMS: [&str; 4] = ["drums", "bass", "other", "vocals"];
const MODEL_FREQ_BINS: usize = 2048;
const MODEL_TIME_FRAMES: usize = 256;

pub struct Separator {
    session: Session,
}

impl Separator {
    pub fn new() -> Result<Self> {
        let model_path = Path::new("models/htdemucs.onnx");
        if !model_path.exists() {
            return Err(anyhow!(
                "ONNX model not found. Please ensure 'htdemucs.onnx' from your Python export is in the 'models' directory."
            ));
        }

        let session = Session::builder()?
            .commit_from_file(model_path)
            .map_err(|e| anyhow!("Failed to build session: {}", e))?;

        Ok(Self { session })
    }

    pub fn separate(
        &mut self,
        mixture: Array2<f32>,
        app_handle: &AppHandle,
        _abort_flag: Arc<AtomicBool>,
    ) -> Result<HashMap<String, Array2<f32>>> {
        let n_samples = mixture.shape()[1];

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "Analyzing Audio...".to_string(),
                description: "Performing Short-Time Fourier Transform.".to_string(),
                progress: Some(10),
            },
        )?;

        // 1. Perform STFT on the mixture (use only first channel for now)
        // Create an owned 2D array from the first channel
        let mono_mixture =
            Array2::from_shape_vec((1, mixture.shape()[1]), mixture.row(0).to_vec())?;
        let mixture_stft = stft(&mono_mixture);
        let mixture_phase = mixture_stft.mapv(|c| c.arg());

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "AI Processing...".to_string(),
                description: "Processing audio chunks through the model.".to_string(),
                progress: Some(40),
            },
        )?;

        // 2. Process in chunks of MODEL_TIME_FRAMES
        let shape = mixture_stft.shape();
        let n_freq_bins = shape[0];
        let total_time_frames = shape[1];
        let n_chunks = (total_time_frames + MODEL_TIME_FRAMES - 1) / MODEL_TIME_FRAMES; // Ceiling division

        // Initialize output arrays for all stems
        let mut all_stems_output =
            Array3::<f32>::zeros((STEMS.len(), n_freq_bins, total_time_frames));

        for chunk_idx in 0..n_chunks {
            let start_frame = chunk_idx * MODEL_TIME_FRAMES;
            let end_frame = std::cmp::min(start_frame + MODEL_TIME_FRAMES, total_time_frames);
            let chunk_size = end_frame - start_frame;

            // Create input tensor for this chunk
            let mut model_input = Array::zeros((1, 4, MODEL_FREQ_BINS, MODEL_TIME_FRAMES));

            // Fill the input tensor
            for freq_idx in 0..std::cmp::min(n_freq_bins, MODEL_FREQ_BINS) {
                for time_idx in 0..chunk_size {
                    let complex_val = mixture_stft[[freq_idx, start_frame + time_idx]];
                    // Channel 0: real part of left channel
                    model_input[[0, 0, freq_idx, time_idx]] = complex_val.re;
                    // Channel 1: imaginary part of left channel
                    model_input[[0, 1, freq_idx, time_idx]] = complex_val.im;
                    // Channel 2: real part of right channel (duplicate for stereo)
                    model_input[[0, 2, freq_idx, time_idx]] = complex_val.re;
                    // Channel 3: imaginary part of right channel
                    model_input[[0, 3, freq_idx, time_idx]] = complex_val.im;
                }
            }

            // Run model inference for this chunk
            let tensor_shape = model_input
                .shape()
                .iter()
                .map(|d| *d as i64)
                .collect::<Vec<_>>();
            let data = model_input.into_raw_vec();
            let input_value = Value::from_array((tensor_shape, data))?;
            let outputs = self.session.run(ort::inputs!["input" => input_value])?;

            // Extract output for this chunk
            let output_tensor = outputs["output"].try_extract_tensor::<f32>()?;
            let (output_shape, output_data) = output_tensor;

            let chunk_output = Array::from_shape_vec(
                (
                    output_shape[0] as usize,
                    output_shape[1] as usize,
                    output_shape[2] as usize,
                    output_shape[3] as usize,
                ),
                output_data.to_vec(),
            )?;

            // Copy this chunk's output to the full output arrays
            for stem_idx in 0..STEMS.len() {
                for freq_idx in 0..std::cmp::min(output_shape[2] as usize, n_freq_bins) {
                    for time_idx in 0..chunk_size {
                        all_stems_output[[stem_idx, freq_idx, start_frame + time_idx]] =
                            chunk_output[[0, stem_idx, freq_idx, time_idx]];
                    }
                }
            }

            // Update progress
            let progress = 40 + ((chunk_idx + 1) * 40 / n_chunks) as u8;
            app_handle.emit(
                "separation_progress",
                &LoadingState {
                    title: "AI Processing...".to_string(),
                    description: format!("Processing chunk {} of {}", chunk_idx + 1, n_chunks),
                    progress: Some(progress),
                },
            )?;
        }

        // 3. Reconstruct waveforms using iSTFT
        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "Reconstructing Audio...".to_string(),
                description: "Converting back to waveform.".to_string(),
                progress: Some(85),
            },
        )?;

        let mut final_stems = HashMap::new();
        for (stem_idx, stem_name) in STEMS.iter().enumerate() {
            let stem_magnitudes = all_stems_output.index_axis(Axis(0), stem_idx);
            let stem_waveform_mono = istft(stem_magnitudes.to_owned(), &mixture_phase, n_samples);

            // Create stereo output
            let mut stem_waveform_stereo = Array2::zeros((2, n_samples));
            stem_waveform_stereo.row_mut(0).assign(&stem_waveform_mono);
            stem_waveform_stereo.row_mut(1).assign(&stem_waveform_mono);

            final_stems.insert(stem_name.to_string(), stem_waveform_stereo);
        }

        Ok(final_stems)
    }
}
