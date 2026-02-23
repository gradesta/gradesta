//! Sidebar module for content display and interaction modes
//!
//! This module replaces the modal dialogs with a unified sidebar-based UI.

pub mod elf;
mod export;
mod keybindings;

pub use keybindings::{render_keybindings_editor, KeybindingsEditorState, KeybindingsAction};
pub use export::{render_export_panel, ExportAction};

/// Sidebar display mode - determines what content is shown in the sidebar
#[derive(Clone, Debug, Default)]
pub enum SidebarMode {
    /// Default preview showing current vertex content
    #[default]
    Preview,

    /// Keybindings editor
    Keybindings,

    /// HTML export panel
    Export,
}

/// State for the sidebar panel
#[derive(Clone, Debug, Default)]
pub struct SidebarState {
    /// Current display mode
    pub mode: SidebarMode,
    /// Whether content is displayed fullscreen
    pub fullscreen: bool,
}
