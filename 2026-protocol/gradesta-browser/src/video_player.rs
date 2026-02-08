//! Video player module for native in-browser video playback
//!
//! Uses mp4 crate for video demuxing, openh264 for H.264 decoding,
//! and symphonia for audio decoding. Audio playback via rodio.

use crossbeam_channel::{Receiver, Sender};
use rodio::{OutputStream, Sink, Source};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// A decoded video frame ready for display
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub pts: Duration, // Presentation timestamp
}

/// Commands sent to the video player thread
#[derive(Clone, Debug)]
pub enum VideoPlayerCommand {
    Play,
    Pause,
    Seek(Duration),
    Stop,
}

/// Current state of the video player
#[derive(Clone, Debug, PartialEq)]
pub enum VideoPlayerState {
    Stopped,
    Playing,
    Paused,
    Finished,
}

/// Video player that decodes video and audio in background threads
pub struct VideoPlayer {
    /// Receiver for decoded frames
    pub frame_rx: Receiver<VideoFrame>,
    /// Sender for commands to the video player thread
    pub command_tx: Sender<VideoPlayerCommand>,
    /// Sender for commands to the audio player thread
    audio_command_tx: Option<Sender<VideoPlayerCommand>>,
    /// Video duration
    pub duration: Duration,
    /// Current playback position (atomic for lock-free access)
    position_ms: Arc<AtomicU64>,
    /// Playing state
    is_playing: Arc<AtomicBool>,
    /// Video dimensions
    pub width: u32,
    pub height: u32,
    /// Frames per second
    pub fps: f64,
}

impl VideoPlayer {
    /// Create a new video player from MP4 data
    pub fn new(data: Vec<u8>) -> Result<Self, String> {
        // Parse MP4 to get metadata
        let cursor = Cursor::new(&data);
        let size = data.len() as u64;
        let mp4 = mp4::Mp4Reader::read_header(cursor, size)
            .map_err(|e| format!("Failed to read MP4 header: {}", e))?;

        // Find video track
        let video_track_id = mp4
            .tracks()
            .iter()
            .find(|(_, track)| matches!(track.track_type(), Ok(mp4::TrackType::Video)))
            .map(|(id, _)| *id)
            .ok_or_else(|| "No video track found".to_string())?;

        // Find audio track (optional)
        let audio_track_id = mp4
            .tracks()
            .iter()
            .find(|(_, track)| matches!(track.track_type(), Ok(mp4::TrackType::Audio)))
            .map(|(id, _)| *id);

        let track = mp4.tracks().get(&video_track_id).unwrap();
        let width = track.width() as u32;
        let height = track.height() as u32;
        let duration_ms = track.duration().as_millis() as u64;
        let duration = Duration::from_millis(duration_ms);
        let sample_count = track.sample_count();
        let fps = if duration_ms > 0 {
            (sample_count as f64) / (duration_ms as f64 / 1000.0)
        } else {
            30.0
        };

        eprintln!(
            "VideoPlayer: {}x{}, {} samples, {:.2} fps, duration: {:?}, audio: {}",
            width, height, sample_count, fps, duration,
            if audio_track_id.is_some() { "yes" } else { "no" }
        );

        // Create channels
        let (frame_tx, frame_rx) = crossbeam_channel::bounded(30);
        let (command_tx, command_rx) = crossbeam_channel::unbounded();

        let position_ms = Arc::new(AtomicU64::new(0));
        let is_playing = Arc::new(AtomicBool::new(true));

        let position_clone = Arc::clone(&position_ms);
        let is_playing_clone = Arc::clone(&is_playing);

        // Clone data for audio thread
        let audio_data = data.clone();
        let audio_position = Arc::clone(&position_ms);
        let audio_playing = Arc::clone(&is_playing);

        // Spawn video decoder thread
        thread::spawn(move || {
            if let Err(e) = decode_video(
                data,
                video_track_id,
                frame_tx,
                command_rx,
                position_clone,
                is_playing_clone,
            ) {
                eprintln!("VideoPlayer: Video decode error: {}", e);
            }
        });

        // Spawn audio decoder thread if audio track exists
        let audio_command_tx = if audio_track_id.is_some() {
            let (audio_cmd_tx, audio_cmd_rx) = crossbeam_channel::unbounded();
            thread::spawn(move || {
                if let Err(e) = decode_and_play_audio(audio_data, audio_cmd_rx, audio_position, audio_playing) {
                    eprintln!("VideoPlayer: Audio decode error: {}", e);
                }
            });
            Some(audio_cmd_tx)
        } else {
            None
        };

        Ok(Self {
            frame_rx,
            command_tx,
            audio_command_tx,
            duration,
            position_ms,
            is_playing,
            width,
            height,
            fps,
        })
    }

    pub fn play(&self) {
        self.is_playing.store(true, Ordering::SeqCst);
        let _ = self.command_tx.send(VideoPlayerCommand::Play);
        if let Some(ref tx) = self.audio_command_tx {
            let _ = tx.send(VideoPlayerCommand::Play);
        }
    }

    pub fn pause(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        let _ = self.command_tx.send(VideoPlayerCommand::Pause);
        if let Some(ref tx) = self.audio_command_tx {
            let _ = tx.send(VideoPlayerCommand::Pause);
        }
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        let _ = self.command_tx.send(VideoPlayerCommand::Stop);
        if let Some(ref tx) = self.audio_command_tx {
            let _ = tx.send(VideoPlayerCommand::Stop);
        }
    }

    pub fn seek(&self, position: Duration) {
        let _ = self.command_tx.send(VideoPlayerCommand::Seek(position));
        if let Some(ref tx) = self.audio_command_tx {
            let _ = tx.send(VideoPlayerCommand::Seek(position));
        }
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
    }

    pub fn get_position(&self) -> Duration {
        Duration::from_millis(self.position_ms.load(Ordering::SeqCst))
    }

    pub fn set_position(&self, pos: Duration) {
        self.position_ms.store(pos.as_millis() as u64, Ordering::SeqCst);
    }
}

/// Background thread function that decodes video frames
fn decode_video(
    data: Vec<u8>,
    video_track_id: u32,
    frame_tx: Sender<VideoFrame>,
    command_rx: Receiver<VideoPlayerCommand>,
    position_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
) -> Result<(), String> {
    // Initialize decoder
    let mut decoder = openh264::decoder::Decoder::new()
        .map_err(|e| format!("Failed to create H.264 decoder: {:?}", e))?;

    // Open MP4 for reading samples
    let cursor = Cursor::new(&data);
    let size = data.len() as u64;
    let mut mp4 = mp4::Mp4Reader::read_header(cursor, size)
        .map_err(|e| format!("Failed to read MP4: {}", e))?;

    let sample_count = mp4.sample_count(video_track_id).unwrap_or(0);
    let track = mp4.tracks().get(&video_track_id).unwrap();
    let timescale = track.timescale();

    // Get SPS and PPS from track (clone to avoid borrow issues)
    let sps = track.sequence_parameter_set().map_err(|e| format!("No SPS: {}", e))?.to_vec();
    let pps = track.picture_parameter_set().map_err(|e| format!("No PPS: {}", e))?.to_vec();

    // NAL length size - typically 4 bytes for AVC
    let nal_length_size = 4usize;

    // Feed SPS/PPS to decoder with start code
    let mut sps_nal = vec![0, 0, 0, 1];
    sps_nal.extend_from_slice(&sps);
    let _ = decoder.decode(&sps_nal);

    let mut pps_nal = vec![0, 0, 0, 1];
    pps_nal.extend_from_slice(&pps);
    let _ = decoder.decode(&pps_nal);

    let playback_start = Instant::now();
    let mut pause_offset = Duration::ZERO;
    let mut paused_at: Option<Instant> = None;
    let mut sample_id = 1u32;
    let mut seek_target: Option<Duration> = None;

    while sample_id <= sample_count {
        // Check for commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                VideoPlayerCommand::Play => {
                    if let Some(paused) = paused_at.take() {
                        pause_offset += paused.elapsed();
                    }
                }
                VideoPlayerCommand::Pause => {
                    if paused_at.is_none() {
                        paused_at = Some(Instant::now());
                    }
                }
                VideoPlayerCommand::Stop => {
                    return Ok(());
                }
                VideoPlayerCommand::Seek(target) => {
                    seek_target = Some(target);
                }
            }
        }

        // Handle seeking
        if let Some(target) = seek_target.take() {
            // Find the sample closest to target time
            let target_ticks = (target.as_secs_f64() * timescale as f64) as u64;

            // Find keyframe at or before target
            let mut best_sample = 1u32;
            for sid in 1..=sample_count {
                if let Ok(Some(sample)) = mp4.read_sample(video_track_id, sid) {
                    if sample.start_time <= target_ticks {
                        if sample.is_sync {
                            best_sample = sid;
                        }
                    } else {
                        break;
                    }
                }
            }

            // Re-initialize decoder for clean seek
            decoder = openh264::decoder::Decoder::new()
                .map_err(|e| format!("Failed to create decoder: {:?}", e))?;
            let mut sps_nal = vec![0, 0, 0, 1];
            sps_nal.extend_from_slice(&sps);
            let _ = decoder.decode(&sps_nal);
            let mut pps_nal = vec![0, 0, 0, 1];
            pps_nal.extend_from_slice(&pps);
            let _ = decoder.decode(&pps_nal);

            sample_id = best_sample;

            // Adjust timing
            position_ms.store(target.as_millis() as u64, Ordering::SeqCst);

            eprintln!("VideoPlayer: Seeked to sample {} for target {:?}", sample_id, target);
        }

        // Handle pause
        if !is_playing.load(Ordering::SeqCst) {
            if paused_at.is_none() {
                paused_at = Some(Instant::now());
            }
            thread::sleep(Duration::from_millis(10));
            continue;
        } else if let Some(paused) = paused_at.take() {
            pause_offset += paused.elapsed();
        }

        // Read sample
        let sample = match mp4.read_sample(video_track_id, sample_id) {
            Ok(Some(s)) => s,
            Ok(None) => break,
            Err(e) => {
                eprintln!("VideoPlayer: Error reading sample {}: {}", sample_id, e);
                sample_id += 1;
                continue;
            }
        };

        // Calculate presentation time
        let pts_ticks = sample.start_time;
        let pts = Duration::from_secs_f64(pts_ticks as f64 / timescale as f64);

        // Update position
        position_ms.store(pts.as_millis() as u64, Ordering::SeqCst);

        // Wait for the right time to present this frame
        let elapsed = playback_start.elapsed() - pause_offset;
        if pts > elapsed {
            let wait_time = pts - elapsed;
            if wait_time > Duration::from_millis(5) {
                thread::sleep(wait_time);
            }
        }

        // Parse NAL units from sample (AVCC format -> Annex B)
        let sample_data = &sample.bytes;
        let mut offset = 0;
        while offset + nal_length_size <= sample_data.len() {
            let nal_len = match nal_length_size {
                4 => u32::from_be_bytes([
                    sample_data[offset],
                    sample_data[offset + 1],
                    sample_data[offset + 2],
                    sample_data[offset + 3],
                ]) as usize,
                2 => u16::from_be_bytes([sample_data[offset], sample_data[offset + 1]]) as usize,
                1 => sample_data[offset] as usize,
                _ => break,
            };
            offset += nal_length_size;

            if offset + nal_len > sample_data.len() {
                break;
            }

            // Convert to Annex B format (prepend start code)
            let mut nal = vec![0, 0, 0, 1];
            nal.extend_from_slice(&sample_data[offset..offset + nal_len]);
            offset += nal_len;

            // Decode
            if let Ok(Some(yuv)) = decoder.decode(&nal) {
                // Use the library's built-in RGBA conversion
                let rgba_size = yuv.estimate_rgba_u8_size();
                let mut rgba = vec![0u8; rgba_size];
                yuv.write_rgba8(&mut rgba);

                use openh264::formats::YUVSource;
                let (w, h) = yuv.dimensions();

                // Try to send frame
                match frame_tx.try_send(VideoFrame {
                    width: w as u32,
                    height: h as u32,
                    rgba,
                    pts,
                }) {
                    Ok(()) => {}
                    Err(crossbeam_channel::TrySendError::Full(_)) => {}
                    Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                        return Ok(());
                    }
                }
            }
        }

        sample_id += 1;
    }

    eprintln!("VideoPlayer: Finished decoding {} samples", sample_count);
    Ok(())
}

/// Decode and play audio using symphonia and rodio
fn decode_and_play_audio(
    data: Vec<u8>,
    command_rx: Receiver<VideoPlayerCommand>,
    position_ms: Arc<AtomicU64>,
    is_playing: Arc<AtomicBool>,
) -> Result<(), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    use symphonia::core::units::Time;

    // Helper function to create format reader and decoder
    fn create_audio_pipeline(data: &[u8]) -> Result<(
        Box<dyn symphonia::core::formats::FormatReader>,
        Box<dyn symphonia::core::codecs::Decoder>,
        u32,  // track_id
        u32,  // sample_rate
        usize // channels
    ), String> {
        let cursor = Cursor::new(data.to_vec());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        let mut hint = Hint::new();
        hint.with_extension("mp4");

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Failed to probe audio: {}", e))?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No audio track found".to_string())?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        let audio_decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create audio decoder: {}", e))?;

        Ok((format, audio_decoder, track_id, sample_rate, channels))
    }

    let (mut format, mut audio_decoder, track_id, sample_rate, channels) =
        create_audio_pipeline(&data)?;

    eprintln!("VideoPlayer: Audio track: {} Hz, {} channels", sample_rate, channels);

    // Create audio output
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| format!("Failed to create audio output: {}", e))?;
    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| format!("Failed to create audio sink: {}", e))?;

    let mut seek_pending: Option<Duration> = None;

    // Decode and play audio
    loop {
        // Check for commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                VideoPlayerCommand::Play => {
                    sink.play();
                }
                VideoPlayerCommand::Pause => {
                    sink.pause();
                }
                VideoPlayerCommand::Stop => {
                    sink.stop();
                    return Ok(());
                }
                VideoPlayerCommand::Seek(target) => {
                    seek_pending = Some(target);
                }
            }
        }

        // Handle seeking
        if let Some(target) = seek_pending.take() {
            // Clear the current audio buffer
            sink.clear();

            // Re-create the audio pipeline to seek from the beginning
            let result = create_audio_pipeline(&data);
            if let Ok((new_format, new_decoder, _, _, _)) = result {
                format = new_format;
                audio_decoder = new_decoder;

                // Try to seek in the format
                let seek_time = Time::from(target.as_secs_f64());
                if let Err(e) = format.seek(SeekMode::Coarse, SeekTo::Time { time: seek_time, track_id: Some(track_id) }) {
                    eprintln!("VideoPlayer: Audio seek error: {}", e);
                } else {
                    eprintln!("VideoPlayer: Audio seeked to {:?}", target);
                }
            }
        }

        // Check if we should be paused
        if !is_playing.load(Ordering::SeqCst) {
            sink.pause();
            thread::sleep(Duration::from_millis(10));
            continue;
        } else {
            sink.play();
        }

        // Read next packet
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break; // End of stream
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                // Reset required after seek
                audio_decoder.reset();
                continue;
            }
            Err(e) => {
                eprintln!("VideoPlayer: Audio packet error: {}", e);
                break;
            }
        };

        // Skip packets from other tracks
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet
        match audio_decoder.decode(&packet) {
            Ok(decoded) => {
                // Convert to samples
                let spec = *decoded.spec();
                let duration = decoded.capacity() as u64;

                let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                sample_buf.copy_interleaved_ref(decoded);

                let samples = sample_buf.samples().to_vec();

                // Create a source from the samples
                let source = SamplesSource {
                    samples,
                    position: 0,
                    sample_rate,
                    channels: channels as u16,
                };

                sink.append(source);
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {
                // Skip decode errors (can happen after seek)
                continue;
            }
            Err(e) => {
                eprintln!("VideoPlayer: Audio decode error: {}", e);
            }
        }
    }

    // Wait for audio to finish
    sink.sleep_until_end();
    eprintln!("VideoPlayer: Audio finished");
    Ok(())
}

/// A rodio source from decoded samples
struct SamplesSource {
    samples: Vec<f32>,
    position: usize,
    sample_rate: u32,
    channels: u16,
}

impl Iterator for SamplesSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for SamplesSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let total_samples = self.samples.len() / self.channels as usize;
        Some(Duration::from_secs_f64(total_samples as f64 / self.sample_rate as f64))
    }
}
