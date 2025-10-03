use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tch::{nn, nn::ModuleT, Device, IndexOp, Kind, Tensor};

// hdemucs high configuration
const CHANNELS: i64 = 48;
const DEPTH: i64 = 6;
const GROWTH: f64 = 2.0;
const NFFT: i64 = 4096;
const KERNEL_SIZE: i64 = 8;
const STRIDE: i64 = 4;
const SAMPLE_RATE: i64 = 44100;

pub struct HDemucs {
    encoder: nn::Sequential,
    decoder: nn::Sequential,
    transformer: TransformerEncoder,
    device: Device,
    sources: Vec<String>,
}

impl HDemucs {
    pub fn new(vs: &nn::VarStore, num_sources: i64, device: Device) -> Self {
        let root = &vs.root();

        // encoder layers
        let mut encoder_layers = Vec::new();
        let mut ch = 2; // input channels (stereo)

        for i in 0..DEPTH {
            let out_ch = (CHANNELS as f64 * GROWTH.powi(i as i32)) as i64;

            encoder_layers.push(conv1d_block(
                root / format!("encoder.{}", i),
                ch,
                out_ch,
                KERNEL_SIZE,
                STRIDE,
            ));
            ch = out_ch;
        }

        let encoder = nn::seq().add_fn_t(move |xs, train| {
            let mut x = xs.shallow_clone();
            for layer in &encoder_layers {
                x = layer.forward_t(&x, train);
            }
            x
        });

        // transformer in the middle
        let transformer = TransformerEncoder::new(root / "transformer", ch);

        // decoder layers (reverse of encoder)
        let mut decoder_layers = Vec::new();
        for i in (0..DEPTH).rev() {
            let in_ch = (CHANNELS as f64 * GROWTH.powi(i as i32)) as i64;
            let out_ch = if i == 0 {
                num_sources * 2 // output channels
            } else {
                (CHANNELS as f64 * GROWTH.powi((i - 1) as i32)) as i64
            };

            decoder_layers.push(conv_transpose1d_block(
                root / format!("decoder.{}", DEPTH - 1 - i),
                in_ch,
                out_ch,
                KERNEL_SIZE,
                STRIDE,
            ));
        }

        let decoder = nn::seq().add_fn_t(move |xs, train| {
            let mut x = xs.shallow_clone();
            for layer in &decoder_layers {
                x = layer.forward_t(&x, train);
            }
            x
        });

        Self {
            encoder,
            decoder,
            transformer,
            device,
            sources: vec![
                "drums".to_string(),
                "bass".to_string(),
                "other".to_string(),
                "vocals".to_string(),
            ],
        }
    }

    pub fn forward(&self, mix: &Tensor) -> Tensor {
        // encode to compressed representation
        let encoded = self.encoder.forward_t(mix, false);

        // apply transformer
        let transformed = self.transformer.forward(&encoded);

        // decode to separated sources
        let decoded = self.decoder.forward_t(&transformed, false);

        // reshape to [batch, sources, channels, time]
        let batch_size = mix.size()[0];
        let time_dim = mix.size()[2];

        decoded.view([batch_size, 4, 2, time_dim])
    }

    pub fn separate_with_chunks(
        &self,
        mix: &Tensor,
        segment: f64,
        overlap: f64,
    ) -> Result<HashMap<String, Tensor>> {
        let sample_rate = SAMPLE_RATE as f64;
        let length = mix.size()[2];

        let chunk_len = (sample_rate * segment * (1.0 + overlap)) as i64;
        let overlap_frames = (overlap * sample_rate) as i64;

        let mut final_output = Tensor::zeros(&[1, 4, 2, length], (Kind::Float, self.device));

        let mut start = 0i64;
        let mut is_first = true;

        while start < length - overlap_frames {
            let end = (start + chunk_len).min(length);
            let chunk_size = end - start;

            // extract and pad chunk
            let chunk = mix.narrow(2, start, chunk_size);
            let padded_chunk = if chunk_size < chunk_len {
                let padding =
                    Tensor::zeros(&[1, 2, chunk_len - chunk_size], (Kind::Float, self.device));
                Tensor::cat(&[chunk, padding], 2)
            } else {
                chunk
            };

            // separate chunk
            let chunk_output = tch::no_grad(|| self.forward(&padded_chunk));

            // apply overlap-add with fading
            let faded_output = self.apply_fade(
                &chunk_output,
                overlap_frames,
                is_first,
                start + chunk_len >= length,
            );

            // add to final output
            let actual_size = chunk_size.min(faded_output.size()[3]);
            let output_slice = final_output.narrow(3, start, actual_size);
            let chunk_slice = faded_output.narrow(3, 0, actual_size);

            let combined = output_slice + chunk_slice;
            final_output.narrow(3, start, actual_size).copy_(&combined);

            // update for next iteration
            start += if is_first {
                chunk_len - overlap_frames
            } else {
                chunk_len
            };
            is_first = false;
        }

        // convert to hashmap
        let final_sources = final_output.squeeze_dim(0); // remove batch dim
        let mut result = HashMap::new();

        for (i, source_name) in self.sources.iter().enumerate() {
            let source_tensor = final_sources.select(0, i as i64);
            result.insert(source_name.clone(), source_tensor);
        }

        Ok(result)
    }

    fn apply_fade(
        &self,
        tensor: &Tensor,
        fade_frames: i64,
        is_first: bool,
        is_last: bool,
    ) -> Tensor {
        let mut result = tensor.shallow_clone();
        let length = tensor.size()[3];

        if !is_first {
            // fade in
            for i in 0..fade_frames.min(length) {
                let fade_factor = i as f64 / fade_frames as f64;
                let slice = result.narrow(3, i, 1);
                let faded = slice * fade_factor;
                result.narrow(3, i, 1).copy_(&faded);
            }
        }

        if !is_last {
            // fade out
            let fade_start = (length - fade_frames).max(0);
            for i in 0..fade_frames.min(length - fade_start) {
                let fade_factor = 1.0 - (i as f64 / fade_frames as f64);
                let pos = fade_start + i;
                let slice = result.narrow(3, pos, 1);
                let faded = slice * fade_factor;
                result.narrow(3, pos, 1).copy_(&faded);
            }
        }

        result
    }
}

// helper functions for building conv layers
fn conv1d_block(path: nn::Path, in_ch: i64, out_ch: i64, kernel: i64, stride: i64) -> impl ModuleT {
    nn::seq_t()
        .add(nn::conv1d(
            &path / "conv",
            in_ch,
            out_ch,
            kernel,
            nn::ConvConfig {
                stride,
                padding: kernel / 2,
                ..Default::default()
            },
        ))
        .add_fn(|xs| xs.relu())
        .add(nn::group_norm(
            &path / "norm",
            1,
            out_ch,
            Default::default(),
        ))
}

fn conv_transpose1d_block(
    path: nn::Path,
    in_ch: i64,
    out_ch: i64,
    kernel: i64,
    stride: i64,
) -> impl ModuleT {
    nn::seq_t()
        .add(nn::conv_transpose1d(
            &path / "conv_t",
            in_ch,
            out_ch,
            kernel,
            nn::ConvTransposeConfig {
                stride,
                padding: kernel / 2,
                ..Default::default()
            },
        ))
        .add_fn(|xs| xs.relu())
        .add(nn::group_norm(
            &path / "norm",
            1,
            out_ch,
            Default::default(),
        ))
}

// simplified transformer encoder
struct TransformerEncoder {
    layers: Vec<TransformerLayer>,
}

impl TransformerEncoder {
    fn new(path: nn::Path, dim: i64) -> Self {
        let mut layers = Vec::new();
        for i in 0..8 {
            // 8 transformer layers
            layers.push(TransformerLayer::new(&path / i, dim));
        }
        Self { layers }
    }

    fn forward(&self, x: &Tensor) -> Tensor {
        let mut out = x.shallow_clone();
        for layer in &self.layers {
            out = layer.forward(&out);
        }
        out
    }
}

struct TransformerLayer {
    self_attn: nn::Linear,
    feed_forward: nn::Sequential,
    norm1: nn::LayerNorm,
    norm2: nn::LayerNorm,
}

impl TransformerLayer {
    fn new(path: &nn::Path, dim: i64) -> Self {
        Self {
            self_attn: nn::linear(path / "self_attn", dim, dim, Default::default()),
            feed_forward: nn::seq()
                .add(nn::linear(
                    path / "ff" / "0",
                    dim,
                    dim * 4,
                    Default::default(),
                ))
                .add_fn(|xs| xs.relu())
                .add(nn::linear(
                    path / "ff" / "2",
                    dim * 4,
                    dim,
                    Default::default(),
                )),
            norm1: nn::layer_norm(path / "norm1", vec![dim], Default::default()),
            norm2: nn::layer_norm(path / "norm2", vec![dim], Default::default()),
        }
    }

    fn forward(&self, x: &Tensor) -> Tensor {
        // simplified transformer layer (would need full multi-head attention for production)
        let normed1 = self.norm1.forward(x);
        let attended = self.self_attn.forward(&normed1);
        let residual1 = x + attended;

        let normed2 = self.norm2.forward(&residual1);
        let ff_out = self.feed_forward.forward(&normed2);
        &residual1 + ff_out
    }
}
