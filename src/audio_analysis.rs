use bevy::prelude::*;
use crossbeam_channel as mpsc;
use rustfft::{num_complex::Complex, FftPlanner};
use std::fs::File;
use std::io::Read;
use std::thread;

#[derive(Clone, Debug)]
pub struct AudioBands {
    pub sub_bass: f32,        // 0-60 Hz
    pub bass: f32,            // 60-250 Hz
    pub low_mid: f32,         // 250-500 Hz
    pub mid: f32,             // 500-2000 Hz
    pub high_mid: f32,        // 2000-4000 Hz
    pub high: f32,            // 4000-8000 Hz
    pub air: f32,             // 8000+ Hz
    pub spectrum: [f32; 128], // Detailed 128-band spectrum for visualizer
}

impl Default for AudioBands {
    fn default() -> Self {
        Self {
            sub_bass: 0.0,
            bass: 0.0,
            low_mid: 0.0,
            mid: 0.0,
            high_mid: 0.0,
            high: 0.0,
            air: 0.0,
            spectrum: [0.0; 128],
        }
    }
}

#[derive(Resource)]
pub struct AudioStreamReceiver {
    pub receiver: mpsc::Receiver<AudioBands>,
    pub current_bands: AudioBands,
}

pub fn start_audio_stream(fifo_path: &str) -> AudioStreamReceiver {
    let (tx, rx) = mpsc::unbounded();

    let path = fifo_path.to_string();
    thread::spawn(move || {
        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return, // if fifo doesn't exist, stop the thread gracefully
        };

        // 44100Hz, 16-bit, 2 channels
        let sample_rate = 44100.0;
        let chunk_size = 2048; // roughly 21.5Hz update rate

        // 2 channels * 2 bytes (16-bit) = 4 bytes per frame
        let bytes_per_frame = 4;
        let mut byte_buffer = vec![0u8; chunk_size * bytes_per_frame];

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(chunk_size);

        let hanning_window: Vec<f32> = (0..chunk_size)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (chunk_size - 1) as f32).cos())
            })
            .collect();

        // rolling max history for normalization (roughly 3 seconds at ~20hz)
        let mut sub_bass_history = vec![0.001; 60];
        let mut bass_history = vec![0.001; 60];
        let mut low_mid_history = vec![0.001; 60];
        let mut mid_history = vec![0.001; 60];
        let mut high_mid_history = vec![0.001; 60];
        let mut high_history = vec![0.001; 60];
        let mut air_history = vec![0.001; 60];

        let mut spectrum_history = vec![vec![0.001; 60]; 128];
        let mut hist_idx = 0;

        loop {
            if file.read_exact(&mut byte_buffer).is_err() {
                // If it fails to read (e.g. MPD closed), wait and reopen
                thread::sleep(std::time::Duration::from_millis(500));
                if let Ok(f) = File::open(&path) {
                    file = f;
                }
                continue;
            }

            // Convert to f32 mono
            let mut all_samples = Vec::with_capacity(chunk_size);
            for i in 0..chunk_size {
                let offset = i * bytes_per_frame;
                let left = i16::from_le_bytes([byte_buffer[offset], byte_buffer[offset + 1]])
                    as f32
                    / i16::MAX as f32;
                let right = i16::from_le_bytes([byte_buffer[offset + 2], byte_buffer[offset + 3]])
                    as f32
                    / i16::MAX as f32;
                all_samples.push((left + right) / 2.0);
            }

            let mut buffer: Vec<Complex<f32>> = all_samples
                .iter()
                .enumerate()
                .map(|(i, &s)| Complex {
                    re: s * hanning_window[i],
                    im: 0.0,
                })
                .collect();

            fft.process(&mut buffer);

            let mut sub_bass_mag = 0.0;
            let mut bass_mag = 0.0;
            let mut low_mid_mag = 0.0;
            let mut mid_mag = 0.0;
            let mut high_mid_mag = 0.0;
            let mut high_mag = 0.0;
            let mut air_mag = 0.0;

            let mut spectrum_mags = [0.0; 128];

            let num_bins = chunk_size / 2;
            let freq_per_bin = sample_rate / chunk_size as f32;

            let min_freq = 20.0_f32;
            let max_freq = 20000.0_f32;
            let log_range = (max_freq / min_freq).log10();

            for (i, complex) in buffer.iter().take(num_bins).enumerate().skip(1) {
                // skip DC offset at i=0
                let freq = i as f32 * freq_per_bin;
                let mag = complex.norm() / chunk_size as f32;

                if freq < 60.0 {
                    sub_bass_mag = f32::max(sub_bass_mag, mag);
                } else if freq < 250.0 {
                    bass_mag = f32::max(bass_mag, mag);
                } else if freq < 500.0 {
                    low_mid_mag = f32::max(low_mid_mag, mag);
                } else if freq < 2000.0 {
                    mid_mag = f32::max(mid_mag, mag);
                } else if freq < 4000.0 {
                    high_mid_mag = f32::max(high_mid_mag, mag);
                } else if freq < 8000.0 {
                    high_mag = f32::max(high_mag, mag);
                } else {
                    air_mag = f32::max(air_mag, mag);
                }

                // Map frequency to one of the 128 bands
                if freq >= min_freq && freq <= max_freq {
                    let log_pos = (freq / min_freq).log10() / log_range;
                    let band_idx = (log_pos * 128.0) as usize;
                    let band_idx = band_idx.clamp(0, 127);
                    spectrum_mags[band_idx] = f32::max(spectrum_mags[band_idx], mag);
                }
            }

            // Update history and normalize
            sub_bass_history[hist_idx] = sub_bass_mag;
            bass_history[hist_idx] = bass_mag;
            low_mid_history[hist_idx] = low_mid_mag;
            mid_history[hist_idx] = mid_mag;
            high_mid_history[hist_idx] = high_mid_mag;
            high_history[hist_idx] = high_mag;
            air_history[hist_idx] = air_mag;

            let mut final_spectrum = [0.0; 128];
            for i in 0..128 {
                spectrum_history[i][hist_idx] = spectrum_mags[i];
                let max_mag = spectrum_history[i].iter().copied().fold(0.001, f32::max);
                final_spectrum[i] = (spectrum_mags[i] / max_mag).clamp(0.0, 1.0).powf(0.5);
            }

            hist_idx = (hist_idx + 1) % 60;

            let max_sub_bass = sub_bass_history.iter().copied().fold(0.001, f32::max);
            let max_bass = bass_history.iter().copied().fold(0.001, f32::max);
            let max_low_mid = low_mid_history.iter().copied().fold(0.001, f32::max);
            let max_mid = mid_history.iter().copied().fold(0.001, f32::max);
            let max_high_mid = high_mid_history.iter().copied().fold(0.001, f32::max);
            let max_high = high_history.iter().copied().fold(0.001, f32::max);
            let max_air = air_history.iter().copied().fold(0.001, f32::max);

            let bands = AudioBands {
                sub_bass: (sub_bass_mag / max_sub_bass).clamp(0.0, 1.0).powf(0.5),
                bass: (bass_mag / max_bass).clamp(0.0, 1.0).powf(0.5),
                low_mid: (low_mid_mag / max_low_mid).clamp(0.0, 1.0).powf(0.5),
                mid: (mid_mag / max_mid).clamp(0.0, 1.0).powf(0.5),
                high_mid: (high_mid_mag / max_high_mid).clamp(0.0, 1.0).powf(0.5),
                high: (high_mag / max_high).clamp(0.0, 1.0).powf(0.5),
                air: (air_mag / max_air).clamp(0.0, 1.0).powf(0.5),
                spectrum: final_spectrum,
            };

            if tx.send(bands).is_err() {
                break; // Receiver dropped, stop thread
            }
        }
    });

    AudioStreamReceiver {
        receiver: rx,
        current_bands: AudioBands::default(),
    }
}
