//! Application state types for the gradesta browser.
//!
//! This module contains the core state types that track UI state, input modes,
//! and pending operations.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::*;

use crate::export::ExportState;
use crate::identity::IdentityConfig;
use crate::keybindings::{KeybindingResolver, KeybindingsConfig};
use crate::local_services::LocalServices;
use crate::sidebar::{KeybindingsEditorState, SidebarState};
use crate::voice_command::VoiceCommandState;

// ============================================================================
// Elf System Types
// ============================================================================

/// A command exposed by an elf
#[derive(Clone, Debug)]
pub struct ElfCommand {
    /// Command name (identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Input types required
    pub inputs: Vec<ElfInputType>,
}

/// Types of inputs an elf command can accept
#[derive(Clone, Debug)]
pub enum ElfInputType {
    /// Current cursor position
    Cursor,
    /// A region of the graph
    Region,
    /// A text prompt
    Prompt,
}

/// Manifest describing an elf's capabilities
#[derive(Clone, Debug)]
pub struct ElfManifest {
    /// Unique identifier for this elf
    pub elf_id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this elf does
    pub description: String,
    /// Available commands
    pub commands: Vec<ElfCommand>,
}

/// A trusted elf configuration (stored locally)
#[derive(Clone, Debug)]
pub struct TrustedElf {
    /// URL of the elf service
    pub url: String,
    /// Cached manifest (fetched from elf)
    pub manifest: Option<ElfManifest>,
    /// Last time manifest was fetched
    pub last_fetched: Option<Instant>,
    /// Whether the elf is currently reachable
    pub is_reachable: bool,
}

impl TrustedElf {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            manifest: None,
            last_fetched: None,
            is_reachable: false,
        }
    }
}

/// Active elf task state
#[derive(Clone, Debug)]
pub struct ElfTask {
    /// Elf URL
    pub elf_url: String,
    /// Command being executed
    pub command: String,
    /// Whether the task has completed
    pub completed: bool,
    /// Final status (if completed)
    pub status: Option<u32>,
    /// Final message (if completed)
    pub message: Option<String>,
}

impl ElfTask {
    pub fn new(elf_url: &str, command: &str) -> Self {
        Self {
            elf_url: elf_url.to_string(),
            command: command.to_string(),
            completed: false,
            status: None,
            message: None,
        }
    }
}

/// Which section of the elf panel has gamepad focus
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ElfPanelFocus {
    #[default]
    /// Elf list selection
    ElfList,
    /// Command list selection
    CommandList,
    /// Direction toggles (W/E/N/S/U/D)
    Directions,
    /// Permission toggles (Read/Write/Create/Delete)
    Permissions,
    /// Summon button
    Summon,
}

/// State for elf panel UI
#[derive(Clone, Debug, Default)]
pub struct ElfPanelState {
    /// URL input field
    pub url_input: String,
    /// Currently selected elf index
    pub selected_elf_index: usize,
    /// Currently selected command index
    pub selected_command_index: usize,
    /// Region direction checkboxes (bitmask)
    pub selected_directions: u8,
    /// Permission checkboxes (bitmask)
    pub selected_permissions: u8,
    /// Which section has gamepad focus
    pub gamepad_focus: ElfPanelFocus,
    /// Which direction toggle is selected (0-5 for W/E/N/S/U/D)
    pub direction_cursor: usize,
    /// Which permission toggle is selected (0-3 for R/W/C/D)
    pub permission_cursor: usize,
}

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
    /// Focus changes (URL bar, text inputs)
    Focus,
}

impl DebugCategory {
    /// Get a short label for display
    pub fn label(&self) -> &'static str {
        match self {
            DebugCategory::Context => "CTX",
            DebugCategory::Keypress => "KEY",
            DebugCategory::Command => "CMD",
            DebugCategory::Execution => "EXE",
            DebugCategory::Focus => "FOC",
        }
    }

    /// Get an icon for display
    pub fn icon(&self) -> &'static str {
        match self {
            DebugCategory::Context => "🔄",
            DebugCategory::Keypress => "⌨",
            DebugCategory::Command => "⚡",
            DebugCategory::Execution => "✓",
            DebugCategory::Focus => "👁",
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

/// Direction edge constants
pub const EDGE_WEST: usize = 0;
pub const EDGE_EAST: usize = 1;
pub const EDGE_NORTH: usize = 2;
pub const EDGE_SOUTH: usize = 3;
pub const EDGE_UP: usize = 4;
pub const EDGE_DOWN: usize = 5;

/// Direction enum for graph navigation and operations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    West,
    East,
    North,
    South,
    Up,
    Down,
}

impl Direction {
    /// Convert to edge index
    pub fn to_edge_index(self) -> usize {
        match self {
            Direction::West => EDGE_WEST,
            Direction::East => EDGE_EAST,
            Direction::North => EDGE_NORTH,
            Direction::South => EDGE_SOUTH,
            Direction::Up => EDGE_UP,
            Direction::Down => EDGE_DOWN,
        }
    }

    /// Human-readable name
    pub fn name(self) -> &'static str {
        match self {
            Direction::West => "West",
            Direction::East => "East",
            Direction::North => "North",
            Direction::South => "South",
            Direction::Up => "Up",
            Direction::Down => "Down",
        }
    }

    /// Arrow character for display
    pub fn arrow(self) -> &'static str {
        match self {
            Direction::West => "←",
            Direction::East => "→",
            Direction::North => "↑",
            Direction::South => "↓",
            Direction::Up => "⬆",
            Direction::Down => "⬇",
        }
    }

    /// All directions
    pub fn all() -> &'static [Direction] {
        &[
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
            Direction::Up,
            Direction::Down,
        ]
    }

    /// Horizontal/vertical directions (excluding up/down stacks)
    pub fn cardinal() -> &'static [Direction] {
        &[
            Direction::West,
            Direction::East,
            Direction::North,
            Direction::South,
        ]
    }
}

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
    /// Inline editing text directly in a cell (not in sidebar)
    /// vertex_id: The vertex being edited
    /// is_new: If true, cancellation deletes the vertex
    /// submitting: If true, we're waiting for server confirmation (don't show edit UI)
    InlineEdit { vertex_id: u64, is_new: bool, submitting: bool },
    /// Recording audio to create new vertex in the given direction
    Recording { direction: usize },
    /// Voice command mode (L2+R2 held on gamepad)
    VoiceCommand(VoiceCommandState),
}

/// Pending inline edit creation - waiting for server acknowledgment
#[derive(Clone, Debug)]
pub struct PendingInlineEdit {
    /// Action ID of the CreateVertex request
    pub action_id: u64,
    /// Direction the vertex was created in
    pub direction: usize,
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
    /// Local placeholder ID (if this was an async audio cell)
    pub local_placeholder_id: Option<u64>,
}

/// Kind-specific data for pending cells
#[derive(Clone, Debug)]
pub enum PendingCellKind {
    /// Audio recording/processing
    Audio {
        /// Current processing status
        status: PendingAudioStatus,
        /// Waveform preview data (downsampled amplitudes for visualization)
        waveform: Vec<f32>,
        /// Current audio level (0.0-1.0) for live recording visualization
        current_audio_level: f32,
    },
    /// Text being edited inline
    Text,
}

/// Status of a pending audio cell being processed in the background
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingAudioStatus {
    /// Currently recording audio (shows live microphone level)
    Recording,
    /// Audio is being encoded to OGG Vorbis
    Encoding,
    /// Encoded audio is being uploaded to server
    Uploading,
    /// Audio is being transcribed by Whisper
    Transcribing,
    /// All processing complete
    Complete,
}

/// A pending cell shown as a placeholder (audio recording or text editing)
#[derive(Clone, Debug)]
pub struct PendingCell {
    /// Temporary local ID (high bits set to distinguish from server IDs)
    pub local_id: u64,
    /// Direction from current vertex where this cell will be created
    pub direction: usize,
    /// Vertex ID this cell is connected from
    pub from_vertex: u64,
    /// When this pending cell was created
    pub created_at: Instant,
    /// Server-assigned vertex ID once creation is acknowledged (None until then)
    pub server_vertex_id: Option<u64>,
    /// Action ID used for CreateVertex (to match Log response)
    pub action_id: Option<u64>,
    /// Kind-specific data
    pub kind: PendingCellKind,
}

impl PendingCell {
    /// Get the audio status if this is an audio cell
    pub fn audio_status(&self) -> Option<PendingAudioStatus> {
        match &self.kind {
            PendingCellKind::Audio { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Set the audio status (no-op if not an audio cell)
    pub fn set_audio_status(&mut self, new_status: PendingAudioStatus) {
        if let PendingCellKind::Audio { status, .. } = &mut self.kind {
            *status = new_status;
        }
    }

    /// Get current audio level if this is an audio cell
    pub fn audio_level(&self) -> f32 {
        match &self.kind {
            PendingCellKind::Audio { current_audio_level, .. } => *current_audio_level,
            _ => 0.0,
        }
    }

    /// Set current audio level (no-op if not an audio cell)
    pub fn set_audio_level(&mut self, level: f32) {
        if let PendingCellKind::Audio { current_audio_level, .. } = &mut self.kind {
            *current_audio_level = level;
        }
    }

    /// Get waveform data if this is an audio cell
    pub fn waveform(&self) -> Option<&Vec<f32>> {
        match &self.kind {
            PendingCellKind::Audio { waveform, .. } => Some(waveform),
            _ => None,
        }
    }

    /// Set waveform data (no-op if not an audio cell)
    pub fn set_waveform(&mut self, new_waveform: Vec<f32>) {
        if let PendingCellKind::Audio { waveform, .. } = &mut self.kind {
            *waveform = new_waveform;
        }
    }

    /// Check if this is a recording audio cell
    pub fn is_recording(&self) -> bool {
        matches!(&self.kind, PendingCellKind::Audio { status: PendingAudioStatus::Recording, .. })
    }

    /// Check if this is a text cell
    pub fn is_text(&self) -> bool {
        matches!(&self.kind, PendingCellKind::Text)
    }
}

/// Backwards compatibility alias
pub type PendingAudioCell = PendingCell;

/// A placeholder cell shown while a portal/landmark is loading
#[derive(Clone, Debug)]
pub struct LoadingPortalCell {
    /// Direction from current vertex where the portal is
    pub direction: usize,
    /// Vertex ID this cell is connected from
    pub from_vertex: u64,
    /// When loading started
    pub created_at: Instant,
    /// The landmark URL being loaded
    pub landmark_url: String,
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

// ============================================================================
// Context Menu Types
// ============================================================================

use crate::commands::{Command, Context as CmdContext};

/// Type of item in the context menu
#[derive(Clone, Debug)]
pub enum ContextMenuItemType {
    /// Execute a command
    Command(Command),
    /// Navigate to a submenu for this context
    Submenu(CmdContext),
    /// Show controller help
    Help,
}

/// A single item in the context menu
#[derive(Clone, Debug)]
pub struct ContextMenuItem {
    pub label: String,
    pub item_type: ContextMenuItemType,
    pub width: f32,
}

/// Grid of context menu items
#[derive(Clone, Debug, Default)]
pub struct ContextMenuGrid {
    pub items: HashMap<(i32, i32), ContextMenuItem>,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl ContextMenuGrid {
    /// Recalculate bounds based on current items
    pub fn recalculate_bounds(&mut self) {
        if self.items.is_empty() {
            self.min_x = 0;
            self.max_x = 0;
            self.min_y = 0;
            self.max_y = 0;
            return;
        }

        self.min_x = i32::MAX;
        self.max_x = i32::MIN;
        self.min_y = i32::MAX;
        self.max_y = i32::MIN;

        for (x, y) in self.items.keys() {
            self.min_x = self.min_x.min(*x);
            self.max_x = self.max_x.max(*x);
            self.min_y = self.min_y.min(*y);
            self.max_y = self.max_y.max(*y);
        }
    }
}

/// State for the gamepad context menu
#[derive(Clone, Debug, Default)]
pub struct ContextMenuState {
    /// Whether the context menu is open
    pub open: bool,
    /// Current cursor position in the grid
    pub current_position: (i32, i32),
    /// The current grid of items
    pub grid: ContextMenuGrid,
    /// Cooldown to prevent rapid navigation
    pub last_nav_time: Option<Instant>,
}

/// State for server bar autocomplete dropdown
#[derive(Default)]
pub struct ServerDropdownState {
    pub selected_index: usize,
    pub filtered_indices: Vec<usize>,
    /// True when user is actively navigating the dropdown (pressed arrow keys)
    /// vs just having the dropdown visible
    pub is_active: bool,
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
    /// The vertex ID of the portal we're currently loading (for loading animation)
    pub loading_portal_vertex: Option<u64>,
    /// Placeholder cell shown while a portal is loading
    pub loading_portal_cell: Option<LoadingPortalCell>,
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
    /// Selected button in identification dialog: 0=Identify, 1=Remember, 2=Refuse
    pub identification_button_selected: usize,
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
    /// Original content before inline edit (for restore on cancel)
    pub inline_edit_original: Option<String>,
    // Audio recording state
    pub audio_samples: Arc<Mutex<Vec<f32>>>,
    pub recording_start: Option<Instant>,
    /// Action ID counter (counts down from MAX to avoid collision with server IDs)
    pub next_action_id: u64,
    /// Local ID counter for placeholder cells (counts down from MAX-1000000 to avoid collision)
    pub next_local_id: u64,
    /// Pending vertex creations: maps action_id -> data for populating vertex locally on ack
    pub pending_creations: HashMap<u64, PendingVertexCreation>,
    /// Pending audio cells being processed in background: maps local_id -> cell
    pub pending_audio_cells: HashMap<u64, PendingAudioCell>,
    /// Pending inline edit - waiting for server to create the vertex
    pub pending_inline_edit: Option<PendingInlineEdit>,
    /// Currently recording placeholder ID (selected/focused during recording)
    pub recording_placeholder_id: Option<u64>,
    /// Skip auto-play for this vertex (set after recording to avoid immediate playback)
    pub skip_autoplay_vertex: Option<u64>,
    /// Last navigation direction (used to determine where new vertices are created)
    pub last_nav_direction: usize,
    /// Focus URL bar on next frame (to avoid 'l' being typed when pressing Ctrl+L)
    pub focus_url_bar_next_frame: bool,
    /// Refresh pending from voice command (checked in main.rs)
    pub voice_refresh_pending: bool,
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
    // Export state
    pub export_state: ExportState,
    // Elf system state
    /// List of trusted elf URLs (user-configured)
    pub trusted_elves: Vec<TrustedElf>,
    /// Active elf tasks (action_id -> task state)
    pub active_elf_tasks: HashMap<u64, ElfTask>,
    /// Whether the elf panel is visible
    pub show_elf_panel: bool,
    /// Elf panel UI state
    pub elf_panel: ElfPanelState,
    /// Local services from gradesta-service-manager
    pub local_services: Option<LocalServices>,
    /// Server bar autocomplete dropdown state
    pub server_dropdown: ServerDropdownState,
    /// Show gamepad help overlay
    pub show_gamepad_help: bool,
    /// Show voice command settings dialog
    pub show_voice_settings: bool,
    /// When L2 trigger was first pressed (for tap vs hold detection)
    pub l2_press_start: Option<Instant>,
    /// Model fetch state for voice settings
    pub model_fetch_state: crate::voice_command::ModelFetchState,
    /// Filter text for model search
    pub model_filter: String,
    /// Generated image buffer (from voice command image generation)
    pub generated_image_buffer: Option<GeneratedImage>,
    /// Context menu state (gamepad)
    pub context_menu: ContextMenuState,
    // Sidebar gamepad navigation state
    /// Selected index in bag panel (for gamepad navigation)
    pub bag_panel_selected: usize,
    /// Selected index in nav panel (for gamepad navigation)
    pub nav_panel_selected: usize,
    /// Which section in nav panel: 0=history, 1=islands
    pub nav_panel_section: usize,
    /// Selected index in debug panel (for gamepad navigation)
    pub debug_panel_selected: usize,
    /// Selected index in identity panel (for gamepad navigation)
    pub identity_panel_selected: usize,
    // Undo navigation state
    /// Position saved before entering undo tree view (landmark, vertex)
    pub pre_undo_position: Option<(String, Option<u64>)>,
    /// Whether we're currently viewing the undo tree
    pub viewing_undo_tree: bool,

    // Command bar LLM state
    /// True while waiting for LLM response
    pub command_bar_llm_pending: bool,
    /// LLM interpretations (from voice_command::AgentInterpretation)
    pub command_bar_interpretations: Vec<crate::voice_command::AgentInterpretation>,
    /// Selected interpretation index
    pub command_bar_interpretation_selected: usize,
    /// True when navigating the filtered command list (up arrow pressed)
    pub command_bar_in_list: bool,

    // Content watching state (for topology/content separation)
    /// Active content watches: (vertex_id, layer) pairs we're watching for updates
    pub active_content_watches: HashSet<(u64, u32)>,
    /// Pending content requests: (vertex_id, layer) pairs with in-flight requests
    pub pending_content_requests: HashSet<(u64, u32)>,
}

/// A generated image waiting to be inserted into a cell
#[derive(Clone, Debug)]
pub struct GeneratedImage {
    /// MIME type of the image (e.g., "image/png")
    pub mime: String,
    /// Raw image data
    pub data: Vec<u8>,
}

/// Playback speed boost state for TTS and audio playback
/// Activated by left trigger on gamepad
#[derive(Resource)]
pub struct PlaybackBoostState {
    /// Current speed multiplier (1.0 = normal)
    pub speed: f32,
    /// When the first boost was applied (to track 3-second window)
    pub first_boost_time: Option<Instant>,
    /// When the last boost was applied (for 10-second timeout)
    pub last_boost_time: Option<Instant>,
}

impl Default for PlaybackBoostState {
    fn default() -> Self {
        Self {
            speed: 1.0,
            first_boost_time: None,
            last_boost_time: None,
        }
    }
}

impl PlaybackBoostState {
    /// Duration after which boost expires
    pub const BOOST_TIMEOUT: Duration = Duration::from_secs(10);

    /// Window after first boost during which speed can still increase
    pub const SPEED_INCREASE_WINDOW: Duration = Duration::from_secs(3);

    /// Maximum speed multiplier
    pub const MAX_SPEED: f32 = 4.0;

    /// Speed increment per boost press
    pub const SPEED_INCREMENT: f32 = 0.5;

    /// Check if the boost has expired
    pub fn is_expired(&self) -> bool {
        match self.last_boost_time {
            Some(t) => t.elapsed() >= Self::BOOST_TIMEOUT,
            None => true,
        }
    }

    /// Check if we're still in the window where speed can increase
    pub fn can_increase_speed(&self) -> bool {
        match self.first_boost_time {
            Some(t) => t.elapsed() < Self::SPEED_INCREASE_WINDOW && self.speed < Self::MAX_SPEED,
            None => true, // First boost can always increase
        }
    }

    /// Apply a boost (called on trigger press)
    /// Returns the new speed
    pub fn apply_boost(&mut self) -> f32 {
        let now = Instant::now();

        // Reset if expired
        if self.is_expired() {
            self.speed = 1.0;
            self.first_boost_time = None;
        }

        // Check if we can increase speed
        if self.can_increase_speed() {
            if self.first_boost_time.is_none() {
                self.first_boost_time = Some(now);
            }
            self.speed = (self.speed + Self::SPEED_INCREMENT).min(Self::MAX_SPEED);
        }

        // Always extend the timeout
        self.last_boost_time = Some(now);

        self.speed
    }

    /// Reset boost to normal speed
    pub fn reset(&mut self) {
        self.speed = 1.0;
        self.first_boost_time = None;
        self.last_boost_time = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        // Try to load identity config
        let identity_config = IdentityConfig::load().unwrap_or_default();

        // Load local services and auto-populate trusted elves
        let local_services = LocalServices::load();
        let trusted_elves: Vec<TrustedElf> = local_services
            .as_ref()
            .map(|ls| {
                ls.elves
                    .iter()
                    .map(|elf| TrustedElf::new(&elf.url))
                    .collect()
            })
            .unwrap_or_default();

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
            loading_portal_vertex: None,
            loading_portal_cell: None,
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
            identification_button_selected: 0,
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
            inline_edit_original: None,
            audio_samples: Arc::new(Mutex::new(Vec::new())),
            recording_start: None,
            next_action_id: u64::MAX,
            next_local_id: u64::MAX - 1_000_000, // Reserve top range for action IDs
            pending_creations: HashMap::new(),
            pending_audio_cells: HashMap::new(),
            pending_inline_edit: None,
            recording_placeholder_id: None,
            skip_autoplay_vertex: None,
            last_nav_direction: EDGE_SOUTH, // Default to south
            focus_url_bar_next_frame: false,
            voice_refresh_pending: false,
            url_bar_has_focus: false,
            show_video_modal: false,
            video_modal_vertex_id: None,
            sidebar: SidebarState::default(),
            keybindings: KeybindingResolver::new(&KeybindingsConfig::load().unwrap_or_default()),
            tts_mode: true,
            show_command_bar: false,
            command_bar_input: String::new(),
            command_bar_selected: 0,
            keybindings_editor: KeybindingsEditorState::default(),
            show_debug_panel: false,
            debug_log: Vec::new(),
            debug_log_file: None,
            debug_filter: DebugFilter::default(),
            debug_last_context: None,
            export_state: ExportState::new(),
            trusted_elves,
            active_elf_tasks: HashMap::new(),
            show_elf_panel: false,
            elf_panel: ElfPanelState::default(),
            local_services,
            server_dropdown: ServerDropdownState::default(),
            show_gamepad_help: false,
            show_voice_settings: false,
            l2_press_start: None,
            model_fetch_state: crate::voice_command::ModelFetchState::default(),
            model_filter: String::new(),
            generated_image_buffer: None,
            context_menu: ContextMenuState::default(),
            bag_panel_selected: 0,
            nav_panel_selected: 0,
            nav_panel_section: 0,
            debug_panel_selected: 0,
            identity_panel_selected: 0,
            pre_undo_position: None,
            viewing_undo_tree: false,
            command_bar_llm_pending: false,
            command_bar_interpretations: Vec::new(),
            command_bar_interpretation_selected: 0,
            command_bar_in_list: false,
            active_content_watches: HashSet::new(),
            pending_content_requests: HashSet::new(),
        }
    }
}
