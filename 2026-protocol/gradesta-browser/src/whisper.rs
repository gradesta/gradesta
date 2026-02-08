//! Whisper transcription support
//!
//! Uses whisper-rs to transcribe audio to text locally.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Global Whisper context (loaded once, reused)
static WHISPER_CTX: OnceLock<Option<WhisperContext>> = OnceLock::new();

/// Download progress tracking
pub struct DownloadProgress {
    pub downloaded: AtomicU64,
    pub total: AtomicU64,
    pub complete: AtomicBool,
    pub error: std::sync::Mutex<Option<String>>,
}

impl Default for DownloadProgress {
    fn default() -> Self {
        Self {
            downloaded: AtomicU64::new(0),
            total: AtomicU64::new(0),
            complete: AtomicBool::new(false),
            error: std::sync::Mutex::new(None),
        }
    }
}

/// Get the path to the Whisper model file
/// Uses tiny.en model (~75MB) for faster CPU inference
pub fn get_model_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gradesta")
        .join("whisper");
    config_dir.join("ggml-tiny.en.bin")
}

/// Check if the Whisper model is available
pub fn is_model_available() -> bool {
    get_model_path().exists()
}

/// Get the model download URL
/// tiny.en is ~75MB vs base.en at ~142MB, and runs ~3x faster on CPU
pub fn get_model_url() -> &'static str {
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
}

/// Download the Whisper model with progress tracking (for GUI)
pub fn download_model_with_progress(progress: Arc<DownloadProgress>) -> Result<()> {
    let model_path = get_model_path();
    if model_path.exists() {
        progress.complete.store(true, Ordering::SeqCst);
        return Ok(());
    }

    // Create directory
    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Download using reqwest with longer timeout for large file
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600)) // 10 minute timeout
        .build()
        .context("Failed to create HTTP client")?;

    let response = client.get(get_model_url())
        .send()
        .context("Failed to download Whisper model")?;

    if !response.status().is_success() {
        let err = format!("Download failed: HTTP {}", response.status());
        *progress.error.lock().unwrap() = Some(err.clone());
        return Err(anyhow!(err));
    }

    // Get content length for progress
    let total_size = response.content_length().unwrap_or(75_000_000); // ~75MB for tiny.en
    progress.total.store(total_size, Ordering::SeqCst);

    // Download with progress tracking
    use std::io::Read;
    let mut downloaded = 0u64;
    let mut buffer = Vec::with_capacity(total_size as usize);
    let mut reader = response;
    let mut chunk = [0u8; 65536]; // 64KB chunks

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&chunk[..n]);
                downloaded += n as u64;
                progress.downloaded.store(downloaded, Ordering::SeqCst);
            }
            Err(e) => {
                let err = format!("Download error: {}", e);
                *progress.error.lock().unwrap() = Some(err.clone());
                return Err(anyhow!(err));
            }
        }
    }

    std::fs::write(&model_path, &buffer)?;
    progress.complete.store(true, Ordering::SeqCst);

    Ok(())
}

/// Download the Whisper model if not present (simple version for CLI)
pub fn download_model() -> Result<()> {
    let progress = Arc::new(DownloadProgress::default());
    download_model_with_progress(progress)
}

/// Initialize the Whisper context (call once at startup or on first use)
fn get_or_init_context() -> Option<&'static WhisperContext> {
    WHISPER_CTX.get_or_init(|| {
        let model_path = get_model_path();
        if !model_path.exists() {
            eprintln!("Whisper model not found at {:?}", model_path);
            return None;
        }

        eprintln!("Loading Whisper model from {:?}...", model_path);
        match WhisperContext::new_with_params(
            model_path.to_str().unwrap_or(""),
            WhisperContextParameters::default(),
        ) {
            Ok(ctx) => {
                eprintln!("Whisper model loaded successfully");
                Some(ctx)
            }
            Err(e) => {
                eprintln!("Failed to load Whisper model: {:?}", e);
                None
            }
        }
    }).as_ref()
}

/// Preload the Whisper model in a background thread
/// Call this at app startup to avoid delay on first transcription
pub fn preload_model() {
    if !is_model_available() {
        return;
    }

    std::thread::spawn(|| {
        eprintln!("Preloading Whisper model in background...");
        let _ = get_or_init_context();
    });
}

/// Transcribe audio samples to text
///
/// # Arguments
/// * `samples` - Audio samples as f32 values (-1.0 to 1.0)
/// * `sample_rate` - Sample rate of the audio (will be resampled to 16kHz if needed)
///
/// # Returns
/// The transcribed text, or an error
pub fn transcribe(samples: &[f32], sample_rate: u32) -> Result<String> {
    let ctx = get_or_init_context()
        .ok_or_else(|| anyhow!("Whisper model not loaded"))?;

    // Whisper expects 16kHz mono audio
    let mut samples_16k = if sample_rate != 16000 {
        resample(samples, sample_rate, 16000)
    } else {
        samples.to_vec()
    };

    // Whisper requires at least 1 second of audio (16000 samples at 16kHz)
    // Pad with silence if too short
    let min_samples = 16000;
    if samples_16k.len() < min_samples {
        samples_16k.resize(min_samples, 0.0);
    }

    // Create a new state for this transcription
    let mut state = ctx.create_state()
        .map_err(|e| anyhow!("Failed to create Whisper state: {:?}", e))?;

    // Configure parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(4);
    params.set_translate(false);
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Run transcription
    state.full(params, &samples_16k)
        .map_err(|e| anyhow!("Transcription failed: {:?}", e))?;

    // Collect results
    let num_segments = state.full_n_segments()
        .map_err(|e| anyhow!("Failed to get segments: {:?}", e))?;

    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
            text.push_str(&segment);
            text.push(' ');
        }
    }

    Ok(text.trim().to_string())
}

/// Simple linear resampling
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio) as usize;
    let mut result = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = i as f64 * ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(samples.len() - 1);
        let frac = src_idx - idx0 as f64;

        let sample = samples[idx0] as f64 * (1.0 - frac) + samples[idx1] as f64 * frac;
        result.push(sample as f32);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample() {
        // Simple test: 4 samples at 2x rate should become 2 samples
        let samples = vec![0.0, 0.5, 1.0, 0.5];
        let resampled = resample(&samples, 4, 2);
        assert_eq!(resampled.len(), 2);
    }
}
