//! UI module for the main interface components
//!
//! This module extracts the ui_system logic into feature-based submodules,
//! following the same action-enum pattern as the sidebar module.

mod command_bar;
mod commands;
mod fullscreen;
mod grid;
mod input;
mod processing;
mod sidebar_content;
mod text_edit;
pub mod url_utils;

pub use command_bar::{execute_command_bar_command, render_command_bar, CommandBarAction};
pub use commands::{execute_commands, finalize_recording};
pub use fullscreen::{render_fullscreen_content, FullscreenAction};
pub use grid::render_grid_view;
pub use input::{capture_keyboard_commands, log_triggered_commands_to_debug};
pub use processing::{process_identification, process_recording_cancel, process_text_input};
pub use sidebar_content::{render_sidebar_content, SidebarContentAction};
pub use text_edit::{consume_text_edit_events, handle_url_bar_copy_shortcut, handle_url_bar_smart_paste, process_text_edit_commands, sync_copy_to_system_clipboard};
