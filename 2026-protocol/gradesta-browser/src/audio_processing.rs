//! Advanced audio processing algorithms.
//!
//! This module contains:
//! - RMS audio normalization with soft limiting
//! - Pitch-preserving time stretching (OLA algorithm)

use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Audio Normalization
// ============================================================================

const TARGET_RMS_DB: f32 = -20.0;
const SILENCE_THRESHOLD: f32 = 1e-6;
const MAX_GAIN_DB: f32 = 40.0;
const MIN_GAIN_DB: f32 = -20.0;
const LIMITER_THRESHOLD: f32 = 0.9;

pub fn normalize_audio(samples: &mut [f32]) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }

    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();

    if rms < SILENCE_THRESHOLD {
        return 1.0;
    }

    let target_rms = 10.0_f32.powf(TARGET_RMS_DB / 20.0);
    let mut gain = target_rms / rms;

    let max_gain = 10.0_f32.powf(MAX_GAIN_DB / 20.0);
    let min_gain = 10.0_f32.powf(MIN_GAIN_DB / 20.0);
    gain = gain.clamp(min_gain, max_gain);

    for sample in samples.iter_mut() {
        *sample *= gain;
    }

    for sample in samples.iter_mut() {
        let abs_val = sample.abs();
        if abs_val > LIMITER_THRESHOLD {
            let excess = abs_val - LIMITER_THRESHOLD;
            let compressed =
                LIMITER_THRESHOLD + (1.0 - LIMITER_THRESHOLD) * (excess / (1.0 + excess)).tanh();
            *sample = sample.signum() * compressed.min(0.99);
        }
    }

    gain
}

// ============================================================================
// Pitch-Preserving Time Stretching
// ============================================================================

static AUDIO_PLAYBACK_SPEED: AtomicU32 = AtomicU32::new(0x3F800000);

pub fn set_audio_speed(speed: f32) {
    AUDIO_PLAYBACK_SPEED.store(speed.to_bits(), Ordering::SeqCst);
}

pub fn get_audio_speed() -> f32 {
    f32::from_bits(AUDIO_PLAYBACK_SPEED.load(Ordering::SeqCst))
}

/// Simple OLA time stretcher with proper overlap handling.
///
/// This uses a straightforward overlap-add approach with Hann windowing.
/// The key insight is that with 50% overlap and Hann window, the windows
/// sum to exactly 1.0 at every point, giving perfect reconstruction at 1x speed.
pub struct TimeStretcher {
    channels: usize,
    frame_size: usize,
    /// Analysis hop (input spacing) - varies with speed
    analysis_hop: usize,
    /// Synthesis hop (output spacing) - fixed at frame_size/2 for 50% overlap
    synthesis_hop: usize,
    /// Hann window
    window: Vec<f32>,
    /// Input buffer (mono float)
    input: Vec<f32>,
    /// Output buffer (mono float)
    output: Vec<f32>,
    /// Current input read position (fractional for smooth speed)
    input_pos: f64,
    /// Output write position
    output_write_pos: usize,
    /// Output read position
    output_read_pos: usize,
    speed: f32,
}

impl TimeStretcher {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        // Frame size ~46ms for better frequency resolution
        let frame_size = ((sample_rate as usize) * 46) / 1000;
        let frame_size = frame_size.next_power_of_two(); // 2048 at 44.1kHz

        // 50% overlap - with Hann window, overlapping windows sum to exactly 1.0
        let synthesis_hop = frame_size / 2;

        // Hann window
        let window: Vec<f32> = (0..frame_size)
            .map(|i| {
                let t = i as f32 / frame_size as f32;
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
            })
            .collect();

        Self {
            channels: channels as usize,
            frame_size,
            analysis_hop: synthesis_hop, // Will be updated by set_speed
            synthesis_hop,
            window,
            input: Vec::new(),
            output: Vec::new(),
            input_pos: 0.0,
            output_write_pos: 0,
            output_read_pos: 0,
            speed: 1.0,
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.5, 4.0);
        // Analysis hop = synthesis_hop * speed
        // At 2x: we read input twice as fast, producing half the output duration
        self.analysis_hop = ((self.synthesis_hop as f32) * self.speed) as usize;
    }

    pub fn write(&mut self, samples: &[i16]) {
        // Convert to mono float
        if self.channels == 1 {
            for &s in samples {
                self.input.push(s as f32 / 32768.0);
            }
        } else {
            for chunk in samples.chunks(self.channels) {
                let sum: f32 = chunk.iter().map(|&s| s as f32).sum();
                self.input.push(sum / (self.channels as f32 * 32768.0));
            }
        }
        self.process();
    }

    fn process(&mut self) {
        // Recompute analysis_hop in case speed changed
        let analysis_hop = (self.synthesis_hop as f64) * (self.speed as f64);

        // Process while we have enough input for a complete frame
        while (self.input_pos as usize) + self.frame_size <= self.input.len() {
            let input_start = self.input_pos as usize;

            // Ensure output buffer is large enough
            let needed = self.output_write_pos + self.frame_size;
            if self.output.len() < needed {
                self.output.resize(needed, 0.0);
            }

            // Apply window and overlap-add
            for i in 0..self.frame_size {
                let sample = self.input[input_start + i];
                self.output[self.output_write_pos + i] += sample * self.window[i];
            }

            // Advance positions
            self.input_pos += analysis_hop;
            self.output_write_pos += self.synthesis_hop;
        }

        // Trim old input data
        let safe_pos = (self.input_pos as usize).saturating_sub(self.frame_size);
        if safe_pos > self.frame_size {
            let trim = safe_pos - self.frame_size / 2;
            self.input.drain(..trim);
            self.input_pos -= trim as f64;
        }
    }

    pub fn read(&mut self, output: &mut [i16]) -> usize {
        // With 50% overlap, samples are ready after 2 overlapping frames
        // So we need synthesis_hop margin before samples are complete
        let margin = self.synthesis_hop;
        let ready = self.output_write_pos.saturating_sub(margin + self.output_read_pos);

        if ready == 0 {
            return 0;
        }

        let mono_slots = if self.channels > 1 {
            output.len() / self.channels
        } else {
            output.len()
        };
        let to_read = ready.min(mono_slots);

        for i in 0..to_read {
            // With Hann window and 50% overlap, sum is exactly 1.0 - no normalization needed
            let sample = self.output[self.output_read_pos + i];
            let clamped = sample.clamp(-1.0, 1.0);
            let sample_i16 = (clamped * 32767.0) as i16;

            if self.channels == 1 {
                output[i] = sample_i16;
            } else {
                for c in 0..self.channels {
                    output[i * self.channels + c] = sample_i16;
                }
            }
        }

        self.output_read_pos += to_read;

        // Trim consumed output
        if self.output_read_pos > self.frame_size * 2 {
            let trim = self.output_read_pos - self.frame_size;
            self.output.drain(..trim);
            self.output_read_pos -= trim;
            self.output_write_pos -= trim;
        }

        if self.channels == 1 {
            to_read
        } else {
            to_read * self.channels
        }
    }

    pub fn flush(&mut self) {
        // Pad with silence to push remaining samples through
        for _ in 0..self.frame_size {
            self.input.push(0.0);
        }
        self.process();
    }

    pub fn has_output(&self) -> bool {
        let margin = self.synthesis_hop;
        self.output_write_pos > margin + self.output_read_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_silent() {
        let mut samples = vec![0.0; 100];
        let gain = normalize_audio(&mut samples);
        assert_eq!(gain, 1.0);
    }

    #[test]
    fn test_time_stretcher_basic() {
        let mut stretcher = TimeStretcher::new(44100, 1);
        stretcher.set_speed(1.0);

        let input: Vec<i16> = (0..8192)
            .map(|i| ((i as f32 * 0.1).sin() * 16000.0) as i16)
            .collect();

        stretcher.write(&input);
        stretcher.flush();

        let mut output = vec![0i16; 16384];
        let mut total = 0;
        loop {
            let n = stretcher.read(&mut output[total..]);
            if n == 0 { break; }
            total += n;
        }

        assert!(total > 0);
        let diff = (total as i32 - input.len() as i32).abs();
        assert!(diff < 4096, "1x: {} vs {}", total, input.len());
    }

    #[test]
    fn test_time_stretcher_speedup() {
        let mut stretcher = TimeStretcher::new(44100, 1);
        stretcher.set_speed(2.0);

        let input: Vec<i16> = (0..16384)
            .map(|i| ((i as f32 * 0.1).sin() * 16000.0) as i16)
            .collect();

        stretcher.write(&input);
        stretcher.flush();

        let mut output = vec![0i16; 16384];
        let mut total = 0;
        loop {
            let n = stretcher.read(&mut output[total..]);
            if n == 0 { break; }
            total += n;
        }

        let expected = input.len() / 2;
        let diff = (total as i32 - expected as i32).abs();
        assert!(diff < 4096, "2x: {} vs {}", total, expected);
    }

    #[test]
    fn test_time_stretcher_slowdown() {
        let mut stretcher = TimeStretcher::new(44100, 1);
        stretcher.set_speed(0.5);

        let input: Vec<i16> = (0..8192)
            .map(|i| ((i as f32 * 0.1).sin() * 16000.0) as i16)
            .collect();

        stretcher.write(&input);
        stretcher.flush();

        let mut output = vec![0i16; 32768];
        let mut total = 0;
        loop {
            let n = stretcher.read(&mut output[total..]);
            if n == 0 { break; }
            total += n;
        }

        let expected = input.len() * 2;
        let diff = (total as i32 - expected as i32).abs();
        assert!(diff < 8192, "0.5x: {} vs {}", total, expected);
    }

    #[test]
    fn test_speed_atomic() {
        set_audio_speed(2.0);
        assert!((get_audio_speed() - 2.0).abs() < 0.001);
        set_audio_speed(1.0);
        assert!((get_audio_speed() - 1.0).abs() < 0.001);
    }
}
