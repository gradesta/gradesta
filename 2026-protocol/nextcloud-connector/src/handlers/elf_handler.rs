//! ELF connection handler

use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{anyhow, Result};
use axum::extract::ws::Message as AxumWsMessage;
use futures_util::{SinkExt, StreamExt};

use crate::connection_manager::SharedConnectionManager;
use crate::elf::{ElfConnection, SharedElfRegistry};
use crate::nextcloud::NextcloudClient;
use crate::notes::{self, NotesIndex};
use crate::protocol::*;
use crate::state::{ConnState, ConnectionState};
use crate::storage::CredentialStore;

use super::landmark::forward_vertex_update_to_browser;
use super::vertex::{handle_create_vertex, handle_delete_vertex, handle_set_vertex_label};
use super::click::handle_click_vertex;

/// Handle an elf connection
pub async fn handle_elf_connection<W, R>(
    first_msg: Vec<u8>,
    state: Arc<Mutex<ConnState>>,
    elf_registry: SharedElfRegistry,
    connection_manager: SharedConnectionManager,
    cred_store: Arc<Mutex<CredentialStore>>,
    write: &mut W,
    read: &mut R,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
    R: StreamExt<Item = Result<AxumWsMessage, axum::Error>> + Unpin,
{
    // Parse the elf connect message
    let token = parse_elf_connect(&first_msg)?;
    log::info!("Elf connecting with token: {}...", &token[..std::cmp::min(8, token.len())]);

    // Validate and consume the invitation
    let (invitation, browser_conn_id) = {
        let mut registry = elf_registry.lock().await;
        let browser_conn_id = registry.get_browser_connection(&token)
            .ok_or_else(|| anyhow!("No browser connection for token"))?;
        let invitation = registry.consume_invitation(&token)
            .ok_or_else(|| anyhow!("Invalid or expired invitation token"))?;
        (invitation, browser_conn_id)
    };

    log::info!("Elf invitation validated: command={}, elf_url={}, summoner={}",
        invitation.command, invitation.elf_url, invitation.summoner_identity);

    // Look up summoner's credentials and set up NextcloudClient
    let cred = {
        let store = cred_store.lock().await;
        store.get(&invitation.summoner_identity).cloned()
    };

    let (nc, index) = if let Some(cred) = cred {
        let nc = NextcloudClient::new(&cred.nextcloud_url, &cred.username, &cred.app_password);
        let index = NotesIndex::load(&nc).await?;
        (Some(nc), Some(index))
    } else {
        log::warn!("No credentials found for summoner: {}", invitation.summoner_identity);
        (None, None)
    };

    // Load cursor vertex content before setting up connection state
    let cursor_vertex = invitation.region.origin_vertex;
    let (cursor_mime, cursor_content) = if let (Some(ref nc_client), Some(ref idx)) = (&nc, &index) {
        // Try to find and load cursor vertex content
        if let Some(uuid) = notes::hash_to_uuid(idx, cursor_vertex) {
            if let Some(vertex) = idx.get_vertex(uuid) {
                log::info!("Loading cursor vertex content: uuid={}, file={}", uuid, vertex.file);
                match nc_client.download(&vertex.file).await {
                    Ok(content) => {
                        log::info!("Loaded cursor content: {} bytes, mime={}", content.len(), vertex.mime);
                        (vertex.mime.clone(), content)
                    }
                    Err(e) => {
                        log::warn!("Failed to load cursor content: {}", e);
                        ("text/plain".to_string(), Vec::new())
                    }
                }
            } else {
                log::warn!("Cursor vertex {} not found in index", cursor_vertex);
                ("text/plain".to_string(), Vec::new())
            }
        } else {
            log::warn!("Cursor vertex hash {} not in notes index", cursor_vertex);
            ("text/plain".to_string(), Vec::new())
        }
    } else {
        log::warn!("No nextcloud client or index for cursor content");
        ("text/plain".to_string(), Vec::new())
    };

    // Set up elf connection state
    {
        let mut s = state.lock().await;
        s.conn_state = ConnectionState::Elf;
        s.identity = Some(invitation.summoner_identity.clone());
        s.nextcloud = nc;
        s.index = index;
        s.elf_connection = Some(ElfConnection::new(invitation.clone(), browser_conn_id));
    }

    // Send ELF_TASK to the elf with cursor content included
    let task_msg = encode_elf_task(
        &invitation.command,
        &invitation.region.origin_landmark,
        cursor_vertex,
        &cursor_mime,
        &cursor_content,
        &invitation.region,
        &invitation.params,
    );
    write.send(AxumWsMessage::Binary(task_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    log::info!("Sent ELF_TASK to elf with {} bytes of cursor content", cursor_content.len());

    // Handle elf messages - reuse the same handlers as browser but with permission checks
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let AxumWsMessage::Binary(data) = msg {
            if data.is_empty() {
                continue;
            }

            let msg_type = data[0];
            match msg_type {
                MSG_ELF_OUTPUT => {
                    let (output_type, output_data) = parse_elf_output(&data)?;
                    log::info!("Elf output: type={}, {} bytes", output_type, output_data.len());

                    // Forward output to browser
                    let fwd_msg = encode_elf_output_fwd(0, output_type, &output_data);
                    let cm = connection_manager.lock().await;
                    if cm.send_to(browser_conn_id, fwd_msg).is_ok() {
                        log::info!("Forwarded elf output to browser");
                    }

                    if output_type == 0 {
                        if let Ok(text) = String::from_utf8(output_data.clone()) {
                            log::info!("Elf text: {}", text);
                        }
                    }
                }
                MSG_ELF_COMPLETE => {
                    let (status, message) = parse_elf_complete(&data)?;
                    log::info!("Elf complete: status={}, message={}", status, message);
                    break;
                }
                MSG_CLIENT_SET_VERTEX_LABEL => {
                    // Check write permission
                    {
                        let s = state.lock().await;
                        if let Some(elf_conn) = &s.elf_connection {
                            if !elf_conn.can_write() {
                                log::warn!("Elf attempted write without permission");
                                let err_msg = encode_log_message(0, 403, 0, "Write permission denied");
                                write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                                continue;
                            }
                        }
                    }
                    // Use existing handler
                    if let Err(e) = handle_set_vertex_label(&data, &state, write).await {
                        log::error!("Elf SetVertexLabel failed: {}", e);
                    } else {
                        // Forward the update to the browser and all watchers
                        forward_vertex_update_to_browser(&state, &connection_manager, &data).await;
                    }
                }
                MSG_CLIENT_CLICK_VERTEX => {
                    // Check read permission
                    {
                        let s = state.lock().await;
                        if let Some(elf_conn) = &s.elf_connection {
                            if !elf_conn.can_read() {
                                log::warn!("Elf attempted read without permission");
                                let err_msg = encode_log_message(0, 403, 0, "Read permission denied");
                                write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                                continue;
                            }
                        }
                    }
                    // Use existing handler
                    if let Err(e) = handle_click_vertex(&data, &state, write).await {
                        log::error!("Elf ClickVertex failed: {}", e);
                    }
                }
                MSG_CLIENT_CREATE_VERTEX => {
                    // Check create permission
                    {
                        let s = state.lock().await;
                        if let Some(elf_conn) = &s.elf_connection {
                            if !elf_conn.can_create() {
                                log::warn!("Elf attempted create without permission");
                                let err_msg = encode_log_message(0, 403, 0, "Create permission denied");
                                write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                                continue;
                            }
                        }
                    }
                    // Use existing handler
                    if let Err(e) = handle_create_vertex(&data, &state, write).await {
                        log::error!("Elf CreateVertex failed: {}", e);
                    } else {
                        // Forward the update to the browser and all watchers
                        forward_vertex_update_to_browser(&state, &connection_manager, &data).await;
                    }
                }
                MSG_CLIENT_DELETE_VERTEX => {
                    // Check delete permission
                    {
                        let s = state.lock().await;
                        if let Some(elf_conn) = &s.elf_connection {
                            if !elf_conn.can_delete() {
                                log::warn!("Elf attempted delete without permission");
                                let err_msg = encode_log_message(0, 403, 0, "Delete permission denied");
                                write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                                continue;
                            }
                        }
                    }
                    // Use existing handler
                    if let Err(e) = handle_delete_vertex(&data, &state, write).await {
                        log::error!("Elf DeleteVertex failed: {}", e);
                    } else {
                        // Forward the update to the browser and all watchers
                        forward_vertex_update_to_browser(&state, &connection_manager, &data).await;
                    }
                }
                _ => {
                    log::warn!("Unknown elf message type: 0x{:02x}", msg_type);
                }
            }
        }
    }

    // Clean up
    {
        let mut registry = elf_registry.lock().await;
        registry.remove_browser_connection(&token);
    }

    log::info!("Elf connection closed");
    Ok(())
}
