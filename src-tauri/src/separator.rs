use crate::dsp::{istft, stft};
use anyhow::{anyhow, Result};
use ndarray::{s, Array, Array2};
use ort::session::Session;
use ort::value::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::separation::LoadingState;

const STEMS: [&str; 4] = ["drums", "bass", "other", "vocals"];

pub struct Separator {
    session: Session,
}

impl Separator {
    pub fn new() -> Result<Self> {
        let model_path = Path::new("models/htdemucs.onnx");
        if !model_path.exists() {
            return Err(anyhow!(
                "ONNX model not found. ensure 'htdemucs.onnx' is in the 'src-tauri/models' directory."
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
                title: "analyzing audio...".to_string(),
                description: "performing Short-Time Fourier Transform".to_string(),
                progress: Some(10),
            },
        )?;

        // 1. Perform STFT on the whole mixture
        let mixture_stft = stft(&mixture); // This line was missing!
        let mixture_mags = mixture_stft.mapv(|c| c.norm());
        let mixture_phase = mixture_stft.mapv(|c| c.arg());

        // 2. Prepare the magnitude spectrogram for the model
        // Model expects: [batch, channels, freq_bins, time_frames]
        let (freq_bins, time_frames) = (mixture_mags.shape()[0], mixture_mags.shape()[1]);
        let model_input = mixture_mags.into_shape((1, 1, freq_bins, time_frames))?;

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "AI processing...".to_string(),
                description: "running the source separation model".to_string(),
                progress: Some(40),
            },
        )?;

        // 3. Run inference
        let shape = model_input
            .shape()
            .iter()
            .map(|d| *d as i64)
            .collect::<Vec<_>>();
        let data = model_input.into_raw_vec();
        let input_value = Value::from_array((shape, data))?;

        // Try different approaches for the session run
        let outputs = self.session.run(ort::inputs!["input" => input_value])?;

        // 4. Process outputs
        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "reconstructing audio...".to_string(),
                description: "performing Inverse STFT for each stem".to_string(),
                progress: Some(80),
            },
        )?;

        // Extract the output tensor
        let output_tensor = outputs["output"].try_extract_tensor::<f32>()?;
        let (output_shape, output_data) = output_tensor;

        // Convert back to ndarray
        let separated_mags_tensor = Array::from_shape_vec(
            (
                output_shape[0] as usize,
                output_shape[1] as usize,
                output_shape[2] as usize,
                output_shape[3] as usize,
            ),
            output_data.to_vec(),
        )?;

        let mut final_stems = HashMap::new();
        for (i, stem_name) in STEMS.iter().enumerate() {
            let stem_mag_view = separated_mags_tensor.slice(s![0, i, .., ..]);

            // Reconstruct the stem waveform using iSTFT
            let stem_waveform_mono = istft(stem_mag_view.to_owned(), &mixture_phase, n_samples);

            // The model is mono-in, mono-out for the freq domain. We create a stereo output.
            let mut stem_waveform_stereo = Array2::zeros((2, n_samples));
            stem_waveform_stereo.row_mut(0).assign(&stem_waveform_mono);
            stem_waveform_stereo.row_mut(1).assign(&stem_waveform_mono);

            final_stems.insert(stem_name.to_string(), stem_waveform_stereo);
        }

        Ok(final_stems)
    }
}
