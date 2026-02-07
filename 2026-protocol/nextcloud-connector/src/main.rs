//! Nextcloud Connector - stores notes, calendar, and files on Nextcloud via WebDAV/CalDAV

mod calendar;
mod files;
mod identity;
mod nextcloud;
mod notes;
mod protocol;
mod router;
mod storage;

use anyhow::{anyhow, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use crate::identity::PendingAuth;
use crate::nextcloud::NextcloudClient;
use crate::notes::{mime_to_extension, uuid_to_hash, NotesIndex};
use crate::protocol::*;
use crate::storage::{Credential, CredentialStore};

/// Gradesta Nextcloud Connector - stores notes and calendar on Nextcloud via WebDAV/CalDAV
#[derive(Parser, Debug)]
#[command(name = "nextcloud-connector")]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8083)]
    port: u16,

    /// Address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: String,
}

/// Connection states
#[derive(Debug, Clone, PartialEq)]
enum ConnectionState {
    AwaitingIdentity,
    AwaitingAuth,
    Browsing,
}

/// Per-connection state
struct State {
    identity: Option<String>,
    nextcloud: Option<NextcloudClient>,
    state: ConnectionState,
    pending_auth: Option<PendingAuth>,
    poll_endpoint: Option<String>,
    poll_token: Option<String>,
    index: Option<NotesIndex>,
    /// Server-generated action IDs (count UP from 1)
    next_action_id: u64,
    /// Mapping from file entry vertex IDs to file paths (for click handling)
    file_entries: HashMap<u64, String>,
    /// Thumbnail cache: path -> (data, mime_type)
    thumbnail_cache: HashMap<String, (Vec<u8>, String)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            identity: None,
            nextcloud: None,
            state: ConnectionState::AwaitingIdentity,
            pending_auth: None,
            poll_endpoint: None,
            poll_token: None,
            index: None,
            next_action_id: 1,
            file_entries: HashMap::new(),
            thumbnail_cache: HashMap::new(),
        }
    }
}

impl State {
    /// Get the next server-generated action ID
    fn get_next_action_id(&mut self) -> u64 {
        let id = self.next_action_id;
        self.next_action_id = self.next_action_id.wrapping_add(1);
        id
    }
}

impl calendar::HasIdentity for State {
    fn get_identity(&self) -> String {
        self.identity.clone().unwrap_or_default()
    }

    fn get_nextcloud(&self) -> Option<NextcloudClient> {
        self.nextcloud.clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let addr = format!("{}:{}", args.bind, args.port);
    let listener = TcpListener::bind(&addr).await?;

    println!("Gradesta Nextcloud Connector v{}", env!("CARGO_PKG_VERSION"));
    println!("Listening on {}", addr);
    println!("Connect with: ws://localhost:{}/ws", args.port);
    println!();
    println!("Waiting for connections...");

    // Load credential store
    let cred_store = Arc::new(Mutex::new(
        CredentialStore::load().unwrap_or_default(),
    ));

    while let Ok((stream, addr)) = listener.accept().await {
        log::info!("New connection from {}", addr);
        let cred_store = Arc::clone(&cred_store);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, cred_store).await {
                log::error!("Connection error: {}", e);
            }
        });
    }

    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    cred_store: Arc<Mutex<CredentialStore>>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    let state = Arc::new(Mutex::new(State::default()));

    // Send identification request with server-generated action_id
    let action_id = {
        let mut s = state.lock().await;
        s.get_next_action_id()
    };
    let pending = PendingAuth::new(action_id);
    let msg = pending.encode_request("Nextcloud connector needs to verify your identity");
    write.send(Message::Binary(msg)).await?;

    {
        let mut s = state.lock().await;
        s.pending_auth = Some(pending);
    }

    log::info!("Sent identification request");

    // Message handling loop
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Binary(data) = msg {
            if data.is_empty() {
                continue;
            }

            let msg_type = data[0];
            let result = handle_message(
                msg_type,
                &data,
                &state,
                &cred_store,
                &mut write,
            )
            .await;

            if let Err(e) = result {
                log::error!("Error handling message: {}", e);
                let log_msg = encode_log_message(0, 500, 0, &format!("Error: {}", e));
                let _ = write.send(Message::Binary(log_msg)).await;
            }
        }
    }

    Ok(())
}

async fn handle_message<W>(
    msg_type: u8,
    data: &[u8],
    state: &Arc<Mutex<State>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    match msg_type {
        MSG_CLIENT_IDENTIFICATION_RESPONSE => {
            handle_identification_response(data, state, cred_store, write).await
        }
        MSG_CLIENT_IDENTIFICATION_REFUSED => {
            log::info!("Client refused identification");
            let msg = encode_log_message(0, 403, 0, "Identification required");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            Ok(())
        }
        MSG_CLIENT_WATCH_LANDMARK => {
            let s = state.lock().await;
            if s.state != ConnectionState::Browsing {
                log::info!("Ignoring watch landmark (not authenticated)");
                return Ok(());
            }
            drop(s);
            handle_watch_landmark(data, state, write).await
        }
        MSG_CLIENT_SET_VERTEX_LABEL => {
            handle_set_vertex_label(data, state, write).await
        }
        MSG_CLIENT_SET_EDGES => {
            handle_set_edges(data, state, write).await
        }
        MSG_CLIENT_CREATE_VERTEX => {
            handle_create_vertex(data, state, write).await
        }
        MSG_CLIENT_CLICK_VERTEX => {
            handle_click_vertex(data, state, write).await
        }
        _ => {
            log::warn!("Unknown message type: 0x{:02x}", msg_type);
            Ok(())
        }
    }
}

async fn handle_identification_response<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, identity_url, signature) = parse_identification_response(data)?;

    let pending = {
        let mut s = state.lock().await;
        s.pending_auth.take()
    };

    let pending = pending.ok_or_else(|| anyhow!("No pending auth"))?;

    // Verify identity
    let verified = identity::verify_identification(&pending, action_id, &identity_url, &signature).await?;

    log::info!("Verified identity: {} ({})", verified.identity_url, verified.embedded_claim);

    // Use the identity URL as the canonical identity for storage
    let identity = verified.identity_url.clone();

    // Check for stored credentials
    let cred = {
        let store = cred_store.lock().await;
        store.get(&identity).cloned()
    };

    if let Some(cred) = cred {
        log::info!("Found stored credentials for {}", identity);

        let nc = NextcloudClient::new(&cred.nextcloud_url, &cred.username, &cred.app_password);

        // Load notes index
        let index = NotesIndex::load(&nc).await?;

        // Get a server-generated action_id for the router
        let action_id = {
            let mut s = state.lock().await;
            s.identity = Some(identity.clone());
            s.nextcloud = Some(nc);
            s.index = Some(index);
            s.state = ConnectionState::Browsing;
            s.get_next_action_id()
        };

        // Send router (entry point with notes and calendar branches)
        router::send_router(&identity, write, action_id).await?;
    } else {
        log::info!("No credentials for {}, starting auth flow", identity);

        // Extract Nextcloud URL from embedded claim (username@server)
        let parts: Vec<&str> = verified.embedded_claim.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid embedded claim format: {}", verified.embedded_claim));
        }
        let nc_url = format!("https://{}", parts[1]);

        // Initiate login flow
        let login_flow = nextcloud::initiate_login_flow(&nc_url).await?;
        log::info!("Login flow initiated: {}", login_flow.login);

        {
            let mut s = state.lock().await;
            s.identity = Some(identity.clone());
            s.state = ConnectionState::AwaitingAuth;
            s.poll_endpoint = Some(login_flow.poll.endpoint);
            s.poll_token = Some(login_flow.poll.token);
        }

        // Send auth context
        let landmark = format!("nextcloud://{}/auth", identity);
        let ctx_msg = encode_set_context(0, &landmark);
        write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send login URL
        let url_vertex_id = hash_string(&format!("auth:{}", identity));
        let label_msg = encode_set_vertex_label(0, url_vertex_id, "text/x-url", login_flow.login.as_bytes());
        write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send instruction
        let instr_id = hash_string(&format!("instr:{}", identity));
        let instruction = format!(
            "Click the link below to authorize this server to access your Nextcloud notes.\n\nIdentity: {}",
            identity
        );
        let instr_msg = encode_set_vertex_label(0, instr_id, "text/plain", instruction.as_bytes());
        write.send(Message::Binary(instr_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send edges
        let edges1 = encode_set_edges(0, instr_id, 0, 0, 0, url_vertex_id, 0, 0, 0);
        let edges2 = encode_set_edges(0, url_vertex_id, 0, 0, instr_id, 0, 0, 0, 0);
        write.send(Message::Binary(edges1)).await.map_err(|e| anyhow!("{:?}", e))?;
        write.send(Message::Binary(edges2)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Start polling in background
        let state_clone = Arc::clone(state);
        let cred_store_clone = Arc::clone(cred_store);
        tokio::spawn(async move {
            poll_for_auth(state_clone, cred_store_clone).await;
        });
    }

    Ok(())
}

async fn poll_for_auth(state: Arc<Mutex<State>>, cred_store: Arc<Mutex<CredentialStore>>) {
    let mut ticker = interval(Duration::from_secs(2));

    loop {
        ticker.tick().await;

        let (endpoint, token, identity) = {
            let s = state.lock().await;
            if s.state != ConnectionState::AwaitingAuth {
                return;
            }
            (
                s.poll_endpoint.clone(),
                s.poll_token.clone(),
                s.identity.clone(),
            )
        };

        let (endpoint, token, identity) = match (endpoint, token, identity) {
            (Some(e), Some(t), Some(i)) => (e, t, i),
            _ => return,
        };

        match nextcloud::poll_login_completion(&endpoint, &token).await {
            Ok(Some(result)) => {
                log::info!("Auth complete for {} (username: {})", identity, result.login_name);

                let nc = NextcloudClient::new(&result.server, &result.login_name, &result.app_password);

                // Store credentials
                {
                    let mut store = cred_store.lock().await;
                    store.set(
                        &identity,
                        Credential {
                            nextcloud_url: result.server.clone(),
                            username: result.login_name.clone(),
                            app_password: result.app_password.clone(),
                        },
                    );
                    if let Err(e) = store.save() {
                        log::error!("Failed to save credentials: {}", e);
                    }
                }

                // Load notes index
                let index = match NotesIndex::load(&nc).await {
                    Ok(idx) => idx,
                    Err(e) => {
                        log::error!("Failed to load notes index: {}", e);
                        return;
                    }
                };

                {
                    let mut s = state.lock().await;
                    s.nextcloud = Some(nc);
                    s.index = Some(index);
                    s.state = ConnectionState::Browsing;
                    s.poll_endpoint = None;
                    s.poll_token = None;
                }

                log::info!("Auth complete, ready to browse notes");
                // Note: We can't send messages from here since we don't have write access
                // The client should send a WatchLandmark to get the notes listing
                return;
            }
            Ok(None) => {
                // Still waiting
            }
            Err(e) => {
                log::error!("Poll error: {}", e);
            }
        }
    }
}

async fn handle_watch_landmark<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, landmark) = parse_watch_landmark(data)?;
    log::info!("Watch landmark: {} (action={})", landmark, action_id);

    let identity = {
        let s = state.lock().await;
        s.identity.clone().unwrap_or_default()
    };

    // Parse landmark URL to route to appropriate handler
    // Format: nextcloud://{identity}/ - router
    // Format: nextcloud://{identity}/notes/ - notes root
    // Format: nextcloud://{identity}/notes/{hash} - specific note
    // Format: nextcloud://{identity}/calendar/ - calendar root
    // Format: nextcloud://{identity}/calendar/{year}/ - year
    // Format: nextcloud://{identity}/calendar/{year}/{month}/ - month
    // Format: nextcloud://{identity}/calendar/{year}/{month}/{day}/ - day
    // Format: nextcloud://{identity}/files/ - files root
    // Format: nextcloud://{identity}/files/{path} - directory or file

    // Extract path after identity
    // The identity itself may contain slashes (e.g., https://server/s/token)
    // So we need to match against the known identity to find where it ends
    let path = if let Some(stripped) = landmark.strip_prefix("nextcloud://") {
        // Try to strip the identity prefix to get the path
        if let Some(after_identity) = stripped.strip_prefix(&identity) {
            after_identity.trim_start_matches('/')
        } else {
            // Identity doesn't match - this shouldn't happen but handle gracefully
            // Try to find common path segments
            if stripped.ends_with("/notes/") || stripped.contains("/notes/") {
                if let Some(idx) = stripped.rfind("/notes/") {
                    &stripped[idx + 1..]
                } else {
                    "notes/"
                }
            } else if stripped.ends_with("/calendar/") || stripped.contains("/calendar/") {
                if let Some(idx) = stripped.rfind("/calendar/") {
                    &stripped[idx + 1..]
                } else {
                    "calendar/"
                }
            } else if stripped.ends_with("/") {
                "" // Router root
            } else {
                stripped
            }
        }
    } else if let Some(stripped) = landmark.strip_prefix("notes://") {
        // Legacy notes:// URLs - treat as notes
        if let Some(idx) = stripped.find('/') {
            let rest = &stripped[idx + 1..];
            if rest.is_empty() {
                "notes/"
            } else {
                // Wrap in notes/ prefix for legacy support
                return handle_notes_landmark(state, write, action_id, rest).await;
            }
        } else {
            "notes/"
        }
    } else if let Some(vertex_hash) = landmark.strip_prefix("vertex/") {
        // Direct vertex request - try to find and load it
        // This is used by the browser when preloading unknown vertices
        log::info!("Direct vertex request: {}", vertex_hash);
        if let Ok(hash) = vertex_hash.parse::<u64>() {
            // Try to find this vertex - could be a note or a calendar vertex
            // First check if it's a note
            let index = {
                let s = state.lock().await;
                s.index.clone()
            };
            if let Some(index) = &index {
                if notes::hash_to_uuid(index, hash).is_some() {
                    // It's a note vertex
                    return handle_notes_landmark(state, write, action_id, vertex_hash).await;
                }
            }
            // If not found in notes, it might be a calendar vertex
            // Calendar vertices are dynamically generated, so we need to figure out what it is
            // For now, just return an empty response - the calendar doesn't support direct vertex loading yet
            log::info!("Vertex {} not found in notes, checking if calendar vertex", hash);
            // Return the calendar root as a fallback so at least something loads
            return calendar::handle_landmark(state, write, action_id, "").await;
        }
        "" // Will fall through to router
    } else {
        ""
    };

    log::info!("Routing path: '{}'", path);

    match path {
        "" => {
            // Router root
            router::send_router(&identity, write, action_id).await
        }
        p if p.starts_with("notes/") || p.starts_with("notes") => {
            let notes_path = p.strip_prefix("notes/").or_else(|| p.strip_prefix("notes")).unwrap_or("");
            handle_notes_landmark(state, write, action_id, notes_path).await
        }
        p if p.starts_with("calendar/") || p.starts_with("calendar") => {
            let calendar_path = p.strip_prefix("calendar/").or_else(|| p.strip_prefix("calendar")).unwrap_or("");
            calendar::handle_landmark(state, write, action_id, calendar_path).await
        }
        p if p.starts_with("files/") || p.starts_with("files") => {
            let files_path = p.strip_prefix("files/").or_else(|| p.strip_prefix("files")).unwrap_or("");
            files::handle_landmark(state, write, action_id, files_path).await
        }
        p if p.starts_with("file/") => {
            let file_path = p.strip_prefix("file/").unwrap_or("");
            files::handle_file_view(state, write, action_id, file_path).await
        }
        _ => {
            log::warn!("Unknown landmark path: {}", path);
            Err(anyhow!("Unknown landmark path: {}", path))
        }
    }
}

/// Handle notes-specific landmarks
async fn handle_notes_landmark<W>(
    state: &Arc<Mutex<State>>,
    write: &mut W,
    action_id: u64,
    notes_path: &str,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Parse vertex hash from path if present
    let start_vertex = if !notes_path.is_empty() {
        // Try to parse as vertex hash
        if let Ok(hash) = notes_path.parse::<u64>() {
            let index = {
                let s = state.lock().await;
                s.index.clone()
            };
            if let Some(index) = index {
                notes::hash_to_uuid(&index, hash)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    log::info!("Notes landmark - start vertex: {:?}", start_vertex);
    send_notes_from_vertex(state, write, action_id, start_vertex).await
}

/// Maximum vertices in a chain before creating a landmark boundary
const MAX_CHAIN_LENGTH: usize = 20;

async fn send_notes_listing<W>(state: &Arc<Mutex<State>>, write: &mut W, action_id: u64) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Default: send from root vertex
    send_notes_from_vertex(state, write, action_id, None).await
}

/// Send notes starting from a specific vertex (or root if None)
/// Only sends vertices within landmark boundaries (forks or every MAX_CHAIN_LENGTH vertices)
async fn send_notes_from_vertex<W>(
    state: &Arc<Mutex<State>>,
    write: &mut W,
    action_id: u64,
    start_vertex: Option<uuid::Uuid>,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (identity, index, nc) = {
        let s = state.lock().await;
        (
            s.identity.clone().unwrap_or_default(),
            s.index.clone(),
            s.nextcloud.clone(),
        )
    };

    let index = index.ok_or_else(|| anyhow!("No index loaded"))?;
    let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;

    // Send context
    let landmark = if let Some(v) = start_vertex {
        format!("nextcloud://{}/notes/{}", identity, uuid_to_hash(v))
    } else {
        format!("nextcloud://{}/notes/", identity)
    };
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send notes portal vertex - allows navigation back to home screen
    let portal_id = router_hash(&identity, "notes-portal");
    let router_id = router_hash(&identity, "main");
    let calendar_portal_id = router_hash(&identity, "calendar-portal");
    let portal_msg = encode_set_vertex_label(action_id, portal_id, "text/plain", b"Notes");
    write.send(Message::Binary(portal_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Get root vertex ID for portal's east edge
    let root_vertex_id = if index.vertices.is_empty() {
        hash_string(&format!("empty:{}", identity))
    } else {
        index.get_root_vertex().map(uuid_to_hash).unwrap_or(0)
    };

    // Portal edges: north to router, south to calendar portal (vertical menu), east to root note
    let portal_edges = encode_set_edges(action_id, portal_id, 0, root_vertex_id, router_id, calendar_portal_id, 0, 0, 0);
    write.send(Message::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

    if index.vertices.is_empty() {
        // Send empty placeholder
        let empty_id = hash_string(&format!("empty:{}", identity));
        let msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(no notes yet - press 'i' to create one)");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Edges: west to portal, full editability
        let edges = encode_set_edges(action_id, empty_id, portal_id, 0, 0, 0, 0, 0, 0x7F);
        write.send(Message::Binary(edges)).await.map_err(|e| anyhow!("{:?}", e))?;
    } else {
        // Get the starting vertex (specified or root)
        let start = start_vertex.or_else(|| index.get_root_vertex());
        let start = match start {
            Some(v) => v,
            None => {
                log::warn!("No starting vertex found");
                return Ok(());
            }
        };

        // Get the actual root vertex (first by creation time)
        let root_uuid = index.get_root_vertex();

        // Get vertices within landmark boundaries
        let (vertices_to_send, landmark_vertices) = index.get_vertices_within_landmark(start, MAX_CHAIN_LENGTH);

        log::info!(
            "Sending {} vertices (bounded by {} landmarks) from start {:?}",
            vertices_to_send.len(),
            landmark_vertices.len(),
            start
        );

        // Send each vertex
        for vertex_uuid in &vertices_to_send {
            let vertex = match index.get_vertex(*vertex_uuid) {
                Some(v) => v,
                None => continue,
            };
            let vertex_id = uuid_to_hash(vertex.id);
            let is_landmark = landmark_vertices.contains(vertex_uuid);
            let is_root = root_uuid == Some(vertex.id);

            // Load actual content for layer 0
            let (content, mime) = match nc.download(&vertex.file).await {
                Ok(data) => (data, vertex.mime.clone()),
                Err(e) => {
                    log::warn!("Failed to load {}: {}", vertex.file, e);
                    (format!("(failed to load: {})", e).into_bytes(), "text/plain".to_string())
                }
            };

            // Send vertex label (layer 0 - actual content)
            let label_msg = encode_set_vertex_label(action_id, vertex_id, &mime, &content);
            write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            // Send transcript as layer 1 if available
            if let Some(ref transcript) = vertex.transcript {
                let transcript_msg = encode_set_vertex_label_layer(
                    action_id, vertex_id, 1, "text/plain", transcript.as_bytes());
                write.send(Message::Binary(transcript_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }

            // Build edges - for landmark boundaries, replace edges going "outside"
            // with portal URLs that the client can follow to load more
            let mut edge_array = index.build_edge_array(vertex.id);

            // Root vertex connects west to the notes portal
            if is_root && edge_array[0] == 0 {
                edge_array[0] = portal_id;
            }

            if is_landmark {
                // For landmark vertices, edges to unloaded vertices become portals
                // The client will see the vertex hash but won't have the data,
                // causing it to request a WatchLandmark for that vertex
                // We keep the hash but the client knows to request more data
                log::info!("Vertex {} is a landmark boundary", vertex_id);
            }

            // Send edges with full editability
            let edges_msg = encode_set_edges(
                action_id,
                vertex_id,
                edge_array[0],
                edge_array[1],
                edge_array[2],
                edge_array[3],
                edge_array[4],
                edge_array[5],
                0x7F, // All edges and label editable
            );
            write.send(Message::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    }

    log::info!("Sent {} notes (action={})", index.vertices.len(), action_id);
    Ok(())
}

async fn handle_set_vertex_label<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id, layer, mime, content) = parse_client_set_vertex_label(data)?;
    log::info!("SetVertexLabel: action={}, vertex={}, layer={}, mime={}", action_id, vertex_id, layer, mime);

    let (nc, mut index) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, vertex_id, "Not authenticated");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find vertex by hash
    if let Some(uuid) = notes::hash_to_uuid(&index, vertex_id) {
        if layer == 0 {
            // Layer 0: Update primary content
            let vertex = match index.get_vertex(uuid) {
                Some(v) => v,
                None => {
                    let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
                    write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    return Ok(());
                }
            };

            // Upload new content
            if let Err(e) = nc.upload(&vertex.file, &content).await {
                let msg = encode_log_message(action_id, 500, vertex_id, &format!("Upload failed: {}", e));
                write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                return Ok(());
            }
        } else if layer == 1 {
            // Layer 1: Transcript - store as text file and update index
            let transcript = String::from_utf8_lossy(&content).to_string();
            log::info!("Received transcript for vertex {}: {}", vertex_id, transcript);

            // Store transcript in index
            if let Err(e) = index.update_vertex(uuid, Some(transcript.clone())) {
                log::warn!("Failed to update transcript: {}", e);
            }

            // Optionally store transcript to separate file
            let transcript_file = format!("{}/{}_transcript.txt", notes::CONTENT_DIR, uuid);
            if let Err(e) = nc.upload(&transcript_file, content.as_slice()).await {
                log::warn!("Failed to upload transcript file: {}", e);
            }
        } else {
            // Other layers: store to layer-specific file
            let layer_file = format!("{}/{}_layer{}.{}", notes::CONTENT_DIR, uuid, layer,
                notes::mime_to_extension(&mime));
            if let Err(e) = nc.upload(&layer_file, &content).await {
                let msg = encode_log_message(action_id, 500, vertex_id, &format!("Upload failed: {}", e));
                write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                return Ok(());
            }
            if let Err(e) = index.set_vertex_layer(uuid, layer, &mime, &layer_file) {
                log::warn!("Failed to set layer: {}", e);
            }
        }

        // Save index
        if let Err(e) = index.save(&nc).await {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Update state
        {
            let mut s = state.lock().await;
            s.index = Some(index);
        }

        log::info!("Updated vertex {}", uuid);

        // Send success acknowledgment
        let msg = encode_log_message(action_id, 200, vertex_id, "OK");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    } else {
        log::warn!("Vertex not found: {}", vertex_id);
        let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    Ok(())
}

async fn handle_set_edges<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id, edges) = parse_client_set_edges(data)?;
    log::info!("SetEdges: action={}, vertex={}", action_id, vertex_id);

    let (nc, mut index) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, vertex_id, "Not authenticated");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find source vertex
    let from_uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Source vertex not found");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Update edges
    let directions = [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Up,
        Direction::Down,
    ];

    for (i, &target_hash) in edges.iter().enumerate() {
        if target_hash != 0 {
            if let Some(to_uuid) = notes::hash_to_uuid(&index, target_hash) {
                index.add_edge(from_uuid, to_uuid, directions[i]);
            }
        }
    }

    // Save index
    if let Err(e) = index.save(&nc).await {
        let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Update state
    {
        let mut s = state.lock().await;
        s.index = Some(index);
    }

    log::info!("Updated edges for vertex");

    // Send success acknowledgment
    let msg = encode_log_message(action_id, 200, vertex_id, "OK");
    write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    Ok(())
}

async fn handle_create_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, from_vertex, direction, layer, mime, content) = parse_client_create_vertex(data)?;
    log::info!(
        "CreateVertex: action={}, from={}, direction={:?}, layer={}, mime={}",
        action_id,
        from_vertex,
        direction,
        layer,
        mime
    );

    let (nc, mut index, identity) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone(), s.identity.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, 0, "No index loaded");
            write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let _identity = identity.unwrap_or_default();

    // Generate UUID and file path
    let id = uuid::Uuid::new_v4();
    let ext = mime_to_extension(&mime);
    let file_path = format!("{}/{}.{}", notes::CONTENT_DIR, id, ext);

    // Upload content
    if let Err(e) = nc.upload(&file_path, &content).await {
        let msg = encode_log_message(action_id, 500, 0, &format!("Upload failed: {}", e));
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Extract transcript for audio
    let transcript = if mime == "audio/ogg" {
        // For now, transcript should be provided separately or extracted
        // The browser will do Whisper transcription and include it
        None
    } else {
        None
    };

    // Create vertex
    let new_id = index.create_vertex(&mime, &file_path, transcript);

    // Insert vertex into chain (if source provided), maintaining connectivity
    // If source already has an edge in this direction, the new vertex is inserted between them
    let displaced_vertex: Option<uuid::Uuid> = if from_vertex != 0 {
        if let Some(from_uuid) = notes::hash_to_uuid(&index, from_vertex) {
            index.insert_vertex(from_uuid, new_id, direction)
        } else {
            None
        }
    } else {
        None
    };

    // Save index
    if let Err(e) = index.save(&nc).await {
        let msg = encode_log_message(action_id, 500, 0, &format!("Save failed: {}", e));
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Update state
    {
        let mut s = state.lock().await;
        s.index = Some(index.clone());
    }

    // Don't send SetVertexLabel for newly created vertices - the client already has the data.
    // Just send edges and acknowledgment.
    let vertex_hash = uuid_to_hash(new_id);

    // Send new vertex edges
    let edge_array = index.build_edge_array(new_id);
    let edges_msg = encode_set_edges(
        action_id,
        vertex_hash,
        edge_array[0],
        edge_array[1],
        edge_array[2],
        edge_array[3],
        edge_array[4],
        edge_array[5],
        0x7F,
    );
    write.send(Message::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Also send updated edges for source vertex (it now points to new vertex)
    log::info!("CreateVertex: from_vertex={}, direction={:?}, new_vertex_hash={}", from_vertex, direction, vertex_hash);
    if from_vertex != 0 {
        if let Some(from_uuid) = notes::hash_to_uuid(&index, from_vertex) {
            // Source is a real vertex in the index - send its updated edges
            let from_edge_array = index.build_edge_array(from_uuid);
            log::info!("Sending updated edges for source vertex {} (uuid={}) -> {:?}", from_vertex, from_uuid, from_edge_array);
            let from_edges_msg = encode_set_edges(
                action_id,
                from_vertex,
                from_edge_array[0],
                from_edge_array[1],
                from_edge_array[2],
                from_edge_array[3],
                from_edge_array[4],
                from_edge_array[5],
                0x7F,
            );
            write.send(Message::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        } else {
            // Source vertex not in index (e.g., the "empty placeholder")
            // Send synthetic edges update so browser can navigate to new vertex
            log::info!("Source vertex {} not in index - sending synthetic edge update pointing to new vertex", from_vertex);
            let dir_idx = match direction {
                Direction::West => 0,
                Direction::East => 1,
                Direction::North => 2,
                Direction::South => 3,
                Direction::Up => 4,
                Direction::Down => 5,
            };
            let mut edges = [0u64; 6];
            edges[dir_idx] = vertex_hash;
            let from_edges_msg = encode_set_edges(
                action_id,
                from_vertex,
                edges[0], edges[1], edges[2], edges[3], edges[4], edges[5],
                0x7F,
            );
            write.send(Message::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    }

    // If we displaced a vertex, send its updated edges too (it now points back to new vertex)
    if let Some(displaced_uuid) = displaced_vertex {
        let displaced_hash = uuid_to_hash(displaced_uuid);
        let displaced_edge_array = index.build_edge_array(displaced_uuid);
        log::info!("Sending updated edges for displaced vertex {} -> {:?}", displaced_hash, displaced_edge_array);
        let displaced_edges_msg = encode_set_edges(
            action_id,
            displaced_hash,
            displaced_edge_array[0],
            displaced_edge_array[1],
            displaced_edge_array[2],
            displaced_edge_array[3],
            displaced_edge_array[4],
            displaced_edge_array[5],
            0x7F,
        );
        write.send(Message::Binary(displaced_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    // Send success acknowledgment
    let msg = encode_log_message(action_id, 200, vertex_hash, "Created");
    write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    log::info!("Created vertex {} at {}", new_id, file_path);
    Ok(())
}

/// Max file size to fetch content (10 MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Handle click on a vertex (toggle, action, etc.)
async fn handle_click_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<State>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id) = parse_click_vertex(data)?;
    log::info!("ClickVertex: action={}, vertex={}", action_id, vertex_id);

    // Check if this is a file entry click
    let (file_path, nc) = {
        let s = state.lock().await;
        (
            s.file_entries.get(&vertex_id).cloned(),
            s.nextcloud.clone(),
        )
    };

    if let Some(path) = file_path {
        let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;

        log::info!("Loading full content for file: {}", path);

        // Fetch full file content
        match nc.download_with_type(&path, MAX_FILE_SIZE).await {
            Ok((content, mime_type)) => {
                log::info!("Loaded full content for {} ({} bytes, {})", path, content.len(), mime_type);
                // Send full content on layer 2, replacing thumbnail
                let msg = encode_set_vertex_label_layer(action_id, vertex_id, 2, &mime_type, &content);
                write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
            Err(err) => {
                log::error!("Failed to load {}: {}", path, err);
                let msg = encode_log_message(action_id, 500, vertex_id, &format!("Failed to load: {}", err));
                write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
        }
    } else {
        // Not a file entry - just acknowledge
        let msg = encode_log_message(action_id, 200, vertex_id, "Clicked");
        write.send(Message::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    Ok(())
}

/// Hash a string to u64
fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Generate hash for router vertices (matches router.rs)
fn router_hash(identity: &str, name: &str) -> u64 {
    hash_string(&format!("router:{}:{}", identity, name))
}
