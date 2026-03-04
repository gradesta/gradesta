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

use crate::audio_processing::{get_audio_speed, TimeStretcher};
use crate::state::PendingAudioStatus;

// Re-export speed control functions from audio_processing
pub use crate::audio_processing::{normalize_audio, set_audio_speed};

/// Pre-computed speed levels for burst playback (0.5 increments from 1.5 to 4.0)
pub const BURST_SPEEDS: [f32; 6] = [1.5, 2.0, 2.5, 3.0, 3.5, 4.0];

/// Time-stretch samples to a target speed using OLA algorithm
fn stretch_samples(samples: &[i16], sample_rate: u32, channels: u16, speed: f32) -> Vec<i16> {
    let mut stretcher = TimeStretcher::new(sample_rate, channels);
    stretcher.set_speed(speed);
    stretcher.write(samples);
    stretcher.flush();

    // Estimate output size (input / speed) with some buffer
    let estimated = ((samples.len() as f32) / speed * 1.5) as usize;
    let mut output = vec![0i16; estimated.max(8192)];
    let mut total = 0;

    loop {
        if total >= output.len() {
            output.resize(output.len() * 2, 0);
        }
        let n = stretcher.read(&mut output[total..]);
        if n == 0 {
            break;
        }
        total += n;
    }

    output.truncate(total);
    output
}

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

/// Pre-decoded audio ready for instant playback with pre-stretched burst versions
#[derive(Clone)]
pub struct PredecodedAudio {
    /// Normal speed samples (1x)
    pub samples: Vec<i16>,
    /// Pre-stretched versions for burst speeds (1.5x, 2.0x, 2.5x, 3.0x, 3.5x, 4.0x)
    /// Index 0 = 1.5x, Index 1 = 2.0x, etc.
    pub burst_samples: Vec<Vec<i16>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl PredecodedAudio {
    /// Get the appropriate samples for the given speed.
    /// Returns (samples, playback_rate) where playback_rate adjusts for exact speed.
    pub fn samples_for_speed(&self, speed: f32) -> (&[i16], f32) {
        if speed <= 1.01 {
            // Normal speed
            return (&self.samples, 1.0);
        }

        // Find the closest pre-stretched version
        // We want the pre-stretched version that's <= target speed if possible
        // Then adjust playback rate to hit exact speed
        for (i, &burst_speed) in BURST_SPEEDS.iter().enumerate() {
            if i < self.burst_samples.len() && !self.burst_samples[i].is_empty() {
                if (burst_speed - speed).abs() < 0.01 {
                    // Exact match
                    return (&self.burst_samples[i], 1.0);
                } else if burst_speed > speed && i > 0 {
                    // Use previous (slower) burst and speed up playback
                    let prev_speed = BURST_SPEEDS[i - 1];
                    let playback_rate = speed / prev_speed;
                    return (&self.burst_samples[i - 1], playback_rate);
                }
            }
        }

        // Use highest available burst speed
        if let Some(last_burst) = self.burst_samples.last() {
            if !last_burst.is_empty() {
                let playback_rate = speed / BURST_SPEEDS[self.burst_samples.len() - 1];
                return (last_burst, playback_rate);
            }
        }

        // Fallback to normal samples with speed adjustment (will change pitch)
        (&self.samples, speed)
    }
}

/// A rodio Source that uses pre-stretched buffers for speed changes.
/// Monitors the global speed atomic and returns samples from the appropriate buffer.
pub struct BurstAwareSource {
    audio: PredecodedAudio,
    /// Current position in whichever buffer we're using
    position: usize,
    /// Index of current buffer: None = normal, Some(i) = burst_samples[i]
    current_buffer_idx: Option<usize>,
    /// Last speed we selected a buffer for
    last_speed: f32,
}

impl BurstAwareSource {
    pub fn new(audio: PredecodedAudio) -> Self {
        let speed = get_audio_speed();
        let buffer_idx = Self::buffer_index_for_speed(speed);
        Self {
            audio,
            position: 0,
            current_buffer_idx: buffer_idx,
            last_speed: speed,
        }
    }

    fn buffer_index_for_speed(speed: f32) -> Option<usize> {
        if speed <= 1.01 {
            return None; // Normal buffer
        }
        // Find the closest burst buffer
        for (i, &burst_speed) in BURST_SPEEDS.iter().enumerate() {
            if speed <= burst_speed + 0.01 {
                return Some(i);
            }
        }
        // Use highest burst
        Some(BURST_SPEEDS.len() - 1)
    }

    fn current_samples(&self) -> &[i16] {
        match self.current_buffer_idx {
            None => &self.audio.samples,
            Some(i) if i < self.audio.burst_samples.len() => &self.audio.burst_samples[i],
            _ => &self.audio.samples,
        }
    }
}

impl Iterator for BurstAwareSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        // Check if speed changed significantly every ~1000 samples
        if self.position % 1000 == 0 {
            let current_speed = get_audio_speed();
            let new_idx = Self::buffer_index_for_speed(current_speed);

            if new_idx != self.current_buffer_idx {
                // Speed changed, switch buffers
                // Map position from old buffer to new buffer
                let old_samples = self.current_samples();
                let old_progress = if old_samples.is_empty() {
                    0.0
                } else {
                    self.position as f64 / old_samples.len() as f64
                };

                self.current_buffer_idx = new_idx;
                let new_samples = self.current_samples();
                self.position = ((old_progress * new_samples.len() as f64) as usize)
                    .min(new_samples.len().saturating_sub(1));
                self.last_speed = current_speed;
            }
        }

        let samples = self.current_samples();
        if self.position >= samples.len() {
            return None;
        }

        let sample = samples[self.position];
        self.position += 1;
        Some(sample)
    }
}

impl Source for BurstAwareSource {
    fn current_frame_len(&self) -> Option<usize> {
        let samples = self.current_samples();
        Some(samples.len() - self.position)
    }

    fn channels(&self) -> u16 {
        self.audio.channels
    }

    fn sample_rate(&self) -> u32 {
        self.audio.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None // Can change based on speed
    }
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
                        // Use burst-aware source with pre-stretched buffers
                        let source = BurstAwareSource::new(audio);
                        sink.append(source);
                        current_sink = Some(sink);
                        current_vertex_id = Some(vertex_id);
                    }
                    Ok(AudioThreadCommand::PlayRaw { vertex_id, data }) => {
                        // Stop current playback
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }

                        // Decode and play (includes pre-stretching)
                        if let Some(decoded) = predecode_audio(&data) {
                            let sink = Sink::try_new(&handle)
                                .expect("Failed to create audio sink");
                            // Use burst-aware source with pre-stretched buffers
                            let source = BurstAwareSource::new(decoded);
                            sink.append(source);
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
                let mut should_clear_sink = false;
                let mut should_stop_sink = false;

                if let Some(ref sink) = current_sink {
                    if sink.empty() {
                        should_clear_sink = true;
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
                                should_stop_sink = true;
                                current_vertex_id = None;
                            }
                        }
                        // Note: Speed changes are handled by PitchPreservingSource
                        // which checks the atomic speed and adjusts sonic in real-time
                    }
                }

                // Handle sink cleanup outside the borrow
                if should_stop_sink {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                } else if should_clear_sink {
                    current_sink = None;
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

/// Decode audio to PCM samples and pre-stretch to all burst speeds (in parallel)
fn predecode_audio(data: &[u8]) -> Option<PredecodedAudio> {
    use rodio::Decoder;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).ok()?;

    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();

    // Collect all samples
    let samples: Vec<i16> = decoder.collect();

    // Pre-stretch to all burst speeds in parallel
    let samples_arc = std::sync::Arc::new(samples.clone());
    let handles: Vec<_> = BURST_SPEEDS
        .iter()
        .map(|&speed| {
            let samples_clone = samples_arc.clone();
            let sr = sample_rate;
            let ch = channels;
            thread::spawn(move || stretch_samples(&samples_clone, sr, ch, speed))
        })
        .collect();

    // Collect results
    let burst_samples: Vec<Vec<i16>> = handles
        .into_iter()
        .map(|h| h.join().unwrap_or_default())
        .collect();

    Some(PredecodedAudio {
        samples,
        burst_samples,
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
