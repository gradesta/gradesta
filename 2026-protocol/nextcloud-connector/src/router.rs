//! Router - Entry point that branches to Notes and Calendar

use anyhow::Result;
use futures_util::SinkExt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::*;

/// Generate a deterministic hash for router vertices
fn router_hash(identity: &str, name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("router:{}:{}", identity, name).hash(&mut hasher);
    hasher.finish()
}

/// Send the router vertex with Notes (west) and Calendar (east) portals
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

    // Router main vertex - the center point
    let router_label = format!("Nextcloud: {}", identity.split('/').last().unwrap_or(identity));
    let router_msg = encode_set_vertex_label(action_id, router_id, "text/plain", router_label.as_bytes());
    write.send(Message::Binary(router_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Router edges: north → notes portal, south → calendar portal (vertical layout)
    let router_edges = encode_set_edges(action_id, router_id, 0, 0, notes_portal_id, calendar_portal_id, 0, 0, 0);
    write.send(Message::Binary(router_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Notes portal - a link to the notes landmark
    let notes_url = format!("nextcloud://{}/notes/", identity);
    let notes_msg = encode_set_vertex_label(action_id, notes_portal_id, "text/gradesta-url", notes_url.as_bytes());
    write.send(Message::Binary(notes_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Notes portal edges: south → router
    let notes_edges = encode_set_edges(action_id, notes_portal_id, 0, 0, 0, router_id, 0, 0, 0);
    write.send(Message::Binary(notes_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calendar portal - a link to the calendar landmark
    let calendar_url = format!("nextcloud://{}/calendar/", identity);
    let calendar_msg = encode_set_vertex_label(action_id, calendar_portal_id, "text/gradesta-url", calendar_url.as_bytes());
    write.send(Message::Binary(calendar_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calendar portal edges: north → router
    let calendar_edges = encode_set_edges(action_id, calendar_portal_id, 0, 0, router_id, 0, 0, 0, 0);
    write.send(Message::Binary(calendar_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    log::info!("Sent router with notes and calendar portals (action={})", action_id);
    Ok(())
}
