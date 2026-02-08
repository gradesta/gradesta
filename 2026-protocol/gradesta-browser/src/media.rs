//! Media loading, caching, and rendering helpers.
//!
//! This module handles loading and caching of images, audio waveforms,
//! GIF animations, and video players.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy_egui::egui;
use crossbeam_channel::{Receiver, Sender};
use gif::DecodeOptions;

use crate::video_player::VideoPlayer;

/// Decoded image ready to be uploaded to GPU
pub struct DecodedImage {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Animated GIF with frame textures and timing
#[derive(Clone)]
pub struct AnimatedGif {
    pub frames: Vec<egui::TextureHandle>,
    pub delays: Vec<Duration>,
    pub current_frame: usize,
    pub last_switch: Instant,
}

/// Cache for media resources
#[derive(Resource)]
pub struct MediaCache {
    pub textures: HashMap<u64, egui::TextureHandle>,
    pub animated_gifs: HashMap<u64, AnimatedGif>,
    /// Pre-computed waveform samples (0.0-1.0)
    pub waveforms: HashMap<u64, Vec<f32>>,
    /// Receiver for images decoded in background threads
    pub decoded_rx: Receiver<DecodedImage>,
    /// Sender for decoded images (cloned to background threads)
    pub decoded_tx: Sender<DecodedImage>,
    /// Set of vertex IDs currently being decoded (to avoid duplicate work)
    pub pending_decodes: HashSet<u64>,
    /// Active video players (vertex_id -> player)
    pub video_players: HashMap<u64, VideoPlayer>,
    /// Current video frame textures
    pub video_textures: HashMap<u64, egui::TextureHandle>,
}

impl Default for MediaCache {
    fn default() -> Self {
        let (decoded_tx, decoded_rx) = crossbeam_channel::unbounded();
        Self {
            textures: HashMap::new(),
            animated_gifs: HashMap::new(),
            waveforms: HashMap::new(),
            decoded_rx,
            decoded_tx,
            pending_decodes: HashSet::new(),
            video_players: HashMap::new(),
            video_textures: HashMap::new(),
        }
    }
}

/// Check if data looks like image content based on magic bytes
pub fn is_image_data(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // PNG magic bytes
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return true;
    }
    // JPEG magic bytes
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    // GIF magic bytes
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return true;
    }
    // WebP magic bytes
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return true;
    }
    false
}

/// Get or load a texture for an image vertex
pub fn get_or_load_texture<'a>(
    vertex_id: u64,
    data: &[u8],
    mime: &str,
    media_cache: &'a mut MediaCache,
    ctx: &egui::Context,
) -> Option<&'a egui::TextureHandle> {
    // Return cached texture if available
    if media_cache.textures.contains_key(&vertex_id) {
        return media_cache.textures.get(&vertex_id);
    }

    // Check for completed background decodes
    while let Ok(decoded) = media_cache.decoded_rx.try_recv() {
        media_cache.pending_decodes.remove(&decoded.id);
        let tex = ctx.load_texture(
            format!("vertex_{}", decoded.id),
            egui::ColorImage::from_rgba_unmultiplied(
                [decoded.width as usize, decoded.height as usize],
                &decoded.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        media_cache.textures.insert(decoded.id, tex);
    }

    // Return if now cached
    if media_cache.textures.contains_key(&vertex_id) {
        return media_cache.textures.get(&vertex_id);
    }

    // Skip if already decoding
    if media_cache.pending_decodes.contains(&vertex_id) {
        return None;
    }

    // Try to decode synchronously for small images, async for large ones
    if data.len() < 100_000 {
        // Small image - decode synchronously
        if let Ok(img) = image::load_from_memory(data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let tex = ctx.load_texture(
                format!("vertex_{}", vertex_id),
                egui::ColorImage::from_rgba_unmultiplied(
                    [width as usize, height as usize],
                    &rgba,
                ),
                egui::TextureOptions::LINEAR,
            );
            media_cache.textures.insert(vertex_id, tex);
            return media_cache.textures.get(&vertex_id);
        }
    } else {
        // Large image - decode in background
        media_cache.pending_decodes.insert(vertex_id);
        let data = data.to_vec();
        let tx = media_cache.decoded_tx.clone();
        let id = vertex_id;
        std::thread::spawn(move || {
            if let Ok(img) = image::load_from_memory(&data) {
                let rgba = img.to_rgba8();
                let (width, height) = rgba.dimensions();
                let _ = tx.send(DecodedImage {
                    id,
                    width,
                    height,
                    rgba: rgba.into_raw(),
                });
            }
        });
    }

    None
}

/// Get or load an animated GIF
pub fn get_or_load_animated_gif<'a>(
    vertex_id: u64,
    data: &[u8],
    media_cache: &'a mut MediaCache,
    ctx: &egui::Context,
) -> Option<&'a mut AnimatedGif> {
    if !media_cache.animated_gifs.contains_key(&vertex_id) {
        // Try to decode as animated GIF
        let cursor = std::io::Cursor::new(data);
        let mut opts = DecodeOptions::new();
        opts.set_color_output(gif::ColorOutput::RGBA);

        if let Ok(mut decoder) = opts.read_info(cursor) {
            let mut frames = Vec::new();
            let mut delays = Vec::new();

            while let Ok(Some(frame)) = decoder.read_next_frame() {
                let width = frame.width as usize;
                let height = frame.height as usize;

                if width == 0 || height == 0 {
                    continue;
                }

                let tex = ctx.load_texture(
                    format!("gif_{}_{}", vertex_id, frames.len()),
                    egui::ColorImage::from_rgba_unmultiplied([width, height], &frame.buffer),
                    egui::TextureOptions::LINEAR,
                );
                frames.push(tex);

                // GIF delay is in centiseconds (1/100 sec), minimum 20ms to avoid crazy speed
                let delay_cs = frame.delay.max(2) as u64;
                delays.push(Duration::from_millis(delay_cs * 10));
            }

            if !frames.is_empty() {
                media_cache.animated_gifs.insert(vertex_id, AnimatedGif {
                    frames,
                    delays,
                    current_frame: 0,
                    last_switch: Instant::now(),
                });
            }
        }
    }

    media_cache.animated_gifs.get_mut(&vertex_id)
}

/// Generate waveform data from audio for visualization
/// Returns a vector of normalized amplitude values (0.0 to 1.0) for display
pub fn generate_waveform(data: &[u8], num_bars: usize) -> Option<Vec<f32>> {
    use rodio::Decoder;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).ok()?;

    // Collect all samples
    let samples: Vec<f32> = decoder
        .map(|s| (s as f32 / i16::MAX as f32).abs())
        .collect();

    if samples.is_empty() {
        return None;
    }

    // Downsample to num_bars by taking max amplitude in each chunk
    let chunk_size = samples.len() / num_bars;
    if chunk_size == 0 {
        return Some(samples);
    }

    let waveform: Vec<f32> = samples
        .chunks(chunk_size)
        .take(num_bars)
        .map(|chunk| {
            chunk.iter().cloned().fold(0.0f32, |a, b| a.max(b))
        })
        .collect();

    Some(waveform)
}

/// Get or generate waveform for an audio vertex
pub fn get_or_generate_waveform<'a>(
    vertex_id: u64,
    data: &[u8],
    media_cache: &'a mut MediaCache,
    num_bars: usize,
) -> Option<&'a Vec<f32>> {
    if !media_cache.waveforms.contains_key(&vertex_id) {
        if let Some(waveform) = generate_waveform(data, num_bars) {
            media_cache.waveforms.insert(vertex_id, waveform);
        }
    }
    media_cache.waveforms.get(&vertex_id)
}

/// Draw a waveform in a given rect
pub fn draw_waveform(
    painter: &egui::Painter,
    rect: egui::Rect,
    waveform: &[f32],
    color: egui::Color32,
    bg_color: egui::Color32,
) {
    if waveform.is_empty() {
        return;
    }

    // Draw background
    painter.rect_filled(rect, 2.0, bg_color);

    let bar_width = rect.width() / waveform.len() as f32;
    let center_y = rect.center().y;
    let half_height = rect.height() * 0.4; // Leave some margin

    for (i, &amplitude) in waveform.iter().enumerate() {
        let x = rect.left() + i as f32 * bar_width;
        let bar_height = amplitude * half_height;

        // Draw bar from center up and down (mirrored waveform)
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x, center_y - bar_height),
            egui::pos2(x + bar_width * 0.8, center_y + bar_height),
        );
        painter.rect_filled(bar_rect, 1.0, color);
    }
}

/// Calculate the RMS (Root Mean Square) amplitude of audio data
/// Returns a value typically in the range 0.0-1.0 representing loudness
pub fn calculate_audio_rms(data: &[u8]) -> Option<f32> {
    use rodio::Decoder;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).ok()?;

    // Collect samples and calculate RMS
    let samples: Vec<f32> = decoder
        .map(|s| s as f32 / i16::MAX as f32)
        .collect();

    if samples.is_empty() {
        return None;
    }

    // Calculate RMS: sqrt(mean(samples^2))
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_squares / samples.len() as f32).sqrt();

    Some(rms)
}

/// Open content with an external application
pub fn open_with_external(data: &[u8], mime: &str) {
    // Determine file extension from MIME type
    let ext = match mime {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "audio/ogg" => "ogg",
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "application/pdf" => "pdf",
        _ => "bin",
    };

    let temp_path = format!("/tmp/gradesta_external.{}", ext);
    if std::fs::write(&temp_path, data).is_ok() {
        let _ = std::process::Command::new("xdg-open")
            .arg(&temp_path)
            .spawn();
    }
}
