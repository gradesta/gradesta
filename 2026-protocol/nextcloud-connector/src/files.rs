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

    // Send context
    let landmark = format!("nextcloud://{}/files{}", identity, dir_path);
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

    // Add ".." entry for non-root directories
    if !is_root {
        let parent_path = {
            let trimmed = dir_path.trim_end_matches('/');
            match trimmed.rsplit_once('/') {
                Some((parent, _)) if parent.is_empty() => "/".to_string(),
                Some((parent, _)) => parent.to_string(),
                None => "/".to_string(),
            }
        };
        let parent_url = format!("nextcloud://{}/files{}", identity, parent_path);
        entries.push(Entry {
            name: "..".to_string(),
            path: parent_path,
            size: 0,
            is_dir: true,
            entry_id: hash64(&["entry", &identity, &dir_path, ".."]),
            content_id: hash64(&["dirurl", &parent_url]),
            content_url: parent_url,
        });
    }

    // Add file/directory entries
    for file in &files {
        let entry_id = hash64(&["entry", &identity, &dir_path, &file.name]);

        let (content_id, content_url) = if file.is_directory {
            let url = format!("nextcloud://{}/files/{}", identity, file.path.trim_matches('/'));
            (hash64(&["dirurl", &url]), url)
        } else {
            let url = format!("nextcloud://{}/file/{}", identity, file.path.trim_matches('/'));
            (hash64(&["fileurl", &url]), url)
        };

        entries.push(Entry {
            name: file.name.clone(),
            path: file.path.clone(),
            size: file.size,
            is_dir: file.is_directory,
            entry_id,
            content_id,
            content_url,
        });
    }

    // Handle empty directory
    if entries.is_empty() {
        let empty_id = hash64(&["empty", &identity, &dir_path]);
        let msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(empty)");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // West edge to files portal
        let west = files_portal_hash(&identity);
        let edges = encode_set_edges(action_id, empty_id, west, 0, 0, 0, 0, 0, 0);
        write.send(Message::Binary(edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Send entries
    for (i, e) in entries.iter().enumerate() {
        // Entry label
        let label = if e.name == ".." {
            "..".to_string()
        } else if e.is_dir {
            format!("📁 {}", e.name)
        } else {
            format!("📄 {} ({})", e.name, format_size(e.size))
        };
        let label_msg = encode_set_vertex_label(action_id, e.entry_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Content portal (gradesta-url for lazy loading)
        let content_msg = encode_set_vertex_label(action_id, e.content_id, "text/gradesta-url", e.content_url.as_bytes());
        write.send(Message::Binary(content_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Entry edges
        let west = if i == 0 {
            // First entry: west goes to files portal (back to menu)
            files_portal_hash(&identity)
        } else {
            0
        };
        let east = e.content_id;
        let north = if i > 0 { entries[i - 1].entry_id } else { 0 };
        let south = if i < entries.len() - 1 { entries[i + 1].entry_id } else { 0 };

        let edges = encode_set_edges(action_id, e.entry_id, west, east, north, south, 0, 0, 0);
        write.send(Message::Binary(edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Content portal edges: west back to entry
        let content_edges = encode_set_edges(action_id, e.content_id, e.entry_id, 0, 0, 0, 0, 0, 0);
        write.send(Message::Binary(content_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    log::info!("Sent {} entries for {}", entries.len(), dir_path);
    Ok(())
}

/// Handle file view landmark - fetches and displays file content
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

    // Send context
    let landmark = format!("nextcloud://{}/file{}", identity, file_path);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // File content vertex
    let content_id = hash64(&["filecontent", &identity, &file_path]);

    // Parent directory for back navigation
    let parent_dir = {
        let trimmed = file_path.trim_end_matches('/');
        match trimmed.rsplit_once('/') {
            Some((parent, _)) if parent.is_empty() => "/".to_string(),
            Some((parent, _)) => parent.to_string(),
            None => "/".to_string(),
        }
    };
    let parent_url = format!("nextcloud://{}/files{}", identity, parent_dir);
    let parent_id = hash64(&["dirurl", &parent_url]);

    // Fetch file content
    match nc.download_with_type(&file_path, MAX_FILE_SIZE).await {
        Ok((content, mime_type)) => {
            log::info!("Fetched file {} ({} bytes, {})", file_path, content.len(), mime_type);
            let msg = encode_set_vertex_label(action_id, content_id, &mime_type, &content);
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
        Err(err) => {
            log::error!("Failed to fetch file {}: {}", file_path, err);
            let msg = encode_set_vertex_label(action_id, content_id, "text/plain", format!("Error: {}", err).as_bytes());
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
    }

    // Content edges: west to parent directory portal
    let edges = encode_set_edges(action_id, content_id, parent_id, 0, 0, 0, 0, 0, 0);
    write.send(Message::Binary(edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Parent directory portal
    let parent_msg = encode_set_vertex_label(action_id, parent_id, "text/gradesta-url", parent_url.as_bytes());
    write.send(Message::Binary(parent_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let parent_edges = encode_set_edges(action_id, parent_id, 0, content_id, 0, 0, 0, 0, 0);
    write.send(Message::Binary(parent_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

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
