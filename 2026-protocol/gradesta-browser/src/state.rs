//! Application state types for the gradesta browser.
//!
//! This module contains the core state types that track UI state, input modes,
//! and pending operations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy::prelude::*;

use crate::identity::IdentityConfig;
use crate::keybindings::{KeybindingResolver, KeybindingsConfig};
use crate::sidebar::{KeybindingsEditorState, SidebarState};

/// Category of debug log entry
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugCategory {
    /// Context changes (Graph, TextInput, etc.)
    Context,
    /// Raw keypresses detected
    Keypress,
    /// Commands that were triggered
    Command,
    /// Command execution results
    Execution,
}

impl DebugCategory {
    /// Get a short label for display
    pub fn label(&self) -> &'static str {
        match self {
            DebugCategory::Context => "CTX",
            DebugCategory::Keypress => "KEY",
            DebugCategory::Command => "CMD",
            DebugCategory::Execution => "EXE",
        }
    }

    /// Get an icon for display
    pub fn icon(&self) -> &'static str {
        match self {
            DebugCategory::Context => "🔄",
            DebugCategory::Keypress => "⌨",
            DebugCategory::Command => "⚡",
            DebugCategory::Execution => "✓",
        }
    }
}

/// A single debug log entry
#[derive(Clone, Debug)]
pub struct DebugLogEntry {
    pub timestamp: Instant,
    pub category: DebugCategory,
    pub message: String,
}

/// Filter state for debug panel
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DebugFilter {
    #[default]
    All,
    Context,
    Keypress,
    Command,
    Execution,
}

impl DebugFilter {
    pub fn matches(&self, category: DebugCategory) -> bool {
        match self {
            DebugFilter::All => true,
            DebugFilter::Context => category == DebugCategory::Context,
            DebugFilter::Keypress => category == DebugCategory::Keypress,
            DebugFilter::Command => category == DebugCategory::Command,
            DebugFilter::Execution => category == DebugCategory::Execution,
        }
    }
}

/// Direction edge constants
pub const EDGE_WEST: usize = 0;
pub const EDGE_EAST: usize = 1;
pub const EDGE_NORTH: usize = 2;
pub const EDGE_SOUTH: usize = 3;
pub const EDGE_UP: usize = 4;
pub const EDGE_DOWN: usize = 5;

/// Key repeat timing constants
pub const KEY_REPEAT_DELAY: std::time::Duration = std::time::Duration::from_millis(400);
pub const KEY_REPEAT_RATE: std::time::Duration = std::time::Duration::from_millis(50);

/// Zoom constants
pub const ZOOM_MIN: f32 = 0.25;
pub const ZOOM_MAX: f32 = 4.0;
pub const ZOOM_STEP: f32 = 0.1;

/// Input mode for the browser
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    /// Editing text; direction indicates where to create new vertex (None = edit current)
    TextInput { direction: Option<usize> },
    /// Recording audio to create new vertex in the given direction
    Recording { direction: usize },
}

/// Pending vertex creation data - waiting for server acknowledgment
pub struct PendingVertexCreation {
    /// Audio samples for transcription (empty for text)
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    /// The data that was sent to the server (to populate vertex locally)
    pub data: Vec<u8>,
    /// MIME type of the data
    pub mime: String,
}

/// Pending identification request from a server
#[derive(Clone, Debug)]
pub struct PendingIdentification {
    pub action_id: u64,
    pub nonce: [u8; 32],
    pub timestamp: u64,
    pub reason: String,
    pub server_url: String,
}

/// State for ongoing Nextcloud login flow
#[derive(Clone, Debug)]
pub struct NextcloudLoginState {
    pub nextcloud_url: String,
    pub poll_endpoint: String,
    pub poll_token: String,
    pub started: Instant,
}

/// State after successful login, waiting for user to choose display name
#[derive(Clone, Debug)]
pub struct PendingIdentitySetup {
    pub nextcloud_url: String,
    pub username: String,
    pub app_password: String,
    pub display_name_input: String,
}

/// Action to take for identification request
pub enum IdentificationAction {
    Identify { remember: bool },
    Refuse,
}

/// Main application state resource
#[derive(Resource)]
pub struct AppState {
    pub server_input: String,
    pub landmark_input: String,
    pub server_bar_has_focus: bool,
    pub landmark_bar_has_focus: bool,
    pub status: String,
    pub connected: bool,
    pub current_vertex: Option<u64>,
    /// For detecting navigation changes (used for auto-play)
    pub last_vertex: Option<u64>,
    pub history: Vec<u64>,
    pub base_ws_url: Option<String>,
    /// Track which landmarks we've already requested
    pub requested_landmarks: HashSet<String>,
    /// If set, we're waiting to jump to this landmark's first vertex
    pub following_portal: Option<String>,
    // Key repeat state
    pub key_repeat_last_move: Option<Instant>,
    pub key_repeat_started: bool,
    // Content modal state
    pub show_text_modal: bool,
    pub text_modal_content: String,
    pub show_image_modal: bool,
    pub image_modal_vertex_id: Option<u64>,
    // Zoom state
    pub zoom_level: f32,
    // Identity state
    pub show_identity_panel: bool,
    pub identity_config: IdentityConfig,
    // Identity consent dialog
    pub pending_identification: Option<PendingIdentification>,
    pub selected_identity_index: usize,
    // Nextcloud login flow state
    pub nextcloud_login_state: Option<NextcloudLoginState>,
    pub pending_identity_setup: Option<PendingIdentitySetup>,
    pub nextcloud_url_input: String,
    // Bag (clipboard stack) for connecting non-adjacent cells
    pub bag: Vec<u64>,
    pub show_bag_panel: bool,
    // Navigation panel - landmark history and islands
    pub show_nav_panel: bool,
    /// Recently visited landmarks (most recent last)
    pub landmark_history: Vec<String>,
    pub max_landmark_history: usize,
    // Input mode state
    pub input_mode: InputMode,
    pub text_input_buffer: String,
    /// Cursor position in text input (byte offset)
    pub text_cursor_pos: usize,
    /// Selection start in text input (byte offset, None if no selection)
    pub text_selection_start: Option<usize>,
    /// Internal clipboard for text editing
    pub text_clipboard: String,
    /// Undo stack for text input
    pub text_undo_stack: Vec<String>,
    /// Redo stack for text input
    pub text_redo_stack: Vec<String>,
    // Audio recording state
    pub audio_samples: Arc<Mutex<Vec<f32>>>,
    pub audio_sample_rate: u32,
    pub recording_start: Option<Instant>,
    /// Action ID counter (counts down from MAX to avoid collision with server IDs)
    pub next_action_id: u64,
    /// Pending vertex creations: maps action_id -> data for populating vertex locally on ack
    pub pending_creations: HashMap<u64, PendingVertexCreation>,
    /// Skip auto-play for this vertex (set after recording to avoid immediate playback)
    pub skip_autoplay_vertex: Option<u64>,
    /// Last navigation direction (used to determine where new vertices are created)
    pub last_nav_direction: usize,
    /// Focus URL bar on next frame (to avoid 'l' being typed when pressing Ctrl+L)
    pub focus_url_bar_next_frame: bool,
    /// True when URL bar currently has focus (from click or Ctrl+L)
    pub url_bar_has_focus: bool,
    // Video player state
    pub show_video_modal: bool,
    pub video_modal_vertex_id: Option<u64>,
    // Sidebar state (for new sidebar-based UI)
    pub sidebar: SidebarState,
    // Keybinding system
    pub keybindings: KeybindingResolver,
    /// Text-to-speech mode - read text cells aloud on navigation
    pub tts_mode: bool,
    // Command bar state
    pub show_command_bar: bool,
    pub command_bar_input: String,
    /// Selected autocomplete suggestion
    pub command_bar_selected: usize,
    // Keybindings editor state
    pub keybindings_editor: KeybindingsEditorState,
    // Debug panel state
    pub show_debug_panel: bool,
    pub debug_log: Vec<DebugLogEntry>,
    pub debug_log_file: Option<PathBuf>,
    pub debug_filter: DebugFilter,
    /// Previous context for detecting changes
    pub debug_last_context: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        // Try to load identity config
        let identity_config = IdentityConfig::load().unwrap_or_default();

        Self {
            server_input: "ws://localhost:8080".to_string(),
            landmark_input: "/".to_string(),
            server_bar_has_focus: false,
            landmark_bar_has_focus: false,
            status: "Enter URL and click Connect".to_string(),
            connected: false,
            current_vertex: None,
            last_vertex: None,
            history: Vec::new(),
            base_ws_url: None,
            requested_landmarks: HashSet::new(),
            following_portal: None,
            key_repeat_last_move: None,
            key_repeat_started: false,
            show_text_modal: false,
            text_modal_content: String::new(),
            show_image_modal: false,
            image_modal_vertex_id: None,
            zoom_level: 1.0,
            show_identity_panel: false,
            identity_config,
            pending_identification: None,
            selected_identity_index: 0,
            nextcloud_login_state: None,
            pending_identity_setup: None,
            nextcloud_url_input: "https://".to_string(),
            bag: Vec::new(),
            show_bag_panel: false,
            show_nav_panel: false,
            landmark_history: Vec::new(),
            max_landmark_history: 20,
            input_mode: InputMode::Normal,
            text_input_buffer: String::new(),
            text_cursor_pos: 0,
            text_selection_start: None,
            text_clipboard: String::new(),
            text_undo_stack: Vec::new(),
            text_redo_stack: Vec::new(),
            audio_samples: Arc::new(Mutex::new(Vec::new())),
            audio_sample_rate: 44100,
            recording_start: None,
            next_action_id: u64::MAX,
            pending_creations: HashMap::new(),
            skip_autoplay_vertex: None,
            last_nav_direction: EDGE_SOUTH, // Default to south
            focus_url_bar_next_frame: false,
            url_bar_has_focus: false,
            show_video_modal: false,
            video_modal_vertex_id: None,
            sidebar: SidebarState::default(),
            keybindings: KeybindingResolver::new(&KeybindingsConfig::load().unwrap_or_default()),
            tts_mode: false,
            show_command_bar: false,
            command_bar_input: String::new(),
            command_bar_selected: 0,
            keybindings_editor: KeybindingsEditorState::default(),
            show_debug_panel: false,
            debug_log: Vec::new(),
            debug_log_file: None,
            debug_filter: DebugFilter::default(),
            debug_last_context: None,
        }
    }
}
