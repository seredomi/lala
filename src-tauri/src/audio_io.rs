use anyhow::Result;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;
use tch::{Device, Tensor};

/// loads a wav file into a pytorch tensor with shape [channels, samples]
/// normalizes to f32 and ensures stereo output
pub fn load_wav_to_tensor(path: &Path, device: Device) -> Result<(Tensor, u32)> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    // collect samples as f32
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
    let _n_samples = samples.len() / n_channels;

    // create tensor [channels, samples]
    let mut tensor = Tensor::from_slice(&samples)
        .to_device(device)
        .reshape(&[_n_samples as i64, n_channels as i64])
        .transpose(0, 1);

    // ensure stereo - duplicate mono channel if needed
    if n_channels == 1 {
        tensor = tensor.repeat(&[2, 1]);
    }

    Ok((tensor, spec.sample_rate))
}

/// saves a pytorch tensor [channels, samples] to wav file
pub fn save_tensor_to_wav(path: &str, tensor: &Tensor, sample_rate: u32) -> Result<()> {
    let tensor_cpu = tensor.to_device(Device::Cpu);
    let shape = tensor_cpu.size();
    let n_channels = shape[0] as u16;

    let spec = WavSpec {
        channels: n_channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;

    // convert to interleaved format
    let interleaved = tensor_cpu.transpose(0, 1).contiguous();
    let data: Vec<f32> = interleaved.try_into()?;

    for sample in data {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;
    Ok(())
}
