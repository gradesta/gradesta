//! Audio recording, playback, and processing functionality.
//!
//! This module handles microphone recording, audio playback via rodio,
//! and encoding to OGG Vorbis format.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use rodio::{OutputStream, Sink, Source};

use crate::state::PendingAudioStatus;

/// Signal to stop audio recording and communicate sample rate
#[derive(Resource, Default)]
pub struct AudioRecordingSignal {
    pub should_stop: Arc<Mutex<bool>>,
    pub actual_sample_rate: Arc<Mutex<u32>>,
}

/// Signal to control audio playback.
/// Uses `playing_vertex` as the sole control mechanism - threads check if they're
/// still the one that should be playing, eliminating race conditions.
#[derive(Resource, Clone, Default)]
pub struct AudioPlaybackState {
    pub playing_vertex: Arc<Mutex<Option<u64>>>,
}

/// Pre-decoded audio ready for instant playback
#[derive(Clone)]
pub struct PredecodedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Command sent to the persistent audio thread
#[allow(dead_code)]
enum AudioThreadCommand {
    /// Play pre-decoded audio for a vertex
    PlayPreloaded { vertex_id: u64, audio: PredecodedAudio },
    /// Play audio that needs decoding first
    PlayRaw { vertex_id: u64, data: Vec<u8> },
    /// Stop current playback (currently unused - stop via playing_vertex instead)
    Stop,
}

/// Cache for pre-decoded audio with persistent audio thread
#[derive(Resource)]
pub struct AudioPreloadCache {
    /// Pre-decoded PCM: vertex_id -> audio data
    pub cache: HashMap<u64, PredecodedAudio>,

    /// Currently pending pre-decodes
    pub pending: HashSet<u64>,

    /// Channel for receiving decoded audio from background threads
    pub decoded_tx: Sender<(u64, PredecodedAudio)>,
    pub decoded_rx: Receiver<(u64, PredecodedAudio)>,

    /// Channel for sending commands to the persistent audio thread
    audio_cmd_tx: Sender<AudioThreadCommand>,
}

impl AudioPreloadCache {
    /// Create a new AudioPreloadCache with persistent audio thread
    pub fn new(playing_vertex: Arc<Mutex<Option<u64>>>) -> Self {
        let (decoded_tx, decoded_rx) = crossbeam_channel::unbounded();
        let (audio_cmd_tx, audio_cmd_rx) = crossbeam_channel::unbounded::<AudioThreadCommand>();

        // Spawn persistent audio thread that owns the OutputStream
        thread::spawn(move || {
            let (stream, handle) = OutputStream::try_default()
                .expect("Failed to create persistent audio output stream");
            // Keep stream alive
            let _stream = stream;

            let mut current_sink: Option<Sink> = None;
            let mut current_vertex_id: Option<u64> = None;

            loop {
                // Check for new commands (non-blocking)
                match audio_cmd_rx.try_recv() {
                    Ok(AudioThreadCommand::PlayPreloaded { vertex_id, audio }) => {
                        // Stop current playback
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }

                        let sink = Sink::try_new(&handle)
                            .expect("Failed to create audio sink");
                        let buffer = rodio::buffer::SamplesBuffer::new(
                            audio.channels,
                            audio.sample_rate,
                            audio.samples,
                        );
                        sink.append(buffer);
                        current_sink = Some(sink);
                        current_vertex_id = Some(vertex_id);
                    }
                    Ok(AudioThreadCommand::PlayRaw { vertex_id, data }) => {
                        // Stop current playback
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }

                        // Decode and play
                        if let Some(decoded) = predecode_audio(&data) {
                            let sink = Sink::try_new(&handle)
                                .expect("Failed to create audio sink");
                            let buffer = rodio::buffer::SamplesBuffer::new(
                                decoded.channels,
                                decoded.sample_rate,
                                decoded.samples,
                            );
                            sink.append(buffer);
                            current_sink = Some(sink);
                            current_vertex_id = Some(vertex_id);
                        }
                    }
                    Ok(AudioThreadCommand::Stop) => {
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }
                        current_vertex_id = None;
                    }
                    Err(crossbeam_channel::TryRecvError::Empty) => {}
                    Err(crossbeam_channel::TryRecvError::Disconnected) => {
                        // Main thread dropped the sender, exit
                        break;
                    }
                }

                // Check if playback finished
                if let Some(ref sink) = current_sink {
                    if sink.empty() {
                        current_sink = None;
                        // Clear playing state
                        if let Ok(mut playing) = playing_vertex.lock() {
                            if *playing == current_vertex_id {
                                *playing = None;
                            }
                        }
                        current_vertex_id = None;
                    } else {
                        // Check if we should stop (another vertex started playing)
                        if let Ok(playing) = playing_vertex.lock() {
                            if *playing != current_vertex_id {
                                sink.stop();
                                current_sink = None;
                                current_vertex_id = None;
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }
        });

        Self {
            cache: HashMap::new(),
            pending: HashSet::new(),
            decoded_tx,
            decoded_rx,
            audio_cmd_tx,
        }
    }

    /// Send a play command to the audio thread
    pub fn play_preloaded(&self, vertex_id: u64, audio: PredecodedAudio) {
        let _ = self.audio_cmd_tx.send(AudioThreadCommand::PlayPreloaded { vertex_id, audio });
    }

    /// Send a play raw command to the audio thread (decode on audio thread)
    pub fn play_raw(&self, vertex_id: u64, data: Vec<u8>) {
        let _ = self.audio_cmd_tx.send(AudioThreadCommand::PlayRaw { vertex_id, data });
    }

    /// Send a stop command to the audio thread (currently unused - stop via playing_vertex)
    #[allow(dead_code)]
    pub fn stop(&self) {
        let _ = self.audio_cmd_tx.send(AudioThreadCommand::Stop);
    }
}

/// Play audio data using rodio.
/// Uses `playing_vertex` to track which vertex should be playing - threads check
/// if they're still the active one and stop if not, eliminating race conditions.
pub fn play_audio(data: &[u8], _mime: &str, vertex_id: u64, state: &AudioPlaybackState) {
    use crate::media::calculate_audio_rms;
    use crate::tts;

    // Set this vertex as the one that should be playing.
    // Any previous playback thread will see it's no longer active and stop.
    if let Ok(mut playing) = state.playing_vertex.lock() {
        *playing = Some(vertex_id);
    }

    // Calculate RMS of audio for TTS volume calibration
    if let Some(rms) = calculate_audio_rms(data) {
        tts::set_reference_audio_level(rms);
    }

    let data_vec = data.to_vec();
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
                                // Poll to check if we're still the active playback
                                while !sink.empty() {
                                    if let Ok(playing) = playing_vertex.lock() {
                                        if *playing != Some(vertex_id) {
                                            // Another vertex is now playing, or playback was stopped
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

        // Clear playing state when done (only if we're still the active one)
        if let Ok(mut playing) = playing_vertex.lock() {
            if *playing == Some(vertex_id) {
                *playing = None;
            }
        }
    });
}

/// Decode audio to PCM samples synchronously
fn predecode_audio(data: &[u8]) -> Option<PredecodedAudio> {
    use rodio::Decoder;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).ok()?;

    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();

    // Collect all samples
    let samples: Vec<i16> = decoder.collect();

    Some(PredecodedAudio {
        samples,
        sample_rate,
        channels,
    })
}

/// Decode audio to PCM in background thread
pub fn predecode_audio_async(
    vertex_id: u64,
    data: Vec<u8>,
    tx: Sender<(u64, PredecodedAudio)>,
) {
    thread::spawn(move || {
        if let Some(decoded) = predecode_audio(&data) {
            let _ = tx.send((vertex_id, decoded));
        }
    });
}

/// Stop any currently playing audio
/// The audio thread monitors playing_vertex and will stop when it becomes None
pub fn stop_audio(state: &AudioPlaybackState) {
    if let Ok(mut playing) = state.playing_vertex.lock() {
        *playing = None;
    }
}

/// Play audio using preloaded cache - instant playback via SamplesBuffer
pub fn play_audio_fast(
    vertex_id: u64,
    data: &[u8],
    _mime: &str,
    preload_cache: &AudioPreloadCache,
    playback_state: &AudioPlaybackState,
) {
    use crate::media::calculate_audio_rms;
    use crate::tts;

    // Update playing state
    if let Ok(mut playing) = playback_state.playing_vertex.lock() {
        *playing = Some(vertex_id);
    }

    // Calculate RMS of audio for TTS volume calibration
    if let Some(rms) = calculate_audio_rms(data) {
        tts::set_reference_audio_level(rms);
    }

    // Try to use preloaded audio first
    if let Some(preloaded) = preload_cache.cache.get(&vertex_id) {
        preload_cache.play_preloaded(vertex_id, preloaded.clone());
        return;
    }

    // Not in cache - send raw data to audio thread for decoding
    preload_cache.play_raw(vertex_id, data.to_vec());
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

// ============================================================================
// Audio Normalization
// ============================================================================

/// Target RMS level in dB (relative to full scale)
/// -20 dBFS leaves headroom for peaks while being audible
const TARGET_RMS_DB: f32 = -20.0;

/// Below this RMS, audio is considered silent and normalization is skipped
const SILENCE_THRESHOLD: f32 = 1e-6;

/// Maximum gain boost in dB (prevents amplifying noise too much)
const MAX_GAIN_DB: f32 = 40.0;

/// Minimum gain (maximum attenuation) in dB
const MIN_GAIN_DB: f32 = -20.0;

/// Threshold above which soft limiting kicks in
const LIMITER_THRESHOLD: f32 = 0.9;

/// Normalize audio to consistent loudness with soft limiting for ear protection.
///
/// Uses RMS (root mean square) normalization which correlates well with perceived
/// loudness for voice. After normalization, applies soft limiting to any peaks
/// above 0.9 to prevent clipping and ear-damaging loud pops.
///
/// Returns the gain applied (1.0 = no change).
pub fn normalize_audio(samples: &mut [f32]) -> f32 {
    if samples.is_empty() {
        return 1.0;
    }

    // Stage 1: Calculate RMS
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();

    // Skip normalization for silent audio
    if rms < SILENCE_THRESHOLD {
        return 1.0;
    }

    // Calculate target RMS in linear scale
    // dB to linear: 10^(dB/20)
    let target_rms = 10.0_f32.powf(TARGET_RMS_DB / 20.0);

    // Calculate gain needed
    let mut gain = target_rms / rms;

    // Limit gain to prevent excessive amplification or attenuation
    let max_gain = 10.0_f32.powf(MAX_GAIN_DB / 20.0);
    let min_gain = 10.0_f32.powf(MIN_GAIN_DB / 20.0);
    gain = gain.clamp(min_gain, max_gain);

    // Apply gain
    for sample in samples.iter_mut() {
        *sample *= gain;
    }

    // Stage 2: Soft limiting for ear protection
    // Uses tanh-based soft curve for samples above threshold
    for sample in samples.iter_mut() {
        let abs_val = sample.abs();
        if abs_val > LIMITER_THRESHOLD {
            // Soft curve: map [threshold, infinity) -> [threshold, 1.0)
            // Using tanh to smoothly compress peaks
            let excess = abs_val - LIMITER_THRESHOLD;
            let compressed = LIMITER_THRESHOLD + (1.0 - LIMITER_THRESHOLD) * (excess / (1.0 + excess)).tanh();
            // Ensure we never exceed 0.99 (ear protection)
            let limited = compressed.min(0.99);
            *sample = sample.signum() * limited;
        }
    }

    gain
}

// ============================================================================
// Background Audio Processing
// ============================================================================

/// Result from background audio processing
#[derive(Debug)]
pub enum AudioProcessingResult {
    /// Audio encoding completed successfully
    Encoded {
        local_id: u64,
        ogg_data: Vec<u8>,
        samples: Vec<f32>,
        sample_rate: u32,
    },
    /// Audio encoding failed
    EncodingFailed {
        local_id: u64,
        error: String,
    },
    /// Status update (for UI feedback)
    StatusUpdate {
        local_id: u64,
        status: PendingAudioStatus,
    },
}

/// Channel for receiving audio processing results in the main thread
#[derive(Resource)]
pub struct AudioProcessingChannel {
    pub tx: Sender<AudioProcessingResult>,
    pub rx: Receiver<AudioProcessingResult>,
}

impl Default for AudioProcessingChannel {
    fn default() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }
}

/// Generate a waveform preview from raw samples for visualization
/// Downsamples to num_points amplitude values (RMS of each chunk)
pub fn generate_waveform_preview(samples: &[f32], num_points: usize) -> Vec<f32> {
    if samples.is_empty() || num_points == 0 {
        return vec![0.0; num_points.max(1)];
    }

    let chunk_size = (samples.len() / num_points).max(1);
    samples
        .chunks(chunk_size)
        .take(num_points)
        .map(|chunk| {
            // RMS (root mean square) gives a better visual than peak
            let sum_sq: f32 = chunk.iter().map(|s| s * s).sum();
            (sum_sq / chunk.len() as f32).sqrt()
        })
        .collect()
}

/// Spawn a background task to encode audio and send results back via channel
pub fn spawn_audio_encoding_task(
    local_id: u64,
    samples: Vec<f32>,
    sample_rate: u32,
    result_tx: Sender<AudioProcessingResult>,
) {
    thread::spawn(move || {
        // Send status update - encoding
        let _ = result_tx.send(AudioProcessingResult::StatusUpdate {
            local_id,
            status: PendingAudioStatus::Encoding,
        });

        // Normalize audio for consistent loudness
        let mut samples = samples;
        let gain = normalize_audio(&mut samples);
        eprintln!(
            "Audio normalization: local_id={} gain={:.2}x ({:.1} dB)",
            local_id,
            gain,
            20.0 * gain.log10()
        );

        // Encode to OGG Vorbis
        match encode_ogg_vorbis(&samples, sample_rate, None) {
            Ok(ogg_data) => {
                eprintln!(
                    "Background encoding complete: local_id={} {} samples -> {} bytes",
                    local_id,
                    samples.len(),
                    ogg_data.len()
                );
                let _ = result_tx.send(AudioProcessingResult::Encoded {
                    local_id,
                    ogg_data,
                    samples,
                    sample_rate,
                });
            }
            Err(e) => {
                eprintln!("Background encoding failed: local_id={} error={}", local_id, e);
                let _ = result_tx.send(AudioProcessingResult::EncodingFailed {
                    local_id,
                    error: e.to_string(),
                });
            }
        }
    });
}
