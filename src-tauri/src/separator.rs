use crate::dsp::{istft_single_channel, stft};
use anyhow::{anyhow, Result};
use ndarray::{s, Array, Array2, Axis, Zip};
use ort::session::Session;
use ort::value::Value;
use rustfft::num_complex::Complex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::separation::LoadingState;

const STEMS_6S: [&str; 6] = ["drums", "bass", "other", "vocals", "piano", "guitar"];
const MODEL_LENGTH: usize = 343980; // Samples the model expects
const OVERLAP_RATIO: f32 = 0.25; // 25% overlap between chunks

pub struct Separator {
    session: Session,
}

impl Separator {
    pub fn new() -> Result<Self> {
        let model_path = Path::new("models/htdemucs.onnx");
        if !model_path.exists() {
            return Err(anyhow!(
                "ONNX model not found. Please ensure 'htdemucs_6s.onnx' is in the 'models' directory."
            ));
        }

        let session = Session::builder()?
            .commit_from_file(model_path)
            .map_err(|e| anyhow!("Failed to build session: {}", e))?;

        // Print detailed input information
        println!("=== MODEL INPUTS ===");
        for (i, input) in session.inputs.iter().enumerate() {
            println!(
                "Input {}: name='{}', type={:?}",
                i, input.name, input.input_type
            );
        }

        println!("=== MODEL OUTPUTS ===");
        for (i, output) in session.outputs.iter().enumerate() {
            println!(
                "Output {}: name='{}', type={:?}",
                i, output.name, output.output_type
            );
        }

        Ok(Self { session })
    }

    pub fn separate(
        &mut self,
        mixture: Array2<f32>,
        app_handle: &AppHandle,
        _abort_flag: Arc<AtomicBool>,
    ) -> Result<HashMap<String, Array2<f32>>> {
        let (n_channels, n_samples) = (mixture.shape()[0], mixture.shape()[1]);

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "Preparing Audio...".to_string(),
                description: format!(
                    "Processing {:.1} seconds of audio",
                    n_samples as f32 / 44100.0
                ),
                progress: Some(10),
            },
        )?;

        println!(
            "Input: {} channels, {} samples ({:.1}s)",
            n_channels,
            n_samples,
            n_samples as f32 / 44100.0
        );

        if n_samples <= MODEL_LENGTH {
            // Audio is short enough to process in one go
            return self.process_single_chunk(mixture, app_handle);
        }

        // Calculate chunking parameters
        let hop_size = (MODEL_LENGTH as f32 * (1.0 - OVERLAP_RATIO)) as usize;
        let n_chunks = ((n_samples - MODEL_LENGTH) as f32 / hop_size as f32).ceil() as usize + 1;

        println!(
            "Processing {} chunks with {}% overlap",
            n_chunks,
            (OVERLAP_RATIO * 100.0) as u8
        );

        // Initialize output arrays for all stems
        let mut separated_stems: HashMap<String, Array2<f32>> = HashMap::new();
        let mut weight_sum = Array2::<f32>::zeros((n_channels, n_samples));

        for stem_name in STEMS_6S.iter() {
            separated_stems.insert(
                stem_name.to_string(),
                Array2::zeros((n_channels, n_samples)),
            );
        }

        // Create a windowing function for smooth blending
        let window = self.create_blend_window(MODEL_LENGTH);

        // Process each chunk
        for chunk_idx in 0..n_chunks {
            let start_sample = chunk_idx * hop_size;
            let end_sample = std::cmp::min(start_sample + MODEL_LENGTH, n_samples);
            let actual_chunk_size = end_sample - start_sample;

            println!(
                "Chunk {}/{}: samples {} to {} (size: {})",
                chunk_idx + 1,
                n_chunks,
                start_sample,
                end_sample,
                actual_chunk_size
            );

            // Extract chunk
            let mut chunk = Array2::zeros((n_channels, MODEL_LENGTH));
            chunk
                .slice_mut(s![.., 0..actual_chunk_size])
                .assign(&mixture.slice(s![.., start_sample..end_sample]));

            // Process this chunk
            let chunk_results = self.process_single_chunk(chunk, app_handle)?;

            // Apply windowing and add to output
            for (stem_name, stem_audio) in chunk_results {
                if let Some(output_stem) = separated_stems.get_mut(&stem_name) {
                    // Apply window and add to output with overlap-add
                    for ch in 0..n_channels {
                        for i in 0..actual_chunk_size {
                            let global_idx = start_sample + i;
                            if global_idx < n_samples {
                                let window_weight = if actual_chunk_size == MODEL_LENGTH {
                                    window[i]
                                } else {
                                    1.0 // No windowing for the last partial chunk
                                };

                                output_stem[[ch, global_idx]] +=
                                    stem_audio[[ch, i]] * window_weight;
                                weight_sum[[ch, global_idx]] += window_weight;
                            }
                        }
                    }
                }
            }

            // Update progress
            let progress = 20 + ((chunk_idx + 1) * 60 / n_chunks) as u8;
            app_handle.emit(
                "separation_progress",
                &LoadingState {
                    title: "AI Processing...".to_string(),
                    description: format!("Processing chunk {} of {}", chunk_idx + 1, n_chunks),
                    progress: Some(progress),
                },
            )?;
        }

        // Normalize by weight sum to complete overlap-add
        for (_stem_name, stem_audio) in separated_stems.iter_mut() {
            Zip::from(stem_audio)
                .and(&weight_sum)
                .for_each(|audio_sample, &weight| {
                    if weight > 0.0 {
                        *audio_sample /= weight;
                    }
                });
        }

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "Finalizing...".to_string(),
                description: "Completing separation process".to_string(),
                progress: Some(90),
            },
        )?;

        println!("Completed separation of {} stems", separated_stems.len());
        Ok(separated_stems)
    }

    fn process_single_chunk(
        &mut self,
        chunk: Array2<f32>,
        _app_handle: &AppHandle,
    ) -> Result<HashMap<String, Array2<f32>>> {
        let original_n_samples = chunk.shape()[1];

        // Step 1: Pad to training length (MODEL_LENGTH)
        let mut padded_mix = Array2::zeros((2, MODEL_LENGTH));
        let copy_samples = std::cmp::min(original_n_samples, MODEL_LENGTH);
        padded_mix
            .slice_mut(s![.., 0..copy_samples])
            .assign(&chunk.slice(s![.., 0..copy_samples]));

        // Step 2: Compute STFT of the mixture (like their standalone_spec function)
        let stft_left = stft(&padded_mix.slice(s![0..1, ..]).to_owned());
        let stft_right = stft(&padded_mix.slice(s![1..2, ..]).to_owned());

        // Step 3: Prepare the two model inputs exactly as in their C++ code

        // Input 0: time-domain mix [1, 2, 343980]
        let mix_input = padded_mix.insert_axis(Axis(0));

        // Input 1: magnitude spectrogram with complex-as-channels [1, 4, 2048, 336]
        // This is their "magspec" - magnitude spectrogram from standalone_magnitude()
        let freq_bins = stft_left.shape()[0]; // 2048
        let time_frames = stft_left.shape()[1]; // 336
        let mut magspec = Array::<f32, _>::zeros((1, 4, freq_bins, time_frames));

        // Fill exactly like their C++ buffer preparation
        for i in 0..freq_bins {
            for j in 0..time_frames {
                magspec[[0, 0, i, j]] = stft_left[[i, j]].re; // Real left
                magspec[[0, 1, i, j]] = stft_left[[i, j]].im; // Imag left
                magspec[[0, 2, i, j]] = stft_right[[i, j]].re; // Real right
                magspec[[0, 3, i, j]] = stft_right[[i, j]].im; // Imag right
            }
        }

        // Step 4: Run the core ONNX model (their "core demucs inference")
        let mix_tensor_shape: Vec<i64> = mix_input.shape().iter().map(|&x| x as i64).collect();
        let magspec_tensor_shape: Vec<i64> = magspec.shape().iter().map(|&x| x as i64).collect();

        let mix_data = mix_input.into_raw_vec();
        let magspec_data = magspec.into_raw_vec();

        let mix_value = Value::from_array((mix_tensor_shape, mix_data))?;
        let magspec_value = Value::from_array((magspec_tensor_shape, magspec_data))?;

        let outputs = self.session.run(ort::inputs![
            "input" => mix_value,
            "onnx::ReduceMean_1" => magspec_value
        ])?;

        // Step 5: Process the output - following their post-processing logic
        if let Some(separated_output) = outputs.get("output") {
            let (output_shape, output_data) = separated_output.try_extract_tensor::<f32>()?;
            let separated_masks = Array::from_shape_vec(
                output_shape.iter().map(|&x| x as usize).collect::<Vec<_>>(),
                output_data.to_vec(),
            )?;

            println!("Model output shape: {:?}", separated_masks.shape());
            // Expected: [1, 4, 4, 2048, 336] = [batch, stems, channels, freq_bins, time_frames]

            // Step 6: Apply masks to original mixture spectrogram and convert back to waveform
            // This mirrors their standalone_mask + standalone_ispec functions
            let stem_names = ["drums", "bass", "other", "vocals"];
            let mut final_stems = HashMap::new();

            for (stem_idx, stem_name) in stem_names.iter().enumerate() {
                println!("Processing {} (stem {})", stem_name, stem_idx);

                let mut stem_waveform = Array2::zeros((2, original_n_samples));

                for ch in 0..2 {
                    // Try treating the output as magnitude spectrograms instead of masks
                    let real_part_slice = separated_masks.slice(s![0, stem_idx, ch * 2, .., ..]);
                    let imag_part_slice =
                        separated_masks.slice(s![0, stem_idx, ch * 2 + 1, .., ..]);

                    // Reconstruct the complex spectrogram directly from model output
                    let mut stem_spectrogram = Array2::zeros((freq_bins, time_frames));

                    for i in 0..freq_bins {
                        for j in 0..time_frames {
                            let real_val = real_part_slice[[i, j]];
                            let imag_val = imag_part_slice[[i, j]];
                            stem_spectrogram[[i, j]] = Complex::new(real_val, imag_val);
                        }
                    }

                    // Convert directly to waveform
                    let waveform_channel =
                        istft_single_channel(stem_spectrogram, original_n_samples)?;

                    for sample_idx in 0..original_n_samples {
                        stem_waveform[[ch, sample_idx]] = waveform_channel[sample_idx];
                    }
                }

                final_stems.insert(stem_name.to_string(), stem_waveform);
            }

            Ok(final_stems)
        } else {
            let output_names: Vec<&str> = outputs.keys().collect();
            Err(anyhow!(
                "Expected 'output' not found. Available: {:?}",
                output_names
            ))
        }
    }

    fn create_blend_window(&self, length: usize) -> Vec<f32> {
        let fade_length = (length as f32 * OVERLAP_RATIO / 2.0) as usize;
        let mut window = vec![1.0; length];

        // Fade in at the beginning
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            window[i] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        // Fade out at the end
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            window[length - 1 - i] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        window
    }
}
