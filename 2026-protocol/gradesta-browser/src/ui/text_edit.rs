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

/// Intercept clipboard events and use system clipboard via arboard
///
/// This should be called BEFORE rendering any TextEdit widgets.
/// It replaces egui's broken clipboard handling with arboard.
pub fn consume_text_edit_events(ctx: &egui::Context, app_state: &AppState) {
    // Handle in any text input context (TextInput mode or URL bar focus)
    let in_text_context = matches!(app_state.input_mode, InputMode::TextInput { .. })
        || app_state.url_bar_has_focus
        || app_state.focus_url_bar_next_frame;

    if !in_text_context {
        return;
    }

    // Get system clipboard content for paste operations
    let system_clipboard = get_clipboard_text();

    ctx.input_mut(|input| {
        // Check if Ctrl is currently held
        let ctrl_held = input.modifiers.ctrl || input.modifiers.command;

        // Check if Ctrl+V was pressed - we need to inject a Paste event
        let ctrl_v_pressed = input.events.iter().any(|event| {
            matches!(event, egui::Event::Key {
                key: egui::Key::V,
                pressed: true,
                modifiers,
                ..
            } if modifiers.command || modifiers.ctrl)
        });

        // Check if Ctrl+C was pressed - we need to inject a Copy event
        let ctrl_c_pressed = input.events.iter().any(|event| {
            matches!(event, egui::Event::Key {
                key: egui::Key::C,
                pressed: true,
                modifiers,
                ..
            } if modifiers.command || modifiers.ctrl)
        });

        // Check if Ctrl+X was pressed - we need to inject a Cut event
        let ctrl_x_pressed = input.events.iter().any(|event| {
            matches!(event, egui::Event::Key {
                key: egui::Key::X,
                pressed: true,
                modifiers,
                ..
            } if modifiers.command || modifiers.ctrl)
        });

        // Replace any existing Paste event content with system clipboard
        for event in input.events.iter_mut() {
            if let egui::Event::Paste(text) = event {
                if let Some(ref clipboard_text) = system_clipboard {
                    *text = clipboard_text.clone();
                }
            }
        }

        // Filter out Ctrl+C/X/V key events and 'v'/'c'/'x' text insertions
        input.events.retain(|event| {
            match event {
                // Keep existing Copy/Cut/Paste events
                egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_) => true,
                // Filter Ctrl+C/X/V key events (we'll inject proper events below)
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if modifiers.command || modifiers.ctrl {
                        !matches!(key, egui::Key::C | egui::Key::X | egui::Key::V)
                    } else {
                        true
                    }
                }
                // Filter Text events when Ctrl is held
                egui::Event::Text(text) if ctrl_held => text.len() > 1,
                _ => true,
            }
        });

        // Inject clipboard events with system clipboard content
        if ctrl_v_pressed {
            if let Some(clipboard_text) = system_clipboard {
                input.events.push(egui::Event::Paste(clipboard_text));
            }
        }
        if ctrl_c_pressed {
            input.events.push(egui::Event::Copy);
        }
        if ctrl_x_pressed {
            input.events.push(egui::Event::Cut);
        }
    });
}

/// Sync selected text to system clipboard when Copy/Cut events occur
/// Call this after TextEdit rendering to capture what was copied
pub fn sync_copy_to_system_clipboard(ctx: &egui::Context) {
    ctx.input(|input| {
        for event in &input.events {
            match event {
                egui::Event::Copy | egui::Event::Cut => {
                    // egui stores copied text in its output
                    // We need to get it and sync to system clipboard
                }
                _ => {}
            }
        }
    });

    // Get text that egui copied to its internal clipboard and sync to system
    ctx.output_mut(|output| {
        if !output.copied_text.is_empty() {
            set_clipboard_text(&output.copied_text);
        }
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
