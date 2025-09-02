use anyhow::{anyhow, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use ndarray::{Array, Array2, Axis};
use std::path::Path;

/// loads a WAV file into a normalized f32 ndarray.
/// the output shape is [channels, samples].
pub fn load_wav_to_ndarray(path: &Path) -> Result<(Array2<f32>, u32)> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    // collect all samples into a single Vec<f32>
    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
        SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .collect::<Result<_, _>>()?
        }
    };

    let n_channels = spec.channels as usize;
    let n_samples = samples.len() / n_channels;

    // convert the flat Vec into an ndarray and reshape it
    let mut array = Array::from(samples)
        .into_shape((n_samples, n_channels))
        .map_err(|e| anyhow!("failed to reshape audio array: {}", e))?
        .t() // transpose to [channels, samples]
        .into_owned();

    // if mono, duplicate the channel to make it stereo as expected by the model
    if n_channels == 1 {
        let mono_channel = array.index_axis(Axis(0), 0).to_owned();
        array.push(Axis(0), mono_channel.view())?;
    }

    Ok((array, spec.sample_rate))
}

/// saves an ndarray [channels, samples] into a WAV file.
pub fn save_ndarray_to_wav(path: &str, array: &Array2<f32>, sample_rate: u32) -> Result<()> {
    let spec = WavSpec {
        channels: array.shape()[0] as u16,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;
    let interleaved_samples = array.t(); // transpose to [samples, channels]

    // write samples frame by frame
    for frame in interleaved_samples.axis_iter(Axis(0)) {
        for sample in frame {
            writer.write_sample(*sample)?;
        }
    }
    writer.finalize()?;
    Ok(())
}
