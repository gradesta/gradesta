//! Sidebar module for content display and interaction modes
//!
//! This module replaces the modal dialogs with a unified sidebar-based UI.

mod bag;
mod identification;
mod identity;
mod image;
mod keybindings;
mod preview;
mod recording;
mod text;
mod text_input;
mod video;

use bevy_egui::egui;

pub use keybindings::{render_keybindings_editor, KeybindingsEditorState, KeybindingsAction};

/// Direction for creating new vertices or recording audio
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    West,
    East,
    North,
    South,
    Up,
    Down,
}

impl Direction {
    pub fn to_edge_index(&self) -> usize {
        match self {
            Direction::West => 0,
            Direction::East => 1,
            Direction::North => 2,
            Direction::South => 3,
            Direction::Up => 4,
            Direction::Down => 5,
        }
    }

    pub fn from_edge_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Direction::West),
            1 => Some(Direction::East),
            2 => Some(Direction::North),
            3 => Some(Direction::South),
            4 => Some(Direction::Up),
            5 => Some(Direction::Down),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Direction::West => "West",
            Direction::East => "East",
            Direction::North => "North",
            Direction::South => "South",
            Direction::Up => "Up",
            Direction::Down => "Down",
        }
    }

    pub fn arrow(&self) -> &'static str {
        match self {
            Direction::West => "←",
            Direction::East => "→",
            Direction::North => "↑",
            Direction::South => "↓",
            Direction::Up => "⬆",
            Direction::Down => "⬇",
        }
    }
}

/// Sidebar display mode - determines what content is shown in the sidebar
#[derive(Clone, Debug)]
pub enum SidebarMode {
    /// Default preview showing current vertex content
    Preview,

    /// Text viewing/editing mode
    TextView {
        vertex_id: u64,
        editing: bool,
        content: String,
    },

    /// Image viewing mode (with optional layer 2 full-res)
    ImageView {
        vertex_id: u64,
    },

    /// Video playback mode
    VideoPlay {
        vertex_id: u64,
    },

    /// Audio recording mode (blocking - greys out rest of UI)
    Recording {
        direction: Direction,
    },

    /// Identity management panel
    IdentityManagement,

    /// Identification request from server (blocking - greys out rest of UI)
    IdentificationRequest {
        action_id: u64,
        reason: String,
        nonce: [u8; 32],
        timestamp: u64,
        server_url: String,
    },

    /// Text input for creating new vertex
    TextInput {
        direction: Option<Direction>, // None = editing current vertex
        content: String,
    },

    /// Bag (clipboard) panel
    Bag,

    /// Keybindings editor
    Keybindings,
}

impl Default for SidebarMode {
    fn default() -> Self {
        SidebarMode::Preview
    }
}

impl SidebarMode {
    /// Check if this mode is blocking (should grey out the rest of the UI)
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            SidebarMode::Recording { .. } | SidebarMode::IdentificationRequest { .. }
        )
    }

    /// Check if this mode supports fullscreen
    pub fn supports_fullscreen(&self) -> bool {
        matches!(
            self,
            SidebarMode::Preview
                | SidebarMode::TextView { .. }
                | SidebarMode::ImageView { .. }
                | SidebarMode::VideoPlay { .. }
        )
    }
}

/// State for the sidebar panel
#[derive(Clone, Debug)]
pub struct SidebarState {
    /// Current display mode
    pub mode: SidebarMode,
    /// Whether content is displayed fullscreen
    pub fullscreen: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        SidebarState {
            mode: SidebarMode::Preview,
            fullscreen: false,
        }
    }
}

impl SidebarState {
    /// Reset to preview mode, exiting fullscreen
    pub fn reset(&mut self) {
        self.mode = SidebarMode::Preview;
        self.fullscreen = false;
    }

    /// Enter a new mode
    pub fn enter_mode(&mut self, mode: SidebarMode) {
        self.mode = mode;
        // Don't auto-fullscreen
    }

    /// Toggle fullscreen for current mode (if supported)
    pub fn toggle_fullscreen(&mut self) {
        if self.mode.supports_fullscreen() {
            self.fullscreen = !self.fullscreen;
        }
    }

    /// Exit fullscreen without changing mode
    pub fn exit_fullscreen(&mut self) {
        self.fullscreen = false;
    }

    /// Check if we're in a blocking mode
    pub fn is_blocking(&self) -> bool {
        self.mode.is_blocking()
    }
}

/// Render a semi-transparent blocking overlay over the given area
pub fn render_blocking_overlay(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("blocking_overlay"),
    ));
    let screen = ctx.screen_rect();
    painter.rect_filled(
        screen,
        0.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180),
    );
}

/// Get the help text for the current sidebar mode
pub fn get_help_text(state: &SidebarState) -> &'static str {
    if state.fullscreen {
        return "Escape: Exit fullscreen | Ctrl+Enter: Toggle fullscreen";
    }

    match &state.mode {
        SidebarMode::Preview => {
            "Arrow/WASD: Navigate | Enter: Activate | I: Edit | Ctrl+Enter: Fullscreen"
        }
        SidebarMode::TextView { editing, .. } => {
            if *editing {
                "Ctrl+Enter: Save | Escape: Cancel"
            } else {
                "I: Edit | Escape: Back | Ctrl+Enter: Fullscreen"
            }
        }
        SidebarMode::ImageView { .. } => "Escape: Back | Ctrl+Enter: Fullscreen",
        SidebarMode::VideoPlay { .. } => {
            "Space: Play/Pause | Escape: Back | Ctrl+Enter: Fullscreen"
        }
        SidebarMode::Recording { .. } => "Release Space: Save | Escape: Cancel",
        SidebarMode::IdentityManagement => "Escape: Close",
        SidebarMode::IdentificationRequest { .. } => "Enter: Accept | Escape: Refuse",
        SidebarMode::TextInput { .. } => "Ctrl+Enter: Save | Escape: Cancel",
        SidebarMode::Bag => "Y: Yank | G: Go to top | Ctrl+Y: Pop | Escape: Close",
        SidebarMode::Keybindings => "Enter: Edit | Escape: Close",
    }
}
