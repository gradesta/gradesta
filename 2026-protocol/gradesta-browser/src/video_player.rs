//! Video player module for native in-browser video playback
//!
//! Uses mp4 crate for demuxing and openh264 for H.264 decoding.

use crossbeam_channel::{Receiver, Sender};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
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
    Error(String),
}

/// Video player that decodes video in a background thread
pub struct VideoPlayer {
    /// Receiver for decoded frames
    pub frame_rx: Receiver<VideoFrame>,
    /// Sender for commands to the player thread
    pub command_tx: Sender<VideoPlayerCommand>,
    /// Current state
    pub state: Arc<Mutex<VideoPlayerState>>,
    /// Video duration
    pub duration: Duration,
    /// Current playback position
    pub position: Arc<Mutex<Duration>>,
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
            .find(|(_, track)| {
                matches!(track.track_type(), Ok(mp4::TrackType::Video))
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| "No video track found".to_string())?;

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
            "VideoPlayer: {}x{}, {} samples, {:.2} fps, duration: {:?}",
            width, height, sample_count, fps, duration
        );

        // Create channels
        // Use bounded channel with enough capacity to smooth playback
        let (frame_tx, frame_rx) = crossbeam_channel::bounded(30); // ~1 second at 30fps
        let (command_tx, command_rx) = crossbeam_channel::unbounded();

        let state = Arc::new(Mutex::new(VideoPlayerState::Playing));
        let position = Arc::new(Mutex::new(Duration::ZERO));

        let state_clone = Arc::clone(&state);
        let position_clone = Arc::clone(&position);

        // Spawn decoder thread
        thread::spawn(move || {
            if let Err(e) = decode_video(
                data,
                video_track_id,
                frame_tx,
                command_rx,
                state_clone,
                position_clone,
            ) {
                eprintln!("VideoPlayer: Decode error: {}", e);
            }
        });

        Ok(Self {
            frame_rx,
            command_tx,
            state,
            duration,
            position,
            width,
            height,
            fps,
        })
    }

    pub fn play(&self) {
        let _ = self.command_tx.send(VideoPlayerCommand::Play);
        if let Ok(mut state) = self.state.lock() {
            *state = VideoPlayerState::Playing;
        }
    }

    pub fn pause(&self) {
        let _ = self.command_tx.send(VideoPlayerCommand::Pause);
        if let Ok(mut state) = self.state.lock() {
            *state = VideoPlayerState::Paused;
        }
    }

    pub fn stop(&self) {
        let _ = self.command_tx.send(VideoPlayerCommand::Stop);
        if let Ok(mut state) = self.state.lock() {
            *state = VideoPlayerState::Stopped;
        }
    }

    pub fn is_playing(&self) -> bool {
        matches!(
            self.state.lock().map(|s| s.clone()),
            Ok(VideoPlayerState::Playing)
        )
    }

    pub fn get_position(&self) -> Duration {
        self.position.lock().map(|p| *p).unwrap_or(Duration::ZERO)
    }
}

/// Background thread function that decodes video frames
fn decode_video(
    data: Vec<u8>,
    video_track_id: u32,
    frame_tx: Sender<VideoFrame>,
    command_rx: Receiver<VideoPlayerCommand>,
    state: Arc<Mutex<VideoPlayerState>>,
    position: Arc<Mutex<Duration>>,
) -> Result<(), String> {
    // Initialize decoder
    let mut decoder = openh264::decoder::Decoder::new()
        .map_err(|e| format!("Failed to create H.264 decoder: {:?}", e))?;

    // Re-open MP4 for reading samples
    let cursor = Cursor::new(&data);
    let size = data.len() as u64;
    let mut mp4 = mp4::Mp4Reader::read_header(cursor, size)
        .map_err(|e| format!("Failed to read MP4: {}", e))?;

    let sample_count = mp4.sample_count(video_track_id).unwrap_or(0);
    let track = mp4.tracks().get(&video_track_id).unwrap();
    let timescale = track.timescale();

    // Get SPS and PPS from track
    let sps = track.sequence_parameter_set().map_err(|e| format!("No SPS: {}", e))?;
    let pps = track.picture_parameter_set().map_err(|e| format!("No PPS: {}", e))?;

    // NAL length size - typically 4 bytes for AVC
    let nal_length_size = 4usize;

    // Feed SPS/PPS to decoder with start code
    let mut sps_nal = vec![0, 0, 0, 1];
    sps_nal.extend_from_slice(sps);
    let _ = decoder.decode(&sps_nal);

    let mut pps_nal = vec![0, 0, 0, 1];
    pps_nal.extend_from_slice(pps);
    let _ = decoder.decode(&pps_nal);

    let mut playing = true;
    let mut sample_id = 1u32;
    // Start the playback clock AFTER setup is complete
    let playback_start = Instant::now();
    let mut paused_at: Option<Instant> = None;
    let mut pause_duration = Duration::ZERO;
    let mut frames_sent = 0u32;

    while sample_id <= sample_count {
        // Check for commands
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                VideoPlayerCommand::Play => {
                    if let Some(paused) = paused_at.take() {
                        pause_duration += paused.elapsed();
                    }
                    playing = true;
                }
                VideoPlayerCommand::Pause => {
                    paused_at = Some(Instant::now());
                    playing = false;
                }
                VideoPlayerCommand::Stop => {
                    if let Ok(mut s) = state.lock() {
                        *s = VideoPlayerState::Stopped;
                    }
                    return Ok(());
                }
                VideoPlayerCommand::Seek(_target) => {
                    // Seeking is complex with H.264 (need to find keyframes)
                    // For now, just ignore seek commands
                }
            }
        }

        if !playing {
            thread::sleep(Duration::from_millis(10));
            continue;
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
        if let Ok(mut pos) = position.lock() {
            *pos = pts;
        }

        // Calculate how far ahead/behind we are
        let elapsed = playback_start.elapsed() - pause_duration;

        // Wait for the right time to present this frame
        // Only wait if we're ahead of schedule
        if pts > elapsed {
            let wait_time = pts - elapsed;
            // Sleep if we need to wait more than 5ms
            if wait_time > Duration::from_millis(5) {
                thread::sleep(wait_time);
            }
        }
        // If we're behind, just keep decoding as fast as possible (no skip)

        // Parse NAL units from sample (AVCC format -> Annex B)
        let sample_data = &sample.bytes;
        let mut offset = 0;
        while offset + nal_length_size <= sample_data.len() {
            // Read NAL unit length
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

                // Get dimensions
                use openh264::formats::YUVSource;
                let (w, h) = yuv.dimensions();

                // Try to send frame, but don't block - drop frames if buffer is full
                match frame_tx.try_send(VideoFrame {
                    width: w as u32,
                    height: h as u32,
                    rgba,
                    pts,
                }) {
                    Ok(()) => {
                        frames_sent += 1;
                        if frames_sent % 30 == 0 {
                            eprintln!("VideoPlayer: sent {} frames, sample {}/{}", frames_sent, sample_id, sample_count);
                        }
                    }
                    Err(crossbeam_channel::TrySendError::Full(_)) => {
                        eprintln!("VideoPlayer: buffer full at frame {}", frames_sent);
                    }
                    Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                        eprintln!("VideoPlayer: receiver disconnected");
                        return Ok(());
                    }
                }
            }
        }

        sample_id += 1;
    }

    // Mark as finished
    if let Ok(mut s) = state.lock() {
        *s = VideoPlayerState::Finished;
    }

    Ok(())
}
