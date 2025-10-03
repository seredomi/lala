use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use tch::{CModule, Device, Kind, Tensor};

const SAMPLE_RATE: i64 = 44100;
const SEGMENT_LENGTH: i64 = 441000; // 10 seconds at 44.1khz
const OVERLAP_RATIO: f64 = 0.25;

pub struct DemucsModel {
    model: CModule,
    device: Device,
    stems: Vec<String>,
}

impl DemucsModel {
    pub fn new(model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow!(
                "model file not found at {}. please ensure demucs.pt is available.",
                model_path.display()
            ));
        }

        // use gpu if available, otherwise cpu
        let device = if tch::Cuda::is_available() {
            println!("using cuda device for inference");
            Device::Cuda(0)
        } else {
            println!("using cpu device for inference");
            Device::Cpu
        };

        let model = CModule::load_on_device(model_path, device)?;

        // demucs 4-stem model outputs
        let stems = vec![
            "drums".to_string(),
            "bass".to_string(),
            "other".to_string(),
            "vocals".to_string(),
        ];

        Ok(Self {
            model,
            device,
            stems,
        })
    }

    /// separates audio into stems using demucs model
    /// input: tensor [2, samples] (stereo audio)
    /// output: hashmap of stem tensors [2, samples]
    pub fn separate<F>(&self, audio: &Tensor, mut progress_cb: F) -> Result<HashMap<String, Tensor>>
    where
        F: FnMut(u32, u32),
    {
        let audio_shape = audio.size();
        let n_channels = audio_shape[0];
        let n_samples = audio_shape[1];

        println!(
            "separating audio: {} channels, {} samples ({:.1}s)",
            n_channels,
            n_samples,
            n_samples as f64 / SAMPLE_RATE as f64
        );

        if n_samples <= SEGMENT_LENGTH {
            // process short audio in one go
            let res = self.separate_segment(audio)?;
            progress_cb(90, 1);
            Ok(res)
        } else {
            // process long audio with overlap-add
            self.separate_with_overlap(audio, progress_cb)
        }
    }

    fn separate_segment(&self, audio: &Tensor) -> Result<HashMap<String, Tensor>> {
        let audio_shape = audio.size();
        let n_channels = audio_shape[0];
        let n_samples = audio_shape[1];

        // pad audio to model's expected length
        let mut padded = audio.shallow_clone();
        if n_samples < SEGMENT_LENGTH {
            let padding = Tensor::zeros(
                &[n_channels, SEGMENT_LENGTH - n_samples],
                (Kind::Float, self.device),
            );
            padded = Tensor::cat(&[padded, padding], 1);
        } else if n_samples > SEGMENT_LENGTH {
            padded = padded.narrow(1, 0, SEGMENT_LENGTH);
        }

        // normalize audio for model input - use unbiased=false for std
        let audio_std = padded.std(false);
        let normalized = &padded / (&audio_std + 1e-8);

        // add batch dimension: [1, 2, SEGMENT_LENGTH]
        let input = normalized.unsqueeze(0);

        // Debug print
        println!("Model input shape: {:?}", input.size());

        // run model inference
        let output = tch::no_grad(|| self.model.forward_ts(&[input]))?;

        // Debug print
        println!("Model output shape: {:?}", output.size());

        // output shape: [1, 4, 2, SEGMENT_LENGTH] (batch, stems, channels, time)
        let separated = output.squeeze_dim(0); // [4, 2, SEGMENT_LENGTH]

        // denormalize and trim to original length
        let denormalized = &separated * &audio_std;
        let trimmed = if n_samples < SEGMENT_LENGTH {
            denormalized.narrow(2, 0, n_samples)
        } else {
            denormalized
        };

        // convert to hashmap
        let mut result = HashMap::new();
        for (i, stem_name) in self.stems.iter().enumerate() {
            // use select instead of i() for tensor indexing
            let stem_audio = trimmed.select(0, i as i64).to_device(self.device);
            result.insert(stem_name.clone(), stem_audio);
        }

        Ok(result)
    }

    fn separate_with_overlap<F>(
        &self,
        audio: &Tensor,
        mut progress_cb: F,
    ) -> Result<HashMap<String, Tensor>>
    where
        F: FnMut(u32, u32),
    {
        let n_samples = audio.size()[1];
        let hop_size = (SEGMENT_LENGTH as f64 * (1.0 - OVERLAP_RATIO)) as i64;
        let n_chunks = ((n_samples - SEGMENT_LENGTH) as f64 / hop_size as f64).ceil() as usize + 1;

        println!(
            "processing {} chunks with {:.0}% overlap",
            n_chunks,
            OVERLAP_RATIO * 100.0
        );

        // initialize output tensors
        let mut separated_stems: HashMap<String, Tensor> = HashMap::new();
        let weight_sum = Tensor::zeros(&[2, n_samples], (Kind::Float, self.device));

        for stem_name in &self.stems {
            separated_stems.insert(
                stem_name.clone(),
                Tensor::zeros(&[2, n_samples], (Kind::Float, self.device)),
            );
        }

        // create blending window for smooth transitions
        let window = self.create_blend_window(SEGMENT_LENGTH);
        let window_tensor = Tensor::from_slice(&window).to_device(self.device);

        // process each chunk
        for chunk_idx in 0..n_chunks {
            let progress = 10 + ((chunk_idx as f32 / n_chunks as f32) * 80.0).round() as u32;
            progress_cb(progress, n_chunks as u32);
            let start = chunk_idx as i64 * hop_size;
            let end = (start + SEGMENT_LENGTH).min(n_samples);
            let actual_size = end - start;

            progress_cb(
                (chunk_idx + 1).try_into().unwrap(),
                n_chunks.try_into().unwrap(),
            );
            println!(
                "chunk {}/{}: samples {} to {} (size: {})",
                chunk_idx + 1,
                n_chunks,
                start,
                end,
                actual_size
            );

            // extract chunk with padding if needed
            let chunk = if actual_size == SEGMENT_LENGTH {
                audio.narrow(1, start, SEGMENT_LENGTH)
            } else {
                // pad short final chunk
                let partial = audio.narrow(1, start, actual_size);
                let padding = Tensor::zeros(
                    &[2, SEGMENT_LENGTH - actual_size],
                    (Kind::Float, self.device),
                );
                Tensor::cat(&[partial, padding], 1)
            };

            // Debug print
            println!("Chunk shape before segment separation: {:?}", chunk.size());

            // separate this chunk
            let chunk_results = self.separate_segment(&chunk)?;

            // apply windowing and overlap-add
            let current_window = if actual_size == SEGMENT_LENGTH {
                window_tensor.shallow_clone()
            } else {
                // no windowing for final partial chunk
                Tensor::ones(&[actual_size], (Kind::Float, self.device))
            };

            for (stem_name, stem_chunk) in chunk_results {
                if let Some(output_stem) = separated_stems.get_mut(&stem_name) {
                    // trim chunk to actual size and apply window
                    let trimmed_chunk = stem_chunk.narrow(1, 0, actual_size);
                    let windowed_chunk = &trimmed_chunk * current_window.unsqueeze(0);

                    // get mutable slice and add windowed chunk
                    let mut output_slice = output_stem.narrow(1, start, actual_size);
                    output_slice += &windowed_chunk;
                }
            }

            // update weight sum for normalization
            let mut weight_slice = weight_sum.narrow(1, start, actual_size);
            let weight_update = current_window.unsqueeze(0).expand_as(&weight_slice);
            weight_slice += &weight_update;
        }

        // normalize by weight sum to complete overlap-add
        for (_stem_name, stem_audio) in separated_stems.iter_mut() {
            let normalized = &*stem_audio / (&weight_sum + 1e-8);
            *stem_audio = normalized;
        }

        Ok(separated_stems)
    }

    fn create_blend_window(&self, length: i64) -> Vec<f32> {
        let fade_length = (length as f64 * OVERLAP_RATIO / 2.0) as usize;
        let mut window = vec![1.0; length as usize];

        // cosine fade-in
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            window[i] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        // cosine fade-out
        for i in 0..fade_length {
            let t = i as f32 / fade_length as f32;
            let idx = length as usize - 1 - i;
            window[idx] = 0.5 * (1.0 - (std::f32::consts::PI * t).cos());
        }

        window
    }
}
