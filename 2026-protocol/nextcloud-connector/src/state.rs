//! Core state types for the Nextcloud Connector

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use clap::Parser;

use crate::calendar;
use crate::connection_manager::SharedConnectionManager;
use crate::elf::{ElfConnection, SharedElfRegistry};
use crate::git_undo;
use crate::http_stream::{JwtSecret, StreamState};
use crate::identity::PendingAuth;
use crate::local_storage;
use crate::nextcloud::NextcloudClient;
use crate::notes::NotesIndex;
use crate::storage::CredentialStore;
use crate::sync_worker;
use crate::webdav_mount;

/// Gradesta Nextcloud Connector - stores notes and calendar on Nextcloud via WebDAV/CalDAV
#[derive(Parser, Debug, Clone)]
#[command(name = "nextcloud-connector")]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8083)]
    pub port: u16,

    /// Address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Run in local/offline mode (no Nextcloud auth, file-based storage)
    /// Provide the path to use for local storage
    #[arg(short, long)]
    pub local: Option<String>,
}

/// Connection states
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Waiting for first message to determine connection type
    AwaitingFirstMessage,
    AwaitingIdentity,
    AwaitingAuth,
    Browsing,
    /// Elf connection mode
    Elf,
}

/// Per-connection state
pub struct ConnState {
    pub identity: Option<String>,
    pub nextcloud: Option<NextcloudClient>,
    /// Local storage client for offline mode
    pub local_storage: Option<local_storage::LocalStorageClient>,
    pub conn_state: ConnectionState,
    pub pending_auth: Option<PendingAuth>,
    pub poll_endpoint: Option<String>,
    pub poll_token: Option<String>,
    pub index: Option<NotesIndex>,
    /// Shared index for sync worker access
    pub shared_index: Option<Arc<Mutex<NotesIndex>>>,
    /// Server-generated action IDs (count UP from 1)
    pub next_action_id: u64,
    /// Mapping from file entry vertex IDs to file paths (for click handling)
    pub file_entries: HashMap<u64, String>,
    /// Thumbnail cache: path -> (data, mime_type)
    pub thumbnail_cache: HashMap<String, (Vec<u8>, String)>,
    /// JWT secret for generating streaming tokens
    pub jwt_secret: Arc<JwtSecret>,
    /// Server port for generating streaming URLs
    pub server_port: u16,
    /// Unique connection ID for this session
    pub conn_id: u64,
    /// Elf connection state (only set when conn_state == Elf)
    pub elf_connection: Option<ElfConnection>,
    /// Git-based undo repo for this session (for notes directory)
    pub git_undo_repo: Option<std::sync::Arc<tokio::sync::Mutex<git_undo::GitUndoRepo>>>,
    /// Sender for queuing background sync work items
    pub sync_tx: Option<tokio::sync::mpsc::Sender<sync_worker::SyncWorkItem>>,
    /// WebDAV mount for efficient git push (dropped on disconnect)
    pub webdav_mount: Option<webdav_mount::WebDavMount>,
}

impl Default for ConnState {
    fn default() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static CONN_COUNTER: AtomicU64 = AtomicU64::new(1);

        Self {
            identity: None,
            nextcloud: None,
            local_storage: None,
            conn_state: ConnectionState::AwaitingFirstMessage,
            pending_auth: None,
            poll_endpoint: None,
            poll_token: None,
            index: None,
            shared_index: None,
            next_action_id: 1,
            file_entries: HashMap::new(),
            thumbnail_cache: HashMap::new(),
            jwt_secret: Arc::new(JwtSecret::default()),
            server_port: 8083,
            conn_id: CONN_COUNTER.fetch_add(1, Ordering::SeqCst),
            elf_connection: None,
            git_undo_repo: None,
            sync_tx: None,
            webdav_mount: None,
        }
    }
}

impl ConnState {
    /// Get the next server-generated action ID
    pub fn get_next_action_id(&mut self) -> u64 {
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
pub struct AppState {
    pub cred_store: Arc<Mutex<CredentialStore>>,
    pub jwt_secret: Arc<JwtSecret>,
    pub port: u16,
    pub elf_registry: SharedElfRegistry,
    /// Connection manager for forwarding messages between connections
    pub connection_manager: SharedConnectionManager,
    /// If set, run in local mode with file-based storage at this path
    pub local_storage_path: Option<std::path::PathBuf>,
}

// Allow extracting StreamState from AppState for the streaming endpoints
impl axum::extract::FromRef<AppState> for StreamState {
    fn from_ref(state: &AppState) -> Self {
        StreamState {
            jwt_secret: Arc::clone(&state.jwt_secret),
        }
    }
}
