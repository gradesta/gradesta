//! Text-to-speech support
//!
//! Uses the `tts` crate which provides cross-platform TTS via system backends:
//! - Linux: Speech Dispatcher
//! - Windows: SAPI / WinRT
//! - macOS: AVFoundation / AppKit

use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU32, Ordering};
use tts::Tts;

/// Global TTS instance (thread-safe)
static TTS_INSTANCE: OnceLock<Arc<Mutex<Option<Tts>>>> = OnceLock::new();

/// Last measured audio RMS level (stored as f32 bits for atomic access)
/// This is updated when audio files are played, and used to set TTS volume
static LAST_AUDIO_RMS: AtomicU32 = AtomicU32::new(0);

/// TTS playback rate multiplier (stored as f32 bits, default 1.0)
static TTS_RATE: AtomicU32 = AtomicU32::new(0x3F800000); // 1.0f32.to_bits()

/// Set the reference audio level (RMS) from played audio
/// This should be called when playing audio files to calibrate TTS volume
pub fn set_reference_audio_level(rms: f32) {
    LAST_AUDIO_RMS.store(rms.to_bits(), Ordering::SeqCst);
}

/// Get the reference audio level
pub fn get_reference_audio_level() -> f32 {
    f32::from_bits(LAST_AUDIO_RMS.load(Ordering::SeqCst))
}

/// Set the TTS playback rate (1.0 = normal, 2.0 = 2x speed, etc.)
pub fn set_rate(rate: f32) {
    TTS_RATE.store(rate.to_bits(), Ordering::SeqCst);
}

/// Get the current TTS playback rate
pub fn get_rate() -> f32 {
    f32::from_bits(TTS_RATE.load(Ordering::SeqCst))
}

/// Get or initialize the TTS instance
fn get_tts() -> Arc<Mutex<Option<Tts>>> {
    TTS_INSTANCE.get_or_init(|| {
        match Tts::default() {
            Ok(tts) => {
                eprintln!("TTS initialized successfully");
                Arc::new(Mutex::new(Some(tts)))
            }
            Err(e) => {
                eprintln!("Failed to initialize TTS: {:?}", e);
                Arc::new(Mutex::new(None))
            }
        }
    }).clone()
}

/// Check if TTS is available on this system
pub fn is_available() -> bool {
    let tts_arc = get_tts();
    let guard = match tts_arc.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    guard.is_some()
}

/// Speak the given text
///
/// This is non-blocking - speech happens asynchronously.
/// Adjusts volume based on reference audio level from recorded audio.
/// Applies the current playback rate.
pub fn speak(text: &str) {
    let tts_arc = get_tts();
    let mut guard = match tts_arc.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(ref mut tts_instance) = *guard {
        // Adjust volume based on reference audio level
        // TTS volume is typically 0.0-1.0, we scale based on RMS
        let rms = get_reference_audio_level();
        if rms > 0.0 {
            // Map RMS to volume: typical speech RMS is 0.1-0.3
            // Scale so that RMS of 0.2 maps to full volume
            let volume = (rms / 0.15).min(1.0).max(0.5);
            let _ = tts_instance.set_volume(volume);
        } else {
            // Default to max volume if no reference
            let _ = tts_instance.set_volume(1.0);
        }

        // Apply playback rate
        // TTS rate is typically in range min_rate..max_rate
        // We map our rate multiplier to that range
        let rate = get_rate();
        let min_rate = tts_instance.min_rate();
        let max_rate = tts_instance.max_rate();
        let normal_rate = tts_instance.normal_rate();

        // Map rate multiplier (0.5-3.0) to TTS rate range
        // rate=1.0 -> normal_rate, rate=2.0 -> towards max, rate=0.5 -> towards min
        let tts_rate = if rate >= 1.0 {
            // Interpolate from normal to max
            let t = ((rate - 1.0) / 2.0).min(1.0); // rate 1.0-3.0 maps to t 0.0-1.0
            normal_rate + t * (max_rate - normal_rate)
        } else {
            // Interpolate from min to normal
            let t = ((rate - 0.5) / 0.5).max(0.0); // rate 0.5-1.0 maps to t 0.0-1.0
            min_rate + t * (normal_rate - min_rate)
        };
        let _ = tts_instance.set_rate(tts_rate);

        // Interrupt any current speech and speak the new text
        if let Err(e) = tts_instance.speak(text, true) {
            eprintln!("TTS speak failed: {:?}", e);
        }
    }
}

/// Stop any current speech
pub fn stop() {
    let tts_arc = get_tts();
    let mut guard = match tts_arc.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(ref mut tts_instance) = *guard {
        if let Err(e) = tts_instance.stop() {
            eprintln!("TTS stop failed: {:?}", e);
        }
    }
}

/// Check if the TTS engine is currently speaking
pub fn is_speaking() -> bool {
    let tts_arc = get_tts();
    let guard = match tts_arc.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if let Some(ref tts_instance) = *guard {
        return tts_instance.is_speaking().unwrap_or(false);
    }
    false
}

/// Initialize TTS in background (optional, for faster first speak)
pub fn preload() {
    std::thread::spawn(|| {
        eprintln!("Preloading TTS in background...");
        let _ = get_tts();
    });
}
