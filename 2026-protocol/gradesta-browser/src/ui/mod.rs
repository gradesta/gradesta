//! UI module for the main interface components
//!
//! This module extracts the ui_system logic into feature-based submodules,
//! following the same action-enum pattern as the sidebar module.

mod command_bar;
mod commands;
mod context_menu;
mod fullscreen;
mod gamepad_help;
mod grid;
mod input;
mod processing;
mod sidebar_content;
mod text_edit;
pub mod url_utils;
mod voice_command_overlay;

pub use command_bar::{execute_command_bar_command, render_command_bar, CommandBarAction};
pub use commands::{execute_commands, execute_voice_action, finalize_recording, finalize_voice_recording, grant_voice_permission, process_voice_command_events};
pub use context_menu::render_context_menu;
pub use fullscreen::{render_fullscreen_content, FullscreenAction};
pub use gamepad_help::render_gamepad_help_overlay;
pub use grid::render_grid_view;
pub use input::{capture_context_menu_gamepad, capture_gamepad_commands, capture_keyboard_commands, capture_voice_command_gamepad, log_triggered_commands_to_debug, CapturedCommands};
pub use processing::{process_identification, process_recording_cancel, process_text_input};
pub use sidebar_content::{render_sidebar_content, SidebarContentAction};
pub use text_edit::{consume_text_edit_events, handle_url_bar_copy_shortcut, handle_url_bar_smart_paste, process_text_edit_commands, sync_copy_to_system_clipboard};
pub use voice_command_overlay::{render_voice_command_overlay, render_voice_settings_dialog, VoiceSettingsAction};
