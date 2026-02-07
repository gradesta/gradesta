//! File browser for Nextcloud files
//!
//! Structure:
//! - Directory listing is a vertical list (north/south)
//! - First entry has west edge to files portal (back to menu)
//! - Each entry has east edge to content (subdirectory or file view)
//! - Subdirectories: east goes to gradesta-url portal for lazy loading
//! - Files: east goes to gradesta-url portal for file view

use anyhow::Result;
use futures_util::SinkExt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::*;
use crate::State;

/// Max file size to fetch content (10 MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Generate deterministic hash for file entries
fn hash64(parts: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for p in parts {
        p.hash(&mut hasher);
        0u8.hash(&mut hasher);
    }
    hasher.finish()
}

/// Generate hash for files portal (matches router.rs)
fn files_portal_hash(identity: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("router:{}:files-portal", identity).hash(&mut hasher);
    hasher.finish()
}

/// Handle files landmark requests (directory listing)
pub async fn handle_landmark<W>(
    state: &Arc<Mutex<State>>,
    write: &mut W,
    action_id: u64,
    path: &str,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (identity, nc) = {
        let s = state.lock().await;
        (
            s.identity.clone().unwrap_or_default(),
            s.nextcloud.clone(),
        )
    };

    let nc = nc.ok_or_else(|| anyhow::anyhow!("No Nextcloud client"))?;

    // Normalize path
    let dir_path = if path.is_empty() || path == "/" {
        "/".to_string()
    } else {
        format!("/{}", path.trim_matches('/'))
    };
    let is_root = dir_path == "/";

    // Send context - directory landmark ends with /
    let landmark = format!("nextcloud://{}{}/", identity, dir_path);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Send files portal label (so it shows as "Files" in the menu)
    let files_portal_id = files_portal_hash(&identity);
    let portal_msg = encode_set_vertex_label(action_id, files_portal_id, "text/plain", b"Files");
    write.send(Message::Binary(portal_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // List directory contents
    let files = nc.list_directory(&dir_path).await?;
    log::info!("Files: Listed {} items in '{}'", files.len(), dir_path);

    // Build entries
    struct Entry {
        name: String,
        path: String,
        size: u64,
        is_dir: bool,
        entry_id: u64,
        content_id: u64,
        content_url: String,
    }

    let mut entries: Vec<Entry> = Vec::new();

    // Calculate the "self entry" ID - this is the entry in the parent directory that points here
    let self_entry_id = if !is_root {
        let folder_name = dir_path.rsplit('/').next().unwrap_or(&dir_path);
        let parent_path = {
            let trimmed = dir_path.trim_end_matches('/');
            match trimmed.rsplit_once('/') {
                Some((parent, _)) if parent.is_empty() => "/".to_string(),
                Some((parent, _)) => parent.to_string(),
                None => "/".to_string(),
            }
        };
        Some(hash64(&["entry", &identity, &parent_path, folder_name]))
    } else {
        None
    };

    // Add file/directory entries (no header needed - parent entry serves as the landmark)
    for file in &files {
        let entry_id = hash64(&["entry", &identity, &dir_path, &file.name]);

        // New landmark scheme: directories end with /, files don't
        let content_url = if file.is_directory {
            format!("nextcloud://{}/{}/", identity, file.path.trim_matches('/'))
        } else {
            format!("nextcloud://{}/{}", identity, file.path.trim_matches('/'))
        };

        entries.push(Entry {
            name: file.name.clone(),
            path: file.path.clone(),
            size: file.size,
            is_dir: file.is_directory,
            entry_id,
            content_id: 0, // Not used anymore
            content_url,
        });
    }

    // Handle empty directory
    if entries.is_empty() {
        let empty_id = hash64(&["empty", &identity, &dir_path]);
        let msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(empty)");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // West edge back to parent
        let west = if is_root {
            files_portal_hash(&identity)
        } else {
            self_entry_id.unwrap_or(0)
        };
        let edges = encode_set_edges(action_id, empty_id, west, 0, 0, 0, 0, 0, 0);
        write.send(Message::Binary(edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Set parent's east edge to empty placeholder
        if is_root {
            let portal_edges = encode_set_edges(action_id, files_portal_id, EDGE_UNCHANGED, empty_id, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, 0);
            write.send(Message::Binary(portal_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        } else if let Some(parent_id) = self_entry_id {
            let parent_edges = encode_set_edges(action_id, parent_id, EDGE_UNCHANGED, empty_id, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, 0);
            write.send(Message::Binary(parent_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        return Ok(());
    }

    // Set parent's east edge to first entry
    let first_entry_id = entries[0].entry_id;
    if is_root {
        // Root: set files portal east edge
        let portal_edges = encode_set_edges(action_id, files_portal_id, EDGE_UNCHANGED, first_entry_id, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, 0);
        write.send(Message::Binary(portal_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    } else if let Some(parent_id) = self_entry_id {
        // Subdirectory: set parent entry's east edge to our first entry
        let parent_edges = encode_set_edges(action_id, parent_id, EDGE_UNCHANGED, first_entry_id, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, EDGE_UNCHANGED, 0);
        write.send(Message::Binary(parent_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    // Send entries
    for (i, e) in entries.iter().enumerate() {
        // Entry label on layer 0
        let label = if e.is_dir {
            format!("📁 {}", e.name)
        } else {
            format!("📄 {} ({})", e.name, format_size(e.size))
        };
        let label_msg = encode_set_vertex_label(action_id, e.entry_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // For directories/files, add gradesta-url on layer 1 for navigation
        let url_msg = encode_set_vertex_label_layer(action_id, e.entry_id, 1, "text/gradesta-url", e.content_url.as_bytes());
        write.send(Message::Binary(url_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // For files, track entry for click handling (full content loaded on click via layer 2)
        if !e.is_dir {
            let mut s = state.lock().await;
            s.file_entries.insert(e.entry_id, e.path.clone());
        }

        // Entry edges
        // First entry: west goes back to parent (files portal or parent entry)
        let west = if i == 0 {
            if is_root {
                files_portal_hash(&identity)
            } else {
                self_entry_id.unwrap_or(0)
            }
        } else {
            0
        };
        // East edge will be set when subdirectory loads (no east edge initially)
        let east = 0;
        let north = if i > 0 { entries[i - 1].entry_id } else { 0 };
        let south = if i < entries.len() - 1 { entries[i + 1].entry_id } else { 0 };

        let edges = encode_set_edges(action_id, e.entry_id, west, east, north, south, 0, 0, 0);
        write.send(Message::Binary(edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    log::info!("Sent {} entries for {}", entries.len(), dir_path);
    Ok(())
}

/// Handle file view landmark - fetches thumbnail and prepares file for viewing
/// This is called when the browser preloads a file landmark (navigating near a file entry)
pub async fn handle_file_view<W>(
    state: &Arc<Mutex<State>>,
    write: &mut W,
    action_id: u64,
    path: &str,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (identity, nc) = {
        let s = state.lock().await;
        (
            s.identity.clone().unwrap_or_default(),
            s.nextcloud.clone(),
        )
    };

    let nc = nc.ok_or_else(|| anyhow::anyhow!("No Nextcloud client"))?;

    let file_path = format!("/{}", path.trim_matches('/'));

    // Send context - file landmark (no trailing /)
    let landmark = format!("nextcloud://{}{}", identity, file_path);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calculate the entry_id for this file (same as used in directory listing)
    // This is the vertex that appears in the file browser
    let parent_dir = {
        let trimmed = file_path.trim_end_matches('/');
        match trimmed.rsplit_once('/') {
            Some((parent, _)) if parent.is_empty() => "/".to_string(),
            Some((parent, _)) => parent.to_string(),
            None => "/".to_string(),
        }
    };
    let file_name = file_path.rsplit('/').next().unwrap_or(&file_path);
    let entry_id = hash64(&["entry", &identity, &parent_dir, file_name]);

    // Fetch and send thumbnail to layer 2 of the entry vertex
    // Check cache first
    let cached = {
        let s = state.lock().await;
        s.thumbnail_cache.get(&file_path).cloned()
    };

    let thumb_result = if let Some((data, mime)) = cached {
        log::debug!("Using cached thumbnail for {}", file_path);
        Some((data, mime))
    } else {
        // Fetch thumbnail (256x256 is a good size for previews)
        match nc.get_thumbnail(&file_path, 256, 256).await {
            Ok(Some((thumb_data, thumb_mime))) => {
                log::debug!("Got thumbnail for {} ({} bytes)", file_path, thumb_data.len());
                // Cache it
                {
                    let mut s = state.lock().await;
                    s.thumbnail_cache.insert(file_path.clone(), (thumb_data.clone(), thumb_mime.clone()));
                }
                Some((thumb_data, thumb_mime))
            }
            Ok(None) => {
                log::debug!("No thumbnail available for {}", file_path);
                None
            }
            Err(err) => {
                log::debug!("Failed to get thumbnail for {}: {}", file_path, err);
                None
            }
        }
    };

    if let Some((thumb_data, thumb_mime)) = thumb_result {
        // Send thumbnail on layer 2 of the file entry (the vertex in the directory listing)
        let thumb_msg = encode_set_vertex_label_layer(action_id, entry_id, 2, &thumb_mime, &thumb_data);
        write.send(Message::Binary(thumb_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
