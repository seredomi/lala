use crate::audio_io::{load_wav_to_tensor, save_tensor_to_wav};
use crate::demucs_model::DemucsModel;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tch::Device;

/// wrapper around demucs separation that returns file paths
pub fn separate_audio(
    input_path: &Path,
    output_dir: &Path,
    model_path: &Path,
) -> Result<HashMap<String, String>> {
    let device = if tch::Cuda::is_available() {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };

    let (audio_tensor, sample_rate) = load_wav_to_tensor(input_path, device)?;

    let demucs = DemucsModel::new(model_path)?;
    let stems = demucs.separate(&audio_tensor, |_current, _total| {
        // progress callback can be wired to emit events if needed
    })?;

    let mut output_paths = HashMap::new();

    for (stem_name, tensor) in stems {
        let output_path = output_dir.join(format!("stem_{}.wav", stem_name));
        save_tensor_to_wav(output_path.to_str().unwrap(), &tensor, sample_rate)?;
        output_paths.insert(stem_name, output_path.to_string_lossy().to_string());
    }

    Ok(output_paths)
}

/// fake transcription: piano wav → midi (takes 10 seconds)
pub fn transcribe_to_midi(input_wav: &Path, output_midi: &Path) -> Result<()> {
    println!(
        "transcribing {} to {}...",
        input_wav.display(),
        output_midi.display()
    );

    thread::sleep(Duration::from_secs(10));

    // fake output: just create an empty file
    std::fs::write(output_midi, b"fake midi data")?;

    println!("transcription complete");
    Ok(())
}

/// fake midi → pdf conversion (takes 10 seconds)
pub fn midi_to_pdf(input_midi: &Path, output_pdf: &Path) -> Result<()> {
    println!(
        "converting {} to {}...",
        input_midi.display(),
        output_pdf.display()
    );

    thread::sleep(Duration::from_secs(10));

    // fake output: just create an empty file
    std::fs::write(output_pdf, b"fake pdf data")?;

    println!("conversion complete");
    Ok(())
}
