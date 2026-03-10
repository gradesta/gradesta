//! Server startup and WebSocket handling

use std::sync::Arc;
use tokio::sync::Mutex;

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

use crate::connection_manager::new_shared_connection_manager;
use crate::elf;
use crate::handlers::{
    handle_click_vertex, handle_create_vertex, handle_delete_vertex,
    handle_elf_connection, handle_identification_response, handle_introduce_elf,
    handle_set_edges, handle_set_vertex_label, handle_watch_landmark,
};
use crate::http_stream::{self, JwtSecret};
use crate::identity::PendingAuth;
use crate::local_storage;
use crate::notes;
use crate::protocol::*;
use crate::router;
use crate::state::{AppState, Args, ConnState, ConnectionState};
use crate::storage::CredentialStore;

use crate::connection_manager::SharedConnectionManager;
use crate::elf::SharedElfRegistry;
use crate::git_undo::GitUndoRepo;
use crate::migration::Migrator;
use crate::nextcloud::NextcloudClient;

#[tokio::main]
pub async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    // Handle migration mode
    if args.migrate {
        return run_migration(&args).await;
    }

    let addr = format!("{}:{}", args.bind, args.port);

    println!("Gradesta Nextcloud Connector v{}", env!("CARGO_PKG_VERSION"));
    println!("Listening on {}", addr);
    println!("Connect with: ws://localhost:{}/ws", args.port);
    println!("HTTP streaming: http://localhost:{}/stream/<token>", args.port);

    // Handle local mode
    let local_storage_path = if let Some(ref path) = args.local {
        let path = std::path::PathBuf::from(path);
        println!("Running in LOCAL MODE with storage at: {}", path.display());
        println!("  - No authentication required");
        println!("  - Data stored in local filesystem");
        // Create the directory if it doesn't exist
        std::fs::create_dir_all(&path)?;
        Some(path)
    } else {
        None
    };

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
        connection_manager: new_shared_connection_manager(),
        local_storage_path,
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

/// Run migration to convert old file-based repos to CAS format
async fn run_migration(args: &Args) -> Result<()> {
    println!("Gradesta Migration Tool");
    println!("=======================");
    println!();

    // For migration, we need either local mode or Nextcloud credentials
    if let Some(ref local_path) = args.local {
        run_local_migration(local_path, args.cleanup_after_migrate).await
    } else {
        // Need Nextcloud credentials - load from stored credentials
        let cred_store = CredentialStore::load().unwrap_or_default();

        if cred_store.credentials.is_empty() {
            println!("ERROR: No stored Nextcloud credentials found.");
            println!();
            println!("To migrate a Nextcloud-connected repository:");
            println!("  1. First run the connector normally and authenticate");
            println!("  2. Then run: nextcloud-connector --migrate");
            println!();
            println!("Or for local-only migration:");
            println!("  nextcloud-connector --migrate --local /path/to/notes");
            return Err(anyhow!("No credentials available for migration"));
        }

        // Use the first stored credential
        let (_identity, cred) = cred_store.credentials.iter().next().unwrap();
        println!("Using stored credentials for: {}", cred.nextcloud_url);
        println!("Username: {}", cred.username);
        println!();

        run_nextcloud_migration(&cred.nextcloud_url, &cred.username, &cred.app_password, args.cleanup_after_migrate).await
    }
}

/// Run migration for local-only mode
async fn run_local_migration(path: &str, _cleanup: bool) -> Result<()> {
    let path = std::path::PathBuf::from(path);
    let notes_dir = path.join(".gradesta-notes");

    println!("Migrating local repository at: {}", notes_dir.display());

    if !notes_dir.exists() {
        println!("No .gradesta-notes directory found. Nothing to migrate.");
        return Ok(());
    }

    // Open or create git repo
    let git_repo = GitUndoRepo::open_or_init(&notes_dir)?;

    // Check if migration is needed
    let index = notes::NotesIndex::load_from_path(&notes_dir)?;
    if !crate::migration::needs_migration(&index) {
        println!("Repository already using CAS format. No migration needed.");
        return Ok(());
    }

    let savings = crate::migration::estimate_savings(&index);
    println!("Estimated space savings: {} MB", savings / 1024 / 1024);
    println!();

    // For local mode, we create a mock client that uses the filesystem
    println!("Note: Local-only migration will convert index.toml format");
    println!("      but cannot rewrite git history without WebDAV access.");
    println!();

    // Just update the index format
    let mut index = index;
    for vertex in &mut index.vertices {
        if !vertex.file.is_empty() && vertex.content_hash.is_empty() {
            // Read the file, compute hash, and update
            let file_path = notes_dir.join(&vertex.file);
            if file_path.exists() {
                let content = std::fs::read(&file_path)?;
                let hash = crate::content_store::compute_hash(&content);
                let ext = notes::mime_to_extension(&vertex.mime);

                // Write to content-store
                let store = crate::content_store::LocalContentStore::new(&notes_dir);
                store.put(&content, ext)?;

                println!("  Migrated: {} -> {}", vertex.file, hash);
                vertex.content_hash = hash;
                vertex.file.clear();
            }
        }
    }

    // Update version
    index.meta.version = notes::CURRENT_INDEX_VERSION;

    // Save
    index.save_to_path(&notes_dir)?;

    // Commit
    git_repo.ensure_on_branch()?;
    git_repo.commit_all("Migrate to CAS format", "migration")?;

    println!();
    println!("Running git gc to reclaim space...");
    run_git_gc(&notes_dir)?;

    println!();
    println!("Migration complete!");

    Ok(())
}

/// Run git gc to reclaim space after migration
fn run_git_gc(workdir: &std::path::Path) -> Result<()> {
    // Remove reflogs
    let git_dir = workdir.join(".git");
    let reflog_dir = git_dir.join("logs");
    if reflog_dir.exists() {
        let _ = std::fs::remove_dir_all(&reflog_dir);
    }

    // Expire reflog
    let _ = std::process::Command::new("git")
        .args(["reflog", "expire", "--expire=now", "--all"])
        .current_dir(workdir)
        .output();

    // Run git gc
    println!("  Running git gc --aggressive --prune=now...");
    let output = std::process::Command::new("git")
        .args(["gc", "--aggressive", "--prune=now"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("  Git gc completed successfully");
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("  Git gc finished with warnings: {}", stderr);
            }
        }
        Err(e) => {
            println!("  Warning: Failed to run git gc: {}", e);
        }
    }

    Ok(())
}

/// Run migration for Nextcloud-connected repository
async fn run_nextcloud_migration(url: &str, username: &str, password: &str, cleanup: bool) -> Result<()> {
    println!("Connecting to Nextcloud...");

    let nc = NextcloudClient::new(url, username, password);

    // Check if notes exist
    if !nc.exists(".gradesta-notes/index.toml").await {
        println!("No .gradesta-notes found on Nextcloud. Nothing to migrate.");
        return Ok(());
    }

    println!("Loading index...");
    let index = notes::NotesIndex::load(&nc).await?;

    if !crate::migration::needs_migration(&index) {
        println!("Repository already using CAS format. No migration needed.");
        return Ok(());
    }

    let savings = crate::migration::estimate_savings(&index);
    println!("Estimated space savings: {} MB", savings / 1024 / 1024);
    println!();

    // Create local temp directory for git operations
    let temp_dir = std::env::temp_dir().join(format!("gradesta-migrate-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    println!("Using temp directory: {}", temp_dir.display());

    // Open git repo from Nextcloud
    let git_repo = crate::git_undo::open_from_nextcloud(&nc).await?;

    println!("Running migration...");
    let migrator = Migrator::new(&nc, &git_repo);
    let stats = migrator.migrate_full().await?;

    println!();
    println!("Migration complete!");
    println!("  Vertices migrated: {}", stats.vertices_migrated);
    println!("  Layers migrated: {}", stats.layers_migrated);
    println!("  Commits rewritten: {}", stats.commits_rewritten);
    println!("  Estimated bytes saved: {}", stats.bytes_saved_estimate);

    if cleanup && !stats.old_files_to_delete.is_empty() {
        println!();
        println!("Cleaning up {} old content files...", stats.old_files_to_delete.len());
        let deleted = migrator.cleanup_old_files(&stats.old_files_to_delete).await?;
        println!("  Deleted {} files", deleted);
    } else if !stats.old_files_to_delete.is_empty() {
        println!();
        println!("Old content files were not deleted. Run with --cleanup-after-migrate to remove them.");
        println!("  {} files can be deleted", stats.old_files_to_delete.len());
    }

    // Cleanup temp directory
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!();
    println!("Done!");

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
        app_state.connection_manager,
        app_state.local_storage_path,
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
    connection_manager: SharedConnectionManager,
    local_storage_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let (mut write, mut read) = socket.split();

    let state = Arc::new(Mutex::new(ConnState {
        jwt_secret: jwt_secret.clone(),
        server_port: port,
        ..ConnState::default()
    }));

    // Get connection ID for later use
    let conn_id = {
        let s = state.lock().await;
        s.conn_id
    };

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
        return handle_elf_connection(first_msg, state, elf_registry, connection_manager, cred_store, &mut write, &mut read).await;
    }

    // Create channel for receiving forwarded messages from elf handlers
    let (fwd_tx, mut fwd_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    // Register this browser connection with the connection manager
    {
        let mut cm = connection_manager.lock().await;
        cm.register(conn_id, fwd_tx);
    }

    // Check if we're in local mode
    if let Some(ref storage_path) = local_storage_path {
        // Local mode: skip authentication, set up local storage
        log::info!("Local mode: skipping authentication");

        let local_storage = local_storage::LocalStorageClient::new(storage_path);
        let identity = "local://test".to_string();

        // Load or create notes index (empty for local mode)
        let index = notes::NotesIndex::default();

        {
            let mut s = state.lock().await;
            s.identity = Some(identity.clone());
            s.conn_state = ConnectionState::Browsing;
            s.index = Some(index);
            // Store local storage in a new field (we'll need to add this)
            s.local_storage = Some(local_storage);
        }

        // Send router
        let action_id = {
            let mut s = state.lock().await;
            s.get_next_action_id()
        };
        router::send_router(&identity, &mut write, action_id).await?;
    } else {
        // Normal browser connection flow - require authentication
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
    }

    // Process the first message if it wasn't an elf connect
    if !first_msg.is_empty() {
        let msg_type = first_msg[0];
        let result = handle_message(
            msg_type,
            &first_msg,
            &state,
            &cred_store,
            &elf_registry,
            &connection_manager,
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

    // Message handling loop - listen to both WebSocket messages and forwarded messages
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            msg = read.next() => {
                match msg {
                    Some(Ok(AxumWsMessage::Binary(data))) => {
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
                            &connection_manager,
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
                    Some(Ok(_)) => {
                        // Ignore non-binary messages
                        continue;
                    }
                    Some(Err(e)) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // Connection closed
                        break;
                    }
                }
            }
            // Handle forwarded messages from elf handlers
            fwd_msg = fwd_rx.recv() => {
                match fwd_msg {
                    Some(data) => {
                        if let Err(e) = write.send(AxumWsMessage::Binary(data)).await {
                            log::error!("Failed to forward message to browser: {:?}", e);
                            break;
                        }
                    }
                    None => {
                        // Channel closed (shouldn't happen normally)
                        log::warn!("Forward channel closed for conn_id={}", conn_id);
                        break;
                    }
                }
            }
        }
    }

    // Clean up on disconnect
    log::info!("Browser connection {} disconnected, cleaning up", conn_id);
    {
        let mut cm = connection_manager.lock().await;
        cm.unregister(conn_id);
    }
    {
        let mut registry = elf_registry.lock().await;
        registry.cleanup_for_browser(conn_id);
    }

    Ok(())
}

async fn handle_message<W>(
    msg_type: u8,
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    elf_registry: &SharedElfRegistry,
    connection_manager: &SharedConnectionManager,
    _server_port: u16,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    match msg_type {
        MSG_CLIENT_IDENTIFICATION_RESPONSE => {
            handle_identification_response(data, state, cred_store, connection_manager, write).await
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
            handle_watch_landmark(data, state, connection_manager, write).await
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
            handle_introduce_elf(data, state, elf_registry, write).await
        }
        _ => {
            log::warn!("Unknown message type: 0x{:02x}", msg_type);
            Ok(())
        }
    }
}
