//! Router - Entry point that branches to Notes, Calendar, and Files

use anyhow::Result;
use chrono::Local;
use chrono::Datelike;
use futures_util::SinkExt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use axum::extract::ws::Message;

use crate::protocol::*;

/// Generate a deterministic hash for router vertices
fn router_hash(identity: &str, name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("router:{}:{}", identity, name).hash(&mut hasher);
    hasher.finish()
}

/// Generate hash for calendar year (matches calendar.rs)
fn calendar_year_hash(identity: &str, year: i32) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("calendar:{}:year:{}", identity, year).hash(&mut hasher);
    hasher.finish()
}

/// Generate hash for files root directory
fn files_root_hash(identity: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("files:{}:path:", identity).hash(&mut hasher);
    hasher.finish()
}

/// Send the router vertex with Notes, Calendar, and Files portals
pub async fn send_router<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Send context for the router landmark
    let landmark = format!("nextcloud://{}/", identity);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Router vertex IDs
    let router_id = router_hash(identity, "main");
    let notes_portal_id = router_hash(identity, "notes-portal");
    let calendar_portal_id = router_hash(identity, "calendar-portal");
    let files_portal_id = router_hash(identity, "files-portal");

    // Router main vertex - the center point
    let router_label = format!("Nextcloud: {}", identity.split('/').last().unwrap_or(identity));
    let router_msg = encode_set_vertex_label(action_id, router_id, "text/plain", router_label.as_bytes());
    write.send(Message::Binary(router_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Router edges: south → notes portal (notes is the top of the vertical menu)
    let router_edges = encode_set_edges(action_id, router_id, 0, 0, 0, notes_portal_id, 0, 0, 0);
    write.send(Message::Binary(router_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Notes portal - a link to the notes landmark
    let notes_url = format!("nextcloud://{}/notes/", identity);
    let notes_msg = encode_set_vertex_label(action_id, notes_portal_id, "text/gradesta-url", notes_url.as_bytes());
    write.send(Message::Binary(notes_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Notes portal edges: north → router, south → calendar portal
    // East edge will be set by notes landmark when navigated to
    let notes_edges = encode_set_edges(action_id, notes_portal_id, 0, 0, router_id, calendar_portal_id, 0, 0, 0);
    write.send(Message::Binary(notes_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calendar portal - a link to the calendar landmark
    let calendar_url = format!("nextcloud://{}/calendar/", identity);
    let calendar_msg = encode_set_vertex_label(action_id, calendar_portal_id, "text/gradesta-url", calendar_url.as_bytes());
    write.send(Message::Binary(calendar_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calendar portal edges: north → notes portal, south → files portal, east → first year
    let current_year = Local::now().year();
    let first_year = current_year - 2; // Calendar shows ±2 years, first is current-2
    let first_year_id = calendar_year_hash(identity, first_year);
    let calendar_edges = encode_set_edges(action_id, calendar_portal_id, 0, first_year_id, notes_portal_id, files_portal_id, 0, 0, 0);
    write.send(Message::Binary(calendar_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Files portal - show "Files" label, with URL on layer 1 for navigation
    let files_msg = encode_set_vertex_label(action_id, files_portal_id, "text/plain", b"Files");
    write.send(Message::Binary(files_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let files_url = format!("nextcloud://{}/files/", identity);
    let files_url_msg = encode_set_vertex_label_layer(action_id, files_portal_id, 1, "text/gradesta-url", files_url.as_bytes());
    write.send(Message::Binary(files_url_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Files portal edges: north → calendar portal
    // East edge will be set by files landmark when navigated to
    let files_edges = encode_set_edges(action_id, files_portal_id, 0, 0, calendar_portal_id, 0, 0, 0, 0);
    write.send(Message::Binary(files_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    log::info!("Sent router with notes, calendar, and files portals (action={})", action_id);
    Ok(())
}
