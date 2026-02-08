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

/// Play audio data using rodio with stop signal support
pub fn play_audio(data: &[u8], _mime: &str, vertex_id: u64, state: &AudioPlaybackState) {
    use crate::media::calculate_audio_rms;
    use crate::tts;

    // Stop any currently playing audio
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = true;
    }
    // Small delay to let the previous thread notice the stop signal
    thread::sleep(Duration::from_millis(50));

    // Reset signal for new playback
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = false;
    }
    if let Ok(mut playing) = state.playing_vertex.lock() {
        *playing = Some(vertex_id);
    }

    // Calculate RMS of audio for TTS volume calibration
    if let Some(rms) = calculate_audio_rms(data) {
        tts::set_reference_audio_level(rms);
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

/// Parse WAV header to get duration in seconds
pub fn parse_wav_duration(data: &[u8]) -> Option<f32> {
    if data.len() < 44 {
        return None;
    }
    // Check RIFF header
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    // Find fmt chunk
    let mut pos = 12;
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos+4];
        let chunk_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let channels = u16::from_le_bytes(data[pos+10..pos+12].try_into().ok()?) as u32;
            let sample_rate = u32::from_le_bytes(data[pos+12..pos+16].try_into().ok()?);
            let bits_per_sample = u16::from_le_bytes(data[pos+22..pos+24].try_into().ok()?) as u32;

            // Find data chunk
            pos += 8 + chunk_size;
            while pos + 8 < data.len() {
                let data_chunk_id = &data[pos..pos+4];
                let data_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?);

                if data_chunk_id == b"data" {
                    let bytes_per_sample = (bits_per_sample / 8) * channels;
                    if bytes_per_sample > 0 && sample_rate > 0 {
                        let num_samples = data_size / bytes_per_sample;
                        return Some(num_samples as f32 / sample_rate as f32);
                    }
                }
                pos += 8 + data_size as usize;
                if data_size % 2 == 1 {
                    pos += 1; // Pad to even
                }
            }
        }
        pos += 8 + chunk_size;
        if chunk_size % 2 == 1 {
            pos += 1; // Pad to even
        }
    }
    None
}

/// Extract transcript from audio data (OGG Vorbis comments or WAV 'trns' chunk)
pub fn extract_transcript(data: &[u8], mime: &str) -> Option<String> {
    if mime == "audio/ogg" || (data.len() >= 4 && &data[0..4] == b"OggS") {
        // Try to extract from OGG Vorbis comments
        extract_ogg_transcript(data)
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        // Try to extract from WAV 'trns' chunk
        extract_wav_transcript(data)
    } else {
        None
    }
}

/// Extract transcript from OGG Vorbis comments
fn extract_ogg_transcript(data: &[u8]) -> Option<String> {
    use lewton::inside_ogg::OggStreamReader;
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let reader = OggStreamReader::new(cursor).ok()?;

    // Look for TRANSCRIPT comment in Vorbis comments
    for (key, value) in reader.comment_hdr.comment_list.iter() {
        if key.eq_ignore_ascii_case("TRANSCRIPT") {
            return Some(value.clone());
        }
    }
    None
}

/// Extract transcript from WAV 'trns' chunk
fn extract_wav_transcript(data: &[u8]) -> Option<String> {
    if data.len() < 44 {
        return None;
    }
    // Find trns chunk
    let mut pos = 12;
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos+4];
        let chunk_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?) as usize;

        if chunk_id == b"trns" {
            let text_end = pos + 8 + chunk_size;
            if text_end <= data.len() {
                return String::from_utf8(data[pos+8..text_end].to_vec()).ok();
            }
        }
        pos += 8 + chunk_size;
        if chunk_size % 2 == 1 {
            pos += 1; // Pad to even
        }
    }
    None
}
