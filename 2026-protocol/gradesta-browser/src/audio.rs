//! Audio recording, playback, and processing functionality.
//!
//! This module handles microphone recording, audio playback via rodio,
//! and encoding to OGG Vorbis format.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Signal to stop audio recording and communicate sample rate
#[derive(Resource, Default)]
pub struct AudioRecordingSignal {
    pub should_stop: Arc<Mutex<bool>>,
    pub actual_sample_rate: Arc<Mutex<u32>>,
}

/// Signal to control audio playback
#[derive(Resource, Clone, Default)]
pub struct AudioPlaybackState {
    pub should_stop: Arc<Mutex<bool>>,
    pub playing_vertex: Arc<Mutex<Option<u64>>>,
}

/// Play audio data in a background thread
pub fn play_audio(data: &[u8], vertex_id: u64, state: &AudioPlaybackState) {
    // First stop any currently playing audio
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = true;
    }
    // Small delay to let the previous playback stop
    thread::sleep(Duration::from_millis(50));

    // Reset the stop signal for the new playback
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = false;
    }

    // Mark this vertex as playing
    if let Ok(mut playing) = state.playing_vertex.lock() {
        *playing = Some(vertex_id);
    }

    let data_vec = data.to_vec();
    let stop_signal = state.should_stop.clone();
    let playing_vertex = state.playing_vertex.clone();

    thread::spawn(move || {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;

        match OutputStream::try_default() {
            Ok((_stream, handle)) => {
                match Sink::try_new(&handle) {
                    Ok(sink) => {
                        let cursor = Cursor::new(data_vec);
                        match Decoder::new(cursor) {
                            Ok(source) => {
                                sink.append(source);
                                // Poll for stop signal - when signaled, drop sink to stop audio
                                while !sink.empty() {
                                    if let Ok(stop) = stop_signal.lock() {
                                        if *stop {
                                            // Drop sink and stream to stop audio immediately
                                            drop(sink);
                                            return;
                                        }
                                    }
                                    thread::sleep(Duration::from_millis(50));
                                }
                            }
                            Err(e) => eprintln!("Failed to decode audio: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to create audio sink: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to get audio output: {}", e),
        }

        // Clear playing state when done
        if let Ok(mut playing) = playing_vertex.lock() {
            *playing = None;
        }
    });
}

/// Stop any currently playing audio
pub fn stop_audio(state: &AudioPlaybackState) {
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = true;
    }
}

/// Run audio recording from the default input device
pub fn run_audio_recording(
    samples: Arc<Mutex<Vec<f32>>>,
    stop_signal: Arc<Mutex<bool>>,
    sample_rate_out: Arc<Mutex<u32>>,
) -> Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or_else(|| anyhow!("No audio input device"))?;

    let config = device.default_input_config()
        .context("Failed to get default input config")?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    eprintln!("Audio: device={:?} rate={} channels={}", device.name(), sample_rate, channels);

    // Communicate actual sample rate back
    if let Ok(mut sr) = sample_rate_out.lock() {
        *sr = sample_rate;
    }

    let samples_clone = samples.clone();
    let err_fn = |err| eprintln!("Audio stream error: {}", err);

    // Helper to convert to mono by averaging channels
    let to_mono = move |data: &[f32], s: &mut Vec<f32>| {
        if channels == 1 {
            s.extend_from_slice(data);
        } else {
            // Average channels to mono
            for chunk in data.chunks(channels) {
                let sum: f32 = chunk.iter().sum();
                s.push(sum / channels as f32);
            }
        }
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        to_mono(data, &mut s);
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let samples_clone = samples.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        let float_data: Vec<f32> = data.iter().map(|&x| x as f32 / 32768.0).collect();
                        if channels == 1 {
                            s.extend(float_data);
                        } else {
                            for chunk in float_data.chunks(channels) {
                                let sum: f32 = chunk.iter().sum();
                                s.push(sum / channels as f32);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let samples_clone = samples.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        let float_data: Vec<f32> = data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0).collect();
                        if channels == 1 {
                            s.extend(float_data);
                        } else {
                            for chunk in float_data.chunks(channels) {
                                let sum: f32 = chunk.iter().sum();
                                s.push(sum / channels as f32);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => return Err(anyhow!("Unsupported sample format")),
    };

    stream.play()?;

    // Wait until stop signal
    loop {
        thread::sleep(Duration::from_millis(50));
        if let Ok(stop) = stop_signal.lock() {
            if *stop {
                break;
            }
        }
    }

    // Stream is dropped when function returns, stopping recording
    Ok(())
}

/// Encode audio samples to OGG Vorbis format
/// Returns the OGG data with embedded transcript in Vorbis comments
pub fn encode_ogg_vorbis(samples: &[f32], sample_rate: u32, transcript: Option<&str>) -> Result<Vec<u8>> {
    use vorbis_rs::VorbisEncoderBuilder;

    // Create output buffer
    let mut output: Vec<u8> = Vec::new();

    // Create encoder that writes to output
    let mut builder = VorbisEncoderBuilder::new(
        std::num::NonZeroU32::new(sample_rate).ok_or_else(|| anyhow!("Invalid sample rate"))?,
        std::num::NonZeroU8::new(1).unwrap(), // mono
        &mut output,
    )
    .context("Failed to create Vorbis encoder builder")?;

    // Add comments
    builder.comment_tag("ENCODER", "Gradesta Browser")
        .context("Failed to add encoder comment")?;
    if let Some(text) = transcript {
        builder.comment_tag("TRANSCRIPT", text)
            .context("Failed to add transcript comment")?;
    }

    let mut encoder = builder.build()
        .context("Failed to build Vorbis encoder")?;

    // Encode audio in chunks
    let chunk_size = 4096;
    for chunk in samples.chunks(chunk_size) {
        encoder.encode_audio_block([chunk])
            .context("Failed to encode audio block")?;
    }

    // Finish encoding - this flushes all data
    encoder.finish().context("Failed to finish encoding")?;

    eprintln!("Encoded OGG: {} samples -> {} bytes", samples.len(), output.len());

    if output.len() < 100 {
        return Err(anyhow!("OGG encoding produced suspiciously small output: {} bytes", output.len()));
    }

    Ok(output)
}

/// Parse WAV header to get duration
pub fn parse_wav_duration(data: &[u8]) -> Option<Duration> {
    if data.len() < 44 {
        return None;
    }

    // Check RIFF header
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    // Parse fmt chunk - look for it after WAVE header
    let mut pos = 12;
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let sample_rate = u32::from_le_bytes([data[pos + 12], data[pos + 13], data[pos + 14], data[pos + 15]]);
            let byte_rate = u32::from_le_bytes([data[pos + 16], data[pos + 17], data[pos + 18], data[pos + 19]]);

            // Find data chunk
            pos += 8 + chunk_size;
            while pos + 8 < data.len() {
                let chunk_id = &data[pos..pos + 4];
                let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

                if chunk_id == b"data" {
                    let duration_secs = chunk_size as f64 / byte_rate as f64;
                    return Some(Duration::from_secs_f64(duration_secs));
                }
                pos += 8 + chunk_size;
            }
            break;
        }
        pos += 8 + chunk_size;
    }

    None
}

/// Extract transcript from audio file (OGG Vorbis comments or embedded metadata)
pub fn extract_transcript(data: &[u8], mime: &str) -> Option<String> {
    if mime == "audio/ogg" || mime.contains("ogg") {
        extract_ogg_transcript(data)
    } else if mime == "audio/wav" || mime.contains("wav") {
        extract_wav_transcript(data)
    } else {
        None
    }
}

/// Extract transcript from OGG Vorbis comments
fn extract_ogg_transcript(data: &[u8]) -> Option<String> {
    // Simple Vorbis comment extraction
    // Look for "TRANSCRIPT=" in the comment header
    let needle = b"TRANSCRIPT=";
    for window in data.windows(needle.len()) {
        if window == needle {
            let start = data.iter().position(|&b| b == needle[0])? + needle.len();
            // Find the end of the comment (null byte or next tag)
            let remaining = &data[start..];
            let end = remaining.iter()
                .position(|&b| b == 0 || b == b'=' && remaining.get((b as usize).saturating_sub(10)..(b as usize)).map(|s| s.iter().all(|&c| c.is_ascii_uppercase())).unwrap_or(false))
                .unwrap_or(remaining.len().min(1000));
            let transcript = String::from_utf8_lossy(&remaining[..end]);
            if !transcript.is_empty() {
                return Some(transcript.to_string());
            }
        }
    }
    None
}

/// Extract transcript from WAV file (LIST/INFO chunk)
fn extract_wav_transcript(data: &[u8]) -> Option<String> {
    // Look for INFO chunk with ICMT (comment) or INAM (name) tag
    let mut pos = 12; // Skip RIFF header
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        if chunk_id == b"LIST" && pos + 12 < data.len() && &data[pos + 8..pos + 12] == b"INFO" {
            // Parse INFO subchunks
            let mut info_pos = pos + 12;
            let info_end = pos + 8 + chunk_size;
            while info_pos + 8 < info_end && info_pos + 8 < data.len() {
                let sub_id = &data[info_pos..info_pos + 4];
                let sub_size = u32::from_le_bytes([data[info_pos + 4], data[info_pos + 5], data[info_pos + 6], data[info_pos + 7]]) as usize;

                if (sub_id == b"ICMT" || sub_id == b"INAM") && info_pos + 8 + sub_size <= data.len() {
                    let text_data = &data[info_pos + 8..info_pos + 8 + sub_size];
                    // Trim null terminator if present
                    let text = text_data.split(|&b| b == 0).next().unwrap_or(text_data);
                    if let Ok(s) = std::str::from_utf8(text) {
                        if !s.is_empty() {
                            return Some(s.to_string());
                        }
                    }
                }
                info_pos += 8 + ((sub_size + 1) & !1); // Align to word boundary
            }
        }
        pos += 8 + ((chunk_size + 1) & !1); // Align to word boundary
    }
    None
}
