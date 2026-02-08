//! Text editing command execution for TextInput mode
//!
//! Implements copy, cut, paste, select all, undo, and redo operations
//! for the text input buffer using the system clipboard via arboard.

use bevy_egui::egui;

use super::input::CapturedCommands;
use crate::state::{AppState, InputMode};

/// Get text from the system clipboard
fn get_clipboard_text() -> Option<String> {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(e) => {
                    eprintln!("Failed to get clipboard text: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to access clipboard: {}", e);
            None
        }
    }
}

/// Set text to the system clipboard
fn set_clipboard_text(text: &str) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(text.to_string()) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("Failed to set clipboard text: {}", e);
                    false
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to access clipboard: {}", e);
            false
        }
    }
}

/// Result of text edit command processing
#[derive(Clone, Debug, Default)]
pub struct TextEditResult {
    /// Whether any text command was processed
    pub any_processed: bool,
    /// Whether the text buffer was modified
    pub text_modified: bool,
}

/// Consume text editing key events to prevent egui from using broken system clipboard
///
/// This should be called BEFORE rendering the TextEdit widget.
/// It consumes Ctrl+C/V/X/A/Z events so egui doesn't try to use the system clipboard.
pub fn consume_text_edit_events(ctx: &egui::Context, app_state: &AppState) {
    // Only consume in TextInput mode
    if !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        return;
    }

    ctx.input_mut(|input| {
        // Check if Ctrl is currently held
        let ctrl_held = input.modifiers.ctrl || input.modifiers.command;

        // Consume clipboard-related events to prevent egui from using system clipboard
        input.events.retain(|event| {
            match event {
                egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) => false,
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    // Consume Ctrl+C, Ctrl+X, Ctrl+V, Ctrl+A, Ctrl+Z, Ctrl+Y, Ctrl+Shift+Z
                    if modifiers.command || modifiers.ctrl {
                        !matches!(key,
                            egui::Key::C | egui::Key::X | egui::Key::V |
                            egui::Key::A | egui::Key::Z | egui::Key::Y
                        )
                    } else {
                        true
                    }
                }
                // Also filter Text events when Ctrl is held - prevents "v" being inserted for Ctrl+V
                egui::Event::Text(text) if ctrl_held => {
                    // Filter out single character text events when Ctrl is held
                    // (these are the fallback characters from Ctrl+key combos)
                    text.len() > 1
                }
                _ => true,
            }
        });
    });
}

/// Process text editing commands (copy, cut, paste, select all, undo, redo)
///
/// This function handles text manipulation commands when in TextInput mode.
/// It operates on the text_input_buffer and maintains selection state.
pub fn process_text_edit_commands(
    cmds: &CapturedCommands,
    app_state: &mut AppState,
) -> TextEditResult {
    let mut result = TextEditResult::default();

    // Only process in TextInput mode
    if !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        return result;
    }

    // Select All - Ctrl+A
    if cmds.text_select_all {
        result.any_processed = true;
        app_state.text_selection_start = Some(0);
        app_state.text_cursor_pos = app_state.text_input_buffer.len();
    }

    // Copy - Ctrl+C
    if cmds.text_copy {
        result.any_processed = true;
        if let Some(sel_start) = app_state.text_selection_start {
            let (start, end) = get_selection_range(sel_start, app_state.text_cursor_pos);
            if start < end && end <= app_state.text_input_buffer.len() {
                let text = app_state.text_input_buffer[start..end].to_string();
                set_clipboard_text(&text);
                app_state.text_clipboard = text; // Keep internal copy as fallback
            }
        }
    }

    // Cut - Ctrl+X
    if cmds.text_cut {
        result.any_processed = true;
        if let Some(sel_start) = app_state.text_selection_start {
            let (start, end) = get_selection_range(sel_start, app_state.text_cursor_pos);
            if start < end && end <= app_state.text_input_buffer.len() {
                // Save to undo stack before modifying
                push_undo(app_state);

                let text = app_state.text_input_buffer[start..end].to_string();
                set_clipboard_text(&text);
                app_state.text_clipboard = text; // Keep internal copy as fallback

                app_state.text_input_buffer = format!(
                    "{}{}",
                    &app_state.text_input_buffer[..start],
                    &app_state.text_input_buffer[end..]
                );
                app_state.text_cursor_pos = start;
                app_state.text_selection_start = None;
                result.text_modified = true;
            }
        }
    }

    // Paste - Ctrl+V
    if cmds.text_paste {
        result.any_processed = true;
        // Try system clipboard first, fall back to internal clipboard
        let clipboard_text = get_clipboard_text()
            .unwrap_or_else(|| app_state.text_clipboard.clone());

        if !clipboard_text.is_empty() {
            // Save to undo stack before modifying
            push_undo(app_state);

            // Delete selection first if any
            let insert_pos = if let Some(sel_start) = app_state.text_selection_start {
                let (start, end) = get_selection_range(sel_start, app_state.text_cursor_pos);
                if start < end && end <= app_state.text_input_buffer.len() {
                    app_state.text_input_buffer = format!(
                        "{}{}",
                        &app_state.text_input_buffer[..start],
                        &app_state.text_input_buffer[end..]
                    );
                }
                app_state.text_selection_start = None;
                start
            } else {
                app_state.text_cursor_pos.min(app_state.text_input_buffer.len())
            };

            // Insert clipboard content
            let clipboard_len = clipboard_text.len();
            app_state.text_input_buffer = format!(
                "{}{}{}",
                &app_state.text_input_buffer[..insert_pos],
                &clipboard_text,
                &app_state.text_input_buffer[insert_pos..]
            );
            app_state.text_cursor_pos = insert_pos + clipboard_len;
            result.text_modified = true;
        }
    }

    // Undo - Ctrl+Z
    if cmds.text_undo {
        result.any_processed = true;
        if let Some(prev_text) = app_state.text_undo_stack.pop() {
            // Push current state to redo stack
            app_state.text_redo_stack.push(app_state.text_input_buffer.clone());
            app_state.text_input_buffer = prev_text;
            app_state.text_cursor_pos = app_state.text_input_buffer.len();
            app_state.text_selection_start = None;
            result.text_modified = true;
        }
    }

    // Redo - Ctrl+Shift+Z or Ctrl+Y
    if cmds.text_redo {
        result.any_processed = true;
        if let Some(next_text) = app_state.text_redo_stack.pop() {
            // Push current state to undo stack
            app_state.text_undo_stack.push(app_state.text_input_buffer.clone());
            app_state.text_input_buffer = next_text;
            app_state.text_cursor_pos = app_state.text_input_buffer.len();
            app_state.text_selection_start = None;
            result.text_modified = true;
        }
    }

    result
}

/// Get selection range as (start, end) where start <= end
fn get_selection_range(sel_start: usize, cursor_pos: usize) -> (usize, usize) {
    if sel_start <= cursor_pos {
        (sel_start, cursor_pos)
    } else {
        (cursor_pos, sel_start)
    }
}

/// Push current text to undo stack, clearing redo stack
fn push_undo(app_state: &mut AppState) {
    app_state.text_undo_stack.push(app_state.text_input_buffer.clone());
    app_state.text_redo_stack.clear();
    // Limit undo stack size
    if app_state.text_undo_stack.len() > 50 {
        app_state.text_undo_stack.remove(0);
    }
}

/// Reset text editing state (call when entering text input mode)
pub fn reset_text_edit_state(app_state: &mut AppState) {
    app_state.text_cursor_pos = app_state.text_input_buffer.len();
    app_state.text_selection_start = None;
    app_state.text_undo_stack.clear();
    app_state.text_redo_stack.clear();
}

/// Select all text (call when entering text input mode for edit)
pub fn select_all(app_state: &mut AppState) {
    app_state.text_selection_start = Some(0);
    app_state.text_cursor_pos = app_state.text_input_buffer.len();
}
