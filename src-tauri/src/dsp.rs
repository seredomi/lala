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
    let _n_channels = waveform.shape()[0];
    let n_samples = waveform.shape()[1];
    let window = hann_window(FFT_SIZE);
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    let n_frames = (n_samples - FFT_SIZE) / HOP_SIZE + 1;
    // We only need the first channel for processing, as the model expects mono input for its freq domain.
    let mut spectrogram = Array2::zeros((FFT_SIZE / 2 + 1, n_frames));
    let channel_data = waveform.index_axis(Axis(0), 0);

    for i in 0..n_frames {
        let start = i * HOP_SIZE;
        let mut frame = channel_data.slice(s![start..start + FFT_SIZE]).to_owned();
        frame *= &window;

        let mut buffer: Vec<Complex<f32>> = frame.iter().map(|&x| Complex::new(x, 0.0)).collect();
        fft.process(&mut buffer);

        for j in 0..(FFT_SIZE / 2 + 1) {
            spectrogram[[j, i]] = buffer[j];
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
