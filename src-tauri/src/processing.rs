use crate::audio_io::{load_wav_to_tensor, save_tensor_to_wav};
use crate::demucs_model::DemucsModel;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tch::Device;

/// wrapper around demucs separation that returns file paths
pub fn separate_audio<F>(
    input_path: &Path,
    output_dir: &Path,
    model_path: &Path,
    mut progress_callback: F,
) -> Result<HashMap<String, String>>
where
    F: FnMut(f32),
{
    let device = if tch::Cuda::is_available() {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };

    progress_callback(0.03); // loading

    let (audio_tensor, sample_rate) = load_wav_to_tensor(input_path, device)?;

    progress_callback(0.06); // loaded

    let demucs = DemucsModel::new(model_path)?;

    progress_callback(0.1); // model loaded

    let stems = demucs.separate(&audio_tensor, |current, total| {
        // map separation progress to 0.3 - 0.9 range
        let separation_progress = current as f32 / total as f32;
        let overall_progress = 0.1 + (separation_progress * 0.85 / 100.0);
        progress_callback(overall_progress);
    })?;

    progress_callback(0.95); // separation complete, saving

    let mut output_paths = HashMap::new();

    for (stem_name, tensor) in stems {
        let output_path = output_dir.join(format!("stem_{}.wav", stem_name));
        save_tensor_to_wav(output_path.to_str().unwrap(), &tensor, sample_rate)?;
        output_paths.insert(stem_name, output_path.to_string_lossy().to_string());
    }

    progress_callback(1.0); // done

    Ok(output_paths)
}

/// fake transcription with progress
pub fn transcribe_to_midi<F>(
    input_wav: &Path,
    output_midi: &Path,
    mut progress_callback: F,
) -> Result<()>
where
    F: FnMut(f32),
{
    println!(
        "transcribing {} to {}...",
        input_wav.display(),
        output_midi.display()
    );

    // simulate progress over 10 seconds
    for i in 0..=10 {
        thread::sleep(Duration::from_secs(1));
        progress_callback(i as f32 / 10.0);
    }

    // fake output: just create an empty file
    std::fs::write(output_midi, b"fake midi data")?;

    println!("transcription complete");
    Ok(())
}

/// fake midi → pdf conversion with progress
pub fn midi_to_pdf<F>(input_midi: &Path, output_pdf: &Path, mut progress_callback: F) -> Result<()>
where
    F: FnMut(f32),
{
    println!(
        "converting {} to {}...",
        input_midi.display(),
        output_pdf.display()
    );

    // simulate progress over 10 seconds
    for i in 0..=10 {
        thread::sleep(Duration::from_secs(1));
        progress_callback(i as f32 / 10.0);
    }

    // fake output: just create an empty file
    std::fs::write(output_pdf, b"fake pdf data")?;

    println!("conversion complete");
    Ok(())
}
