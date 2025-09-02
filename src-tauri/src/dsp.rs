use anyhow::Result;
use ndarray::{s, Array1, Array2, Axis};
use rustfft::{num_complex::Complex, FftPlanner};

// These are the standard parameters for the Demucs model.
const FFT_SIZE: usize = 4096;
const HOP_SIZE: usize = 1024;

/// Creates a Hann window, which is standard for audio processing.
fn hann_window(size: usize) -> Array1<f32> {
    Array1::from_shape_fn(size, |i| {
        0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (size - 1) as f32).cos())
    })
}

/// Performs a Short-Time Fourier Transform (STFT).
/// Takes a 2D waveform `[channels, samples]` and returns a 3D complex spectrogram `[channels, freq_bins, time_frames]`.
pub fn stft(waveform: &Array2<f32>) -> Array2<Complex<f32>> {
    let n_samples = waveform.shape()[1];
    let n_channels = waveform.shape()[0];
    let window = hann_window(FFT_SIZE);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let model_expected_frames = 336;
    let hop_size_calculated = (343980 - FFT_SIZE) / (model_expected_frames - 1);

    // Process only the first channel for now (mono STFT)
    let mut spectrogram = Array2::zeros((2048, model_expected_frames));
    let channel_data = waveform.index_axis(Axis(0), 0); // Use first channel

    for i in 0..model_expected_frames {
        let start = i * hop_size_calculated;
        let end = std::cmp::min(start + FFT_SIZE, n_samples);
        let mut frame = Array1::zeros(FFT_SIZE);
        let actual_size = end - start;

        if actual_size > 0 {
            frame
                .slice_mut(s![0..actual_size])
                .assign(&channel_data.slice(s![start..end]));
        }
        frame *= &window;

        let mut buffer: Vec<Complex<f32>> = frame.iter().map(|&x| Complex::new(x, 0.0)).collect();
        fft.process(&mut buffer);

        for j in 0..2048 {
            if j < buffer.len() {
                spectrogram[[j, i]] = buffer[j];
            }
        }
    }

    spectrogram
}

/// Performs an Inverse Short-Time Fourier Transform (iSTFT).
/// Takes a 3D magnitude spectrogram `[stems, freq_bins, time_frames]`,
/// and a 2D complex mixture spectrogram (for phase information).
/// Returns a 3D waveform `[stems, channels, samples]`.
pub fn istft(
    separated_mags: Array2<f32>,
    mixture_phase: &Array2<f32>,
    n_samples: usize,
) -> Array1<f32> {
    let n_freq_bins = separated_mags.shape()[0];
    let n_frames = separated_mags.shape()[1];

    // Reconstruct the full complex spectrogram using the separated magnitude and mixture phase
    let full_spectrogram_hermitian = Array2::from_shape_fn((FFT_SIZE, n_frames), |(j, i)| {
        if j < n_freq_bins {
            let mag = separated_mags[[j, i]];
            let phase = mixture_phase[[j, i]];
            Complex::from_polar(mag, phase)
        } else if j > 0 && j < FFT_SIZE {
            // Apply Hermitian symmetry
            let mag = separated_mags[[FFT_SIZE - j, i]];
            let phase = -mixture_phase[[FFT_SIZE - j, i]];
            Complex::from_polar(mag, phase)
        } else {
            Complex::new(0.0, 0.0)
        }
    });

    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(FFT_SIZE);
    let window = hann_window(FFT_SIZE);

    let mut output_waveform = Array1::<f32>::zeros(n_samples);
    let mut window_sum = Array1::<f32>::zeros(n_samples);

    for i in 0..n_frames {
        let start = i * HOP_SIZE;
        let mut buffer: Vec<Complex<f32>> = full_spectrogram_hermitian.column(i).to_vec();

        ifft.process(&mut buffer);

        for j in 0..FFT_SIZE {
            let sample_pos = start + j;
            if sample_pos < n_samples {
                output_waveform[sample_pos] += buffer[j].re * window[j];
                window_sum[sample_pos] += window[j].powi(2);
            }
        }
    }

    // Normalize by window sum to avoid artifacts
    for i in 0..n_samples {
        if window_sum[i] > 1e-8 {
            output_waveform[i] /= window_sum[i];
        }
    }

    output_waveform
}

pub fn istft_single_channel(
    complex_spec: Array2<Complex<f32>>,
    n_samples: usize,
) -> Result<Vec<f32>> {
    let n_freq_bins = complex_spec.shape()[0];
    let n_frames = complex_spec.shape()[1];

    let mut planner = FftPlanner::new();
    let ifft = planner.plan_fft_inverse(FFT_SIZE);
    let window = hann_window(FFT_SIZE);

    // Use the exact same hop size calculation as in STFT
    let hop_size_calculated = (343980 - FFT_SIZE) / (336 - 1);

    let mut output_waveform = vec![0.0; n_samples];
    let mut window_sum = vec![0.0; n_samples];

    for frame_idx in 0..n_frames {
        // Prepare the full FFT buffer with proper Hermitian symmetry
        let mut buffer = vec![Complex::new(0.0, 0.0); FFT_SIZE];

        // Copy positive frequencies
        for freq_idx in 0..n_freq_bins.min(FFT_SIZE / 2 + 1) {
            buffer[freq_idx] = complex_spec[[freq_idx, frame_idx]];
        }

        // Hermitian symmetry for negative frequencies
        for freq_idx in 1..FFT_SIZE / 2 {
            if FFT_SIZE - freq_idx < buffer.len() {
                buffer[FFT_SIZE - freq_idx] = buffer[freq_idx].conj();
            }
        }

        ifft.process(&mut buffer);

        // Overlap-add with windowing (matching their approach)
        let start = frame_idx * hop_size_calculated;
        for i in 0..FFT_SIZE {
            let sample_pos = start + i;
            if sample_pos < n_samples {
                let windowed_sample = buffer[i].re * window[i];
                output_waveform[sample_pos] += windowed_sample;
                window_sum[sample_pos] += window[i] * window[i];
            }
        }
    }

    // Normalize by window overlap
    for i in 0..n_samples {
        if window_sum[i] > 1e-8 {
            output_waveform[i] /= window_sum[i];
        }
    }

    Ok(output_waveform)
}
