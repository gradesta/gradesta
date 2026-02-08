//! Text-to-speech support
//!
//! Uses the `tts` crate which provides cross-platform TTS via system backends:
//! - Linux: Speech Dispatcher
//! - Windows: SAPI / WinRT
//! - macOS: AVFoundation / AppKit

use std::sync::{Arc, Mutex, OnceLock};
use tts::Tts;

/// Global TTS instance (thread-safe)
static TTS_INSTANCE: OnceLock<Arc<Mutex<Option<Tts>>>> = OnceLock::new();

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
/// This is non-blocking - speech happens asynchronously
pub fn speak(text: &str) {
    let tts_arc = get_tts();
    let mut guard = match tts_arc.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if let Some(ref mut tts_instance) = *guard {
        if let Err(e) = tts_instance.speak(text, false) {
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
