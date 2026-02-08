//! UI module for the main interface components
//!
//! This module extracts the ui_system logic into feature-based submodules,
//! following the same action-enum pattern as the sidebar module.

mod command_bar;
mod commands;
mod fullscreen;
mod grid;
mod input;
mod panels;
mod processing;
mod sidebar_content;

pub use command_bar::{execute_command_bar_command, render_command_bar, CommandBarAction};
pub use commands::{execute_commands, finalize_recording};
pub use fullscreen::{render_fullscreen_content, FullscreenAction};
pub use grid::render_grid_view;
pub use input::capture_keyboard_commands;
pub use processing::{process_identification, process_recording_cancel, process_text_input};
pub use sidebar_content::{render_sidebar_content, SidebarContentAction};

/// Result of a zoom operation
#[derive(Clone, Debug, Default)]
pub struct ZoomResult {
    pub zoom_changed: bool,
    pub new_zoom: f32,
}

/// Common action types used across UI components
#[derive(Clone, Debug, PartialEq)]
pub enum NavigationAction {
    None,
    Navigate { direction: usize },
    ClickVertex,
    GoToBagTop,
}

/// Result of connection/URL operations
#[derive(Clone, Debug)]
pub enum ConnectionAction {
    None,
    Connect { url: String },
    Disconnect,
    Refresh,
}

/// Actions for media playback control
#[derive(Clone, Debug, PartialEq)]
pub enum PlaybackAction {
    None,
    Play,
    Pause,
    Stop,
    Toggle,
}
