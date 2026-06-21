use std::fs::File;
use std::io::Read;
use crossbeam_channel as mpsc;
use std::thread;
use bevy::prelude::*;
use rustfft::{num_complex::Complex, FftPlanner};

#[derive(Clone, Default, Debug)]
pub struct AudioBands {
    pub low: f32,
    pub mid: f32,
    pub high: f32,
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
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (chunk_size - 1) as f32).cos()))
            .collect();

        // rolling max history for normalization (roughly 3 seconds at ~20hz)
        let mut low_history = vec![0.001; 60];
        let mut mid_history = vec![0.001; 60];
        let mut high_history = vec![0.001; 60];
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
                let left = i16::from_le_bytes([byte_buffer[offset], byte_buffer[offset + 1]]) as f32 / i16::MAX as f32;
                let right = i16::from_le_bytes([byte_buffer[offset + 2], byte_buffer[offset + 3]]) as f32 / i16::MAX as f32;
                all_samples.push((left + right) / 2.0);
            }

            let mut buffer: Vec<Complex<f32>> = all_samples.iter().enumerate().map(|(i, &s)| Complex { 
                re: s * hanning_window[i], 
                im: 0.0 
            }).collect();
            
            fft.process(&mut buffer);

            let mut low_mag = 0.0;
            let mut mid_mag = 0.0;
            let mut high_mag = 0.0;

            let num_bins = chunk_size / 2;
            let freq_per_bin = sample_rate / chunk_size as f32;

            for i in 1..num_bins { // skip DC offset at i=0
                let freq = i as f32 * freq_per_bin;
                let mag = buffer[i].norm() / chunk_size as f32;

                if freq < 250.0 {
                    low_mag = f32::max(low_mag, mag);
                } else if freq < 2000.0 {
                    mid_mag = f32::max(mid_mag, mag);
                } else if freq < 10000.0 {
                    high_mag = f32::max(high_mag, mag);
                }
            }

            // Update history and normalize
            low_history[hist_idx] = low_mag;
            mid_history[hist_idx] = mid_mag;
            high_history[hist_idx] = high_mag;
            hist_idx = (hist_idx + 1) % 60;

            let max_low = low_history.iter().copied().fold(0.001, f32::max);
            let max_mid = mid_history.iter().copied().fold(0.001, f32::max);
            let max_high = high_history.iter().copied().fold(0.001, f32::max);

            let bands = AudioBands {
                low: (low_mag / max_low).clamp(0.0, 1.0).powf(0.5),
                mid: (mid_mag / max_mid).clamp(0.0, 1.0).powf(0.5),
                high: (high_mag / max_high).clamp(0.0, 1.0).powf(0.5),
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
