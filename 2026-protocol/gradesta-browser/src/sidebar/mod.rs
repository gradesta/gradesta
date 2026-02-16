//! Sidebar module for content display and interaction modes
//!
//! This module replaces the modal dialogs with a unified sidebar-based UI.

mod bag;
pub mod elf;
mod export;
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
pub use export::{render_export_panel, ExportAction};

// Re-export Direction from state for backwards compatibility
pub use crate::state::Direction;

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

    /// HTML export panel
    Export,

    /// Elf management panel
    ElfBrowser,
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
        SidebarMode::Export => "E/W/N/S/U/D: Toggle directions | Enter: Export | Escape: Cancel",
        SidebarMode::ElfBrowser => "Enter: Summon | Escape: Close",
    }
}
