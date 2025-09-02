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

const STEMS_4S: [&str; 4] = ["drums", "bass", "other", "vocals"];
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
                "ONNX model not found. please ensure 'htdemucs.onnx' is in the 'models' directory."
            ));
        }

        let session = Session::builder()?
            .commit_from_file(model_path)
            .map_err(|e| anyhow!("failed to build session: {}", e))?;

        // print detailed input information
        println!("=== MODEL INPUTS ===");
        for (i, input) in session.inputs.iter().enumerate() {
            println!(
                "input {}: name='{}', type={:?}",
                i, input.name, input.input_type
            );
        }

        println!("=== MODEL OUTPUTS ===");
        for (i, output) in session.outputs.iter().enumerate() {
            println!(
                "output {}: name='{}', type={:?}",
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
                title: "preparing audio...".to_string(),
                description: format!(
                    "processing {:.1} seconds of audio",
                    n_samples as f32 / 44100.0
                ),
                progress: Some(10),
            },
        )?;

        println!(
            "input: {} channels, {} samples ({:.1}s)",
            n_channels,
            n_samples,
            n_samples as f32 / 44100.0
        );

        if n_samples <= MODEL_LENGTH {
            // audio is short enough to process in one go
            return self.process_single_chunk_with_progress(mixture, app_handle, 1, 1);
        }

        // calculate chunking parameters
        let hop_size = (MODEL_LENGTH as f32 * (1.0 - OVERLAP_RATIO)) as usize;
        let n_chunks = ((n_samples - MODEL_LENGTH) as f32 / hop_size as f32).ceil() as usize + 1;

        println!(
            "processing {} chunks with {}% overlap",
            n_chunks,
            (OVERLAP_RATIO * 100.0) as u8
        );

        // initialize output arrays for all stems
        let mut separated_stems: HashMap<String, Array2<f32>> = HashMap::new();
        let mut weight_sum = Array2::<f32>::zeros((n_channels, n_samples));

        for stem_name in STEMS_4S.iter() {
            separated_stems.insert(
                stem_name.to_string(),
                Array2::zeros((n_channels, n_samples)),
            );
        }

        // create a windowing function for smooth blending
        let window = self.create_blend_window(MODEL_LENGTH);

        // process each chunk
        for chunk_idx in 0..n_chunks {
            let start_sample = chunk_idx * hop_size;
            let end_sample = std::cmp::min(start_sample + MODEL_LENGTH, n_samples);
            let actual_chunk_size = end_sample - start_sample;

            println!(
                "chunk {}/{}: samples {} to {} (size: {})",
                chunk_idx + 1,
                n_chunks,
                start_sample,
                end_sample,
                actual_chunk_size
            );

            // extract chunk
            let mut chunk = Array2::zeros((n_channels, MODEL_LENGTH));
            chunk
                .slice_mut(s![.., 0..actual_chunk_size])
                .assign(&mixture.slice(s![.., start_sample..end_sample]));

            // process this chunk with progress tracking
            let chunk_results = self.process_single_chunk_with_progress(
                chunk,
                app_handle,
                chunk_idx + 1,
                n_chunks,
            )?;

            // apply windowing and add to output
            for (stem_name, stem_audio) in chunk_results {
                if let Some(output_stem) = separated_stems.get_mut(&stem_name) {
                    // apply window and add to output with overlap-add
                    for ch in 0..n_channels {
                        for i in 0..actual_chunk_size {
                            let global_idx = start_sample + i;
                            if global_idx < n_samples {
                                let window_weight = if actual_chunk_size == MODEL_LENGTH {
                                    window[i]
                                } else {
                                    1.0 // no windowing for the last partial chunk
                                };

                                output_stem[[ch, global_idx]] +=
                                    stem_audio[[ch, i]] * window_weight;
                                weight_sum[[ch, global_idx]] += window_weight;
                            }
                        }
                    }
                }
            }
        }

        // normalize by weight sum to complete overlap-add
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
                title: "finalizing...".to_string(),
                description: "completing separation process".to_string(),
                progress: Some(92),
            },
        )?;

        println!("completed separation of {} stems", separated_stems.len());
        Ok(separated_stems)
    }

    fn process_single_chunk_with_progress(
        &mut self,
        chunk: Array2<f32>,
        app_handle: &AppHandle,
        chunk_num: usize,
        total_chunks: usize,
    ) -> Result<HashMap<String, Array2<f32>>> {
        let original_n_samples = chunk.shape()[1];

        // calculate chunk progress range (15% to 90%)
        let chunk_progress_start = 15;
        let chunk_progress_end = 90;
        let chunk_progress_range = chunk_progress_end - chunk_progress_start;
        let current_chunk_start =
            chunk_progress_start + ((chunk_num - 1) * chunk_progress_range / total_chunks);

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "processing with model...".to_string(),
                description: format!(
                    "processing chunk {} of {} - preparing audio",
                    chunk_num, total_chunks
                ),
                progress: Some(current_chunk_start as u8),
            },
        )?;

        // pad to training length (MODEL_LENGTH)
        let mut padded_mix = Array2::zeros((2, MODEL_LENGTH));
        let copy_samples = std::cmp::min(original_n_samples, MODEL_LENGTH);
        padded_mix
            .slice_mut(s![.., 0..copy_samples])
            .assign(&chunk.slice(s![.., 0..copy_samples]));

        // compute STFT of the mixture (like their standalone_spec function)
        let stft_left = stft(&padded_mix.slice(s![0..1, ..]).to_owned());
        let stft_right = stft(&padded_mix.slice(s![1..2, ..]).to_owned());

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "processing with model...".to_string(),
                description: format!(
                    "processing chunk {} of {} - running AI model",
                    chunk_num, total_chunks
                ),
                progress: Some((current_chunk_start + 2) as u8),
            },
        )?;

        // prepare the two model inputs exactly as in their C++ code

        // input 0: time-domain mix [1, 2, 343980]
        let mix_input = padded_mix.insert_axis(Axis(0));

        // input 1: magnitude spectrogram with complex-as-channels [1, 4, 2048, 336]
        // this is sevagh's "magspec" - magnitude spectrogram from standalone_magnitude()
        let freq_bins = stft_left.shape()[0]; // 2048
        let time_frames = stft_left.shape()[1]; // 336
        let mut magspec = Array::<f32, _>::zeros((1, 4, freq_bins, time_frames));

        // fill exactly like sevagh's C++ buffer preparation
        for i in 0..freq_bins {
            for j in 0..time_frames {
                magspec[[0, 0, i, j]] = stft_left[[i, j]].re; // Real left
                magspec[[0, 1, i, j]] = stft_left[[i, j]].im; // Imag left
                magspec[[0, 2, i, j]] = stft_right[[i, j]].re; // Real right
                magspec[[0, 3, i, j]] = stft_right[[i, j]].im; // Imag right
            }
        }

        // run the core ONNX model (their "core demucs inference")
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

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "processing with model...".to_string(),
                description: format!(
                    "processing chunk {} of {} - converting to audio",
                    chunk_num, total_chunks
                ),
                progress: Some((current_chunk_start + 5) as u8),
            },
        )?;

        // process the output - following their post-processing logic
        if let Some(separated_output) = outputs.get("output") {
            let (output_shape, output_data) = separated_output.try_extract_tensor::<f32>()?;
            let separated_masks = Array::from_shape_vec(
                output_shape.iter().map(|&x| x as usize).collect::<Vec<_>>(),
                output_data.to_vec(),
            )?;

            println!("model output shape: {:?}", separated_masks.shape());
            // expected: [1, 4, 4, 2048, 336] = [batch, stems, channels, freq_bins, time_frames]
            let min_val = output_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_val = output_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            println!("model output range: {} to {}", min_val, max_val);

            // apply masks to original mixture spectrogram and convert back to waveform
            // this mirrors their standalone_mask + standalone_ispec functions
            let stem_names = ["drums", "bass", "other", "vocals"];
            let mut final_stems = HashMap::new();

            for (stem_idx, stem_name) in stem_names.iter().enumerate() {
                app_handle.emit(
                    "separation_progress",
                    &LoadingState {
                        title: "processing with model...".to_string(),
                        description: format!(
                            "processing chunk {} of {} - extracting track {} of 4",
                            chunk_num,
                            total_chunks,
                            stem_idx + 1,
                        ),
                        progress: Some((current_chunk_start + 6 + stem_idx) as u8),
                    },
                )?;
                println!("processing {} (stem {})", stem_name, stem_idx);

                let mut stem_waveform = Array2::zeros((2, original_n_samples));

                for ch in 0..2 {
                    // use only the real part as magnitude mask
                    let mask_slice = separated_masks.slice(s![0, stem_idx, ch * 2, .., ..]);

                    let original_stft = if ch == 0 { &stft_left } else { &stft_right };
                    let mut masked_spectrogram = Array2::zeros((freq_bins, time_frames));

                    for i in 0..freq_bins {
                        for j in 0..time_frames {
                            let original_complex = original_stft[[i, j]];
                            let magnitude_mask = mask_slice[[i, j]].clamp(0.0, 1.0); // clamp to 0-1 range

                            // apply as ratio mask to preserve phase
                            let original_magnitude = original_complex.norm();
                            let new_magnitude = original_magnitude * magnitude_mask;
                            let phase = original_complex.arg();

                            masked_spectrogram[[i, j]] = Complex::from_polar(new_magnitude, phase);
                        }
                    }
                    let waveform_channel =
                        istft_single_channel(masked_spectrogram, original_n_samples)?;

                    // copy to final output
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
                "expected 'output' not found. available: {:?}",
                output_names
            ))
        }
    }

    fn create_blend_window(&self, length: usize) -> Vec<f32> {
        let fade_length = (length as f32 * OVERLAP_RATIO / 2.0) as usize;
        let mut window = vec![1.0; length];

        // fade in at the beginning
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            window[i] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        // fade out at the end
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            window[length - 1 - i] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        window
    }
}
