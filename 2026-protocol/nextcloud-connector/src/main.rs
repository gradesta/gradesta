//! Nextcloud Connector - stores notes, calendar, and files on Nextcloud via WebDAV/CalDAV

mod calendar;
mod elf;
mod files;
mod http_stream;
mod identity;
mod nextcloud;
mod notes;
mod protocol;
mod router;
mod storage;

use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message as AxumWsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use crate::elf::{ElfConnection, ElfRegistry, SharedElfRegistry};
use crate::http_stream::{JwtSecret, StreamState};
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
    /// Waiting for first message to determine connection type
    AwaitingFirstMessage,
    AwaitingIdentity,
    AwaitingAuth,
    Browsing,
    /// Elf connection mode
    Elf,
}

/// Per-connection state
struct ConnState {
    identity: Option<String>,
    nextcloud: Option<NextcloudClient>,
    conn_state: ConnectionState,
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
    /// JWT secret for generating streaming tokens
    jwt_secret: Arc<JwtSecret>,
    /// Server port for generating streaming URLs
    server_port: u16,
    /// Unique connection ID for this session
    conn_id: u64,
    /// Elf connection state (only set when conn_state == Elf)
    elf_connection: Option<ElfConnection>,
}

impl Default for ConnState {
    fn default() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CONN_COUNTER: AtomicU64 = AtomicU64::new(1);

        Self {
            identity: None,
            nextcloud: None,
            conn_state: ConnectionState::AwaitingFirstMessage,
            pending_auth: None,
            poll_endpoint: None,
            poll_token: None,
            index: None,
            next_action_id: 1,
            file_entries: HashMap::new(),
            thumbnail_cache: HashMap::new(),
            jwt_secret: Arc::new(JwtSecret::default()),
            server_port: 8083,
            conn_id: CONN_COUNTER.fetch_add(1, Ordering::SeqCst),
            elf_connection: None,
        }
    }
}

impl ConnState {
    /// Get the next server-generated action ID
    fn get_next_action_id(&mut self) -> u64 {
        let id = self.next_action_id;
        self.next_action_id = self.next_action_id.wrapping_add(1);
        id
    }
}

impl calendar::HasIdentity for ConnState {
    fn get_identity(&self) -> String {
        self.identity.clone().unwrap_or_default()
    }

    fn get_nextcloud(&self) -> Option<NextcloudClient> {
        self.nextcloud.clone()
    }
}

/// Shared application state - contains both WebSocket and HTTP streaming state
#[derive(Clone)]
struct AppState {
    cred_store: Arc<Mutex<CredentialStore>>,
    jwt_secret: Arc<JwtSecret>,
    port: u16,
    elf_registry: SharedElfRegistry,
}

// Allow extracting StreamState from AppState for the streaming endpoints
impl axum::extract::FromRef<AppState> for StreamState {
    fn from_ref(state: &AppState) -> Self {
        StreamState {
            jwt_secret: Arc::clone(&state.jwt_secret),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    let addr = format!("{}:{}", args.bind, args.port);

    println!("Gradesta Nextcloud Connector v{}", env!("CARGO_PKG_VERSION"));
    println!("Listening on {}", addr);
    println!("Connect with: ws://localhost:{}/ws", args.port);
    println!("HTTP streaming: http://localhost:{}/stream/<token>", args.port);
    println!();
    println!("Waiting for connections...");

    // Load credential store
    let cred_store = Arc::new(Mutex::new(
        CredentialStore::load().unwrap_or_default(),
    ));

    // Generate JWT secret for this server instance
    let jwt_secret = Arc::new(JwtSecret::default());

    let app_state = AppState {
        cred_store,
        jwt_secret,
        port: args.port,
        elf_registry: elf::new_shared_registry(),
    };

    // Build router with both WebSocket and HTTP streaming endpoints
    // StreamState is extracted from AppState via FromRef trait
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/stream/:token", get(http_stream::handle_stream))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app_state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, app_state: AppState) {
    log::info!("New WebSocket connection");
    if let Err(e) = handle_connection_axum(
        socket,
        app_state.cred_store,
        app_state.jwt_secret,
        app_state.port,
        app_state.elf_registry,
    ).await {
        log::error!("Connection error: {}", e);
    }
}

async fn handle_connection_axum(
    socket: WebSocket,
    cred_store: Arc<Mutex<CredentialStore>>,
    jwt_secret: Arc<JwtSecret>,
    port: u16,
    elf_registry: SharedElfRegistry,
) -> Result<()> {
    let (mut write, mut read) = socket.split();

    let state = Arc::new(Mutex::new(ConnState {
        jwt_secret: jwt_secret.clone(),
        server_port: port,
        ..ConnState::default()
    }));

    // Wait for first message to determine connection type
    // If it's ELF_CONNECT, treat this as an elf connection
    // Otherwise, proceed with normal browser flow
    let first_msg = match read.next().await {
        Some(Ok(AxumWsMessage::Binary(data))) if !data.is_empty() => data,
        Some(Ok(_)) => {
            // Non-binary or empty message - treat as browser
            vec![]
        }
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()), // Connection closed immediately
    };

    if !first_msg.is_empty() && first_msg[0] == MSG_ELF_CONNECT {
        // This is an elf connection
        log::info!("Elf connection detected");
        return handle_elf_connection(first_msg, state, elf_registry, &mut write, &mut read).await;
    }

    // Normal browser connection flow
    {
        let mut s = state.lock().await;
        s.conn_state = ConnectionState::AwaitingIdentity;
    }

    // Send identification request with server-generated action_id
    let action_id = {
        let mut s = state.lock().await;
        s.get_next_action_id()
    };
    let pending = PendingAuth::new(action_id);
    let msg = pending.encode_request("Nextcloud connector needs to verify your identity");
    write.send(AxumWsMessage::Binary(msg)).await?;

    {
        let mut s = state.lock().await;
        s.pending_auth = Some(pending);
    }

    log::info!("Sent identification request");

    // Process the first message if it wasn't an elf connect
    if !first_msg.is_empty() {
        let msg_type = first_msg[0];
        let result = handle_message(
            msg_type,
            &first_msg,
            &state,
            &cred_store,
            &elf_registry,
            port,
            &mut write,
        )
        .await;

        if let Err(e) = result {
            log::error!("Error handling first message: {}", e);
            let log_msg = encode_log_message(0, 500, 0, &format!("Error: {}", e));
            let _ = write.send(AxumWsMessage::Binary(log_msg)).await;
        }
    }

    // Message handling loop
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let AxumWsMessage::Binary(data) = msg {
            if data.is_empty() {
                continue;
            }

            let msg_type = data[0];
            let result = handle_message(
                msg_type,
                &data,
                &state,
                &cred_store,
                &elf_registry,
                port,
                &mut write,
            )
            .await;

            if let Err(e) = result {
                log::error!("Error handling message: {}", e);
                let log_msg = encode_log_message(0, 500, 0, &format!("Error: {}", e));
                let _ = write.send(AxumWsMessage::Binary(log_msg)).await;
            }
        }
    }

    Ok(())
}

/// Handle an elf connection
async fn handle_elf_connection<W, R>(
    first_msg: Vec<u8>,
    state: Arc<Mutex<ConnState>>,
    elf_registry: SharedElfRegistry,
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

    log::info!("Elf invitation validated: command={}, elf_url={}",
        invitation.command, invitation.elf_url);

    // Set up elf connection state
    {
        let mut s = state.lock().await;
        s.conn_state = ConnectionState::Elf;
        s.elf_connection = Some(ElfConnection::new(invitation.clone(), browser_conn_id));
    }

    // Send ELF_TASK to the elf
    let task_msg = encode_elf_task(
        &invitation.command,
        &invitation.region.origin_landmark,
        invitation.region.origin_vertex,
        &invitation.region,
        &invitation.params,
    );
    write.send(AxumWsMessage::Binary(task_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    log::info!("Sent ELF_TASK to elf");

    // Handle elf messages
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

                    // TODO: Forward to browser via a shared channel or stored connection
                    // For now, just log the output
                    if output_type == 0 {
                        // Text output
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
                    // Elf is trying to set a vertex label
                    let s = state.lock().await;
                    if let Some(elf_conn) = &s.elf_connection {
                        if !elf_conn.can_write() {
                            log::warn!("Elf attempted write without permission");
                            let err_msg = encode_log_message(0, 403, 0, "Write permission denied");
                            write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                            continue;
                        }
                    }
                    drop(s);
                    // TODO: Validate vertex is in allowed region and forward to appropriate handler
                    log::info!("Elf SetVertexLabel - forwarding to handler");
                }
                MSG_CLIENT_CREATE_VERTEX => {
                    let s = state.lock().await;
                    if let Some(elf_conn) = &s.elf_connection {
                        if !elf_conn.can_create() {
                            log::warn!("Elf attempted create without permission");
                            let err_msg = encode_log_message(0, 403, 0, "Create permission denied");
                            write.send(AxumWsMessage::Binary(err_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                            continue;
                        }
                    }
                    drop(s);
                    log::info!("Elf CreateVertex - forwarding to handler");
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

async fn handle_message<W>(
    msg_type: u8,
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    elf_registry: &SharedElfRegistry,
    server_port: u16,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    match msg_type {
        MSG_CLIENT_IDENTIFICATION_RESPONSE => {
            handle_identification_response(data, state, cred_store, write).await
        }
        MSG_CLIENT_IDENTIFICATION_REFUSED => {
            log::info!("Client refused identification");
            let msg = encode_log_message(0, 403, 0, "Identification required");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            Ok(())
        }
        MSG_CLIENT_WATCH_LANDMARK => {
            let s = state.lock().await;
            if s.conn_state != ConnectionState::Browsing {
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
        MSG_CLIENT_DELETE_VERTEX => {
            handle_delete_vertex(data, state, write).await
        }
        MSG_CLIENT_CLICK_VERTEX => {
            handle_click_vertex(data, state, write).await
        }
        MSG_CLIENT_INTRODUCE_ELF => {
            handle_introduce_elf(data, state, elf_registry, server_port, write).await
        }
        _ => {
            log::warn!("Unknown message type: 0x{:02x}", msg_type);
            Ok(())
        }
    }
}

/// Handle INTRODUCE_ELF message from browser
async fn handle_introduce_elf<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    elf_registry: &SharedElfRegistry,
    server_port: u16,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, elf_url, command, cursor_landmark, cursor_vertex, region, permissions, params) =
        parse_introduce_elf(data)?;

    log::info!("IntroduceElf: action={} elf_url={} command={} region={:?}",
        action_id, elf_url, command, region.origin_landmark);

    // Verify the user is authenticated
    let (identity, conn_id) = {
        let s = state.lock().await;
        if s.conn_state != ConnectionState::Browsing {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
        (s.identity.clone().unwrap_or_default(), s.conn_id)
    };

    // Create invitation in registry
    let token = {
        let mut registry = elf_registry.lock().await;
        registry.create_invitation(
            identity,
            elf_url.clone(),
            region,
            permissions,
            command.clone(),
            params,
            conn_id,
            300, // 5 minute TTL
        )
    };

    log::info!("Created elf invitation with token: {}...", &token[..std::cmp::min(8, token.len())]);

    // Build WebSocket URL for elf to connect to
    // Use ws://localhost for now - in production this would be configurable
    let server_ws_url = format!("ws://localhost:{}/ws", server_port);

    // Send IntroductionToken back to browser
    let token_msg = encode_introduction_token(action_id, &token, &server_ws_url);
    write.send(AxumWsMessage::Binary(token_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    log::info!("Sent IntroductionToken to browser");
    Ok(())
}

async fn handle_identification_response<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
            s.conn_state = ConnectionState::Browsing;
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
            s.conn_state = ConnectionState::AwaitingAuth;
            s.poll_endpoint = Some(login_flow.poll.endpoint);
            s.poll_token = Some(login_flow.poll.token);
        }

        // Send auth context
        let landmark = format!("nextcloud://{}/auth", identity);
        let ctx_msg = encode_set_context(0, &landmark);
        write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send login URL
        let url_vertex_id = hash_string(&format!("auth:{}", identity));
        let label_msg = encode_set_vertex_label(0, url_vertex_id, "text/x-url", login_flow.login.as_bytes());
        write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send instruction
        let instr_id = hash_string(&format!("instr:{}", identity));
        let instruction = format!(
            "Click the link below to authorize this server to access your Nextcloud notes.\n\nIdentity: {}",
            identity
        );
        let instr_msg = encode_set_vertex_label(0, instr_id, "text/plain", instruction.as_bytes());
        write.send(AxumWsMessage::Binary(instr_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send edges
        let edges1 = encode_set_edges(0, instr_id, 0, 0, 0, url_vertex_id, 0, 0, 0);
        let edges2 = encode_set_edges(0, url_vertex_id, 0, 0, instr_id, 0, 0, 0, 0);
        write.send(AxumWsMessage::Binary(edges1)).await.map_err(|e| anyhow!("{:?}", e))?;
        write.send(AxumWsMessage::Binary(edges2)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Start polling in background
        let state_clone = Arc::clone(state);
        let cred_store_clone = Arc::clone(cred_store);
        tokio::spawn(async move {
            poll_for_auth(state_clone, cred_store_clone).await;
        });
    }

    Ok(())
}

async fn poll_for_auth(state: Arc<Mutex<ConnState>>, cred_store: Arc<Mutex<CredentialStore>>) {
    let mut ticker = interval(Duration::from_secs(2));

    loop {
        ticker.tick().await;

        let (endpoint, token, identity) = {
            let s = state.lock().await;
            if s.conn_state != ConnectionState::AwaitingAuth {
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
                    s.conn_state = ConnectionState::Browsing;
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
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
            // Legacy: files/ prefix routes to directory listing
            let files_path = p.strip_prefix("files/").or_else(|| p.strip_prefix("files")).unwrap_or("");
            files::handle_landmark(state, write, action_id, files_path).await
        }
        p if p.starts_with("file/") => {
            // Legacy: file/ prefix routes to file view
            let file_path = p.strip_prefix("file/").unwrap_or("");
            files::handle_file_view(state, write, action_id, file_path).await
        }
        p if p.ends_with('/') => {
            // New scheme: paths ending in / are directories
            // Strip leading and trailing / to get the directory path
            let dir_path = p.trim_matches('/');
            files::handle_landmark(state, write, action_id, dir_path).await
        }
        p if !p.is_empty() => {
            // New scheme: paths not ending in / are files (but must not be empty)
            files::handle_file_view(state, write, action_id, p).await
        }
        _ => {
            log::warn!("Unknown landmark path: {}", path);
            Err(anyhow!("Unknown landmark path: {}", path))
        }
    }
}

/// Handle notes-specific landmarks
async fn handle_notes_landmark<W>(
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
    action_id: u64,
    notes_path: &str,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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

async fn send_notes_listing<W>(state: &Arc<Mutex<ConnState>>, write: &mut W, action_id: u64) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Default: send from root vertex
    send_notes_from_vertex(state, write, action_id, None).await
}

/// Send notes starting from a specific vertex (or root if None)
/// Only sends vertices within landmark boundaries (forks or every MAX_CHAIN_LENGTH vertices)
async fn send_notes_from_vertex<W>(
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
    action_id: u64,
    start_vertex: Option<uuid::Uuid>,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
    write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send notes portal vertex - allows navigation back to home screen
    let portal_id = router_hash(&identity, "notes-portal");
    let router_id = router_hash(&identity, "main");
    let calendar_portal_id = router_hash(&identity, "calendar-portal");
    let portal_msg = encode_set_vertex_label(action_id, portal_id, "text/plain", b"Notes");
    write.send(AxumWsMessage::Binary(portal_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Get root vertex ID for portal's east edge
    let root_vertex_id = if index.vertices.is_empty() {
        hash_string(&format!("empty:{}", identity))
    } else {
        index.get_root_vertex().map(uuid_to_hash).unwrap_or(0)
    };

    // Portal edges: north to router, south to calendar portal (vertical menu), east to root note
    let portal_edges = encode_set_edges(action_id, portal_id, 0, root_vertex_id, router_id, calendar_portal_id, 0, 0, 0);
    write.send(AxumWsMessage::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

    if index.vertices.is_empty() {
        // Send empty placeholder
        let empty_id = hash_string(&format!("empty:{}", identity));
        let msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(no notes yet - press 'i' to create one)");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Edges: west to portal, full editability
        let edges = encode_set_edges(action_id, empty_id, portal_id, 0, 0, 0, 0, 0, 0x7F);
        write.send(AxumWsMessage::Binary(edges)).await.map_err(|e| anyhow!("{:?}", e))?;
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
            write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            // Send transcript as layer 1 if available
            if let Some(ref transcript) = vertex.transcript {
                let transcript_msg = encode_set_vertex_label_layer(
                    action_id, vertex_id, 1, "text/plain", transcript.as_bytes());
                write.send(AxumWsMessage::Binary(transcript_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
            write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    }

    log::info!("Sent {} notes (action={})", index.vertices.len(), action_id);
    Ok(())
}

async fn handle_set_vertex_label<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    return Ok(());
                }
            };

            // Upload new content
            if let Err(e) = nc.upload(&vertex.file, &content).await {
                let msg = encode_log_message(action_id, 500, vertex_id, &format!("Upload failed: {}", e));
                write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
                write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                return Ok(());
            }
            if let Err(e) = index.set_vertex_layer(uuid, layer, &mime, &layer_file) {
                log::warn!("Failed to set layer: {}", e);
            }
        }

        // Save index
        if let Err(e) = index.save(&nc).await {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    } else {
        log::warn!("Vertex not found: {}", vertex_id);
        let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    Ok(())
}

async fn handle_set_edges<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find source vertex
    let from_uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Source vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Update edges
    // EDGE_UNCHANGED (u64::MAX) means keep existing, 0 means remove, other values set the edge
    let directions = [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Up,
        Direction::Down,
    ];

    for (i, &target_hash) in edges.iter().enumerate() {
        if target_hash == u64::MAX {
            // EDGE_UNCHANGED - skip this edge
            continue;
        } else if target_hash == 0 {
            // Remove edge in this direction
            index.remove_edge(from_uuid, directions[i]);
        } else {
            // Set edge to target vertex
            if let Some(to_uuid) = notes::hash_to_uuid(&index, target_hash) {
                index.add_edge(from_uuid, to_uuid, directions[i]);
            }
        }
    }

    // Save index
    if let Err(e) = index.save(&nc).await {
        let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    Ok(())
}

async fn handle_create_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
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
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, 0, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
    write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

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
            write.send(AxumWsMessage::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
            write.send(AxumWsMessage::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
        write.send(AxumWsMessage::Binary(displaced_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    // Send success acknowledgment
    let msg = encode_log_message(action_id, 200, vertex_hash, "Created");
    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    log::info!("Created vertex {} at {}", new_id, file_path);
    Ok(())
}

async fn handle_delete_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id) = parse_client_delete_vertex(data)?;
    log::info!("DeleteVertex: action={}, vertex={}", action_id, vertex_id);

    let (nc, mut index, _identity) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone(), s.identity.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, 0, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find vertex by hash
    let vertex_uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Delete vertex and get files to delete + affected neighbors
    let (files_to_delete, affected_neighbors) = match index.delete_vertex(vertex_uuid) {
        Ok(result) => result,
        Err(e) => {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Delete failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Delete content files from Nextcloud
    for file_path in &files_to_delete {
        if let Err(e) = nc.delete(file_path).await {
            log::warn!("Failed to delete file {}: {}", file_path, e);
            // Continue anyway - the index is the source of truth
        }
    }

    // Save updated index
    if let Err(e) = index.save(&nc).await {
        let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Update state
    {
        let mut s = state.lock().await;
        s.index = Some(index.clone());
    }

    // Send updated edges for all affected neighbors
    for neighbor_uuid in affected_neighbors {
        let neighbor_hash = uuid_to_hash(neighbor_uuid);
        let edge_array = index.build_edge_array(neighbor_uuid);
        log::info!("Sending updated edges for neighbor {} after delete -> {:?}", neighbor_hash, edge_array);
        let edges_msg = encode_set_edges(
            action_id,
            neighbor_hash,
            edge_array[0],
            edge_array[1],
            edge_array[2],
            edge_array[3],
            edge_array[4],
            edge_array[5],
            0x7F,
        );
        write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    // Send edges update for the deleted vertex with all zeros to signal deletion
    let deleted_edges_msg = encode_set_edges(
        action_id,
        vertex_id,
        0, 0, 0, 0, 0, 0,
        0,  // edit_mask = 0 means read-only (deleted)
    );
    write.send(AxumWsMessage::Binary(deleted_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send success acknowledgment
    let msg = encode_log_message(action_id, 200, vertex_id, "Deleted");
    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    log::info!("Deleted vertex {} (uuid={})", vertex_id, vertex_uuid);
    Ok(())
}

/// Max file size to send via WebSocket (10 MB) - larger files use HTTP streaming
const MAX_WS_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// MIME types that should always use HTTP streaming (videos)
const STREAMING_MIME_TYPES: &[&str] = &[
    "video/mp4",
    "video/webm",
    "video/quicktime",
    "video/x-matroska",
    "video/ogg",
];

/// Handle click on a vertex (toggle, action, etc.)
async fn handle_click_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id) = parse_click_vertex(data)?;
    log::info!("ClickVertex: action={}, vertex={}", action_id, vertex_id);

    // Check if this is a file entry click
    let (file_path, nc, jwt_secret, server_port) = {
        let s = state.lock().await;
        (
            s.file_entries.get(&vertex_id).cloned(),
            s.nextcloud.clone(),
            Arc::clone(&s.jwt_secret),
            s.server_port,
        )
    };

    if let Some(path) = file_path {
        let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;

        log::info!("Loading full content for file: {}", path);

        // Get file info to decide whether to use WebSocket or HTTP streaming
        let (file_size, mime_type) = nc.get_file_info(&path).await?;
        let is_video = STREAMING_MIME_TYPES.iter().any(|&m| mime_type.starts_with(m));
        let use_http_streaming = file_size > MAX_WS_FILE_SIZE || is_video;

        if use_http_streaming {
            // Generate JWT token and send HTTP streaming URL on layer 3
            log::info!("Using HTTP streaming for {} ({} bytes, {})", path, file_size, mime_type);

            let token = http_stream::generate_token(&jwt_secret, &path, &mime_type, &nc)?;
            let stream_url = format!("http://localhost:{}/stream/{}", server_port, token);

            // Format: expected-mime\nurl
            let layer3_content = format!("{}\n{}", mime_type, stream_url);
            let msg = encode_set_vertex_label_layer(
                action_id,
                vertex_id,
                3,
                "text/x-http-stream-url",
                layer3_content.as_bytes(),
            );
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            log::info!("Sent HTTP streaming URL for {} on layer 3", path);
        } else {
            // Small file - fetch full content via WebSocket
            match nc.download_with_type(&path, MAX_WS_FILE_SIZE).await {
                Ok((content, mime_type)) => {
                    log::info!("Loaded full content for {} ({} bytes, {})", path, content.len(), mime_type);
                    // Send full content on layer 2, replacing thumbnail
                    let msg = encode_set_vertex_label_layer(action_id, vertex_id, 2, &mime_type, &content);
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                }
                Err(err) => {
                    log::error!("Failed to load {}: {}", path, err);
                    let msg = encode_log_message(action_id, 500, vertex_id, &format!("Failed to load: {}", err));
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                }
            }
        }
    } else {
        // Not a file entry - just acknowledge
        let msg = encode_log_message(action_id, 200, vertex_id, "Clicked");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
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
