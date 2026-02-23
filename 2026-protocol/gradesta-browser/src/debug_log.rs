//! Debug logging module for command and keybinding diagnostics.
//!
//! This module provides functions for logging commands, keypresses, context changes,
//! and command execution results. Logs are stored in memory (with a limit) and
//! also written to a file in /tmp for later analysis.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use chrono::Local;

use crate::commands::Context;
use crate::state::{AppState, DebugCategory, DebugLogEntry};

/// Maximum number of log entries to keep in memory
const MAX_LOG_ENTRIES: usize = 500;

/// Initialize the debug log file and return its path.
/// Creates a timestamped file in /tmp.
pub fn init_debug_log() -> PathBuf {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let path = PathBuf::from(format!("/tmp/gradesta-debug-{}.log", timestamp));

    // Create the file with a header
    if let Ok(mut file) = File::create(&path) {
        let _ = writeln!(file, "# Gradesta Debug Log");
        let _ = writeln!(file, "# Started: {}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
        let _ = writeln!(file, "# Format: [timestamp] [category] message");
        let _ = writeln!(file, "#");
    }

    path
}

/// Add a log entry to the debug log (both memory and file)
pub fn log_entry(app_state: &mut AppState, category: DebugCategory, message: String) {
    let entry = DebugLogEntry {
        timestamp: Instant::now(),
        category,
        message: message.clone(),
    };

    // Add to in-memory log
    app_state.debug_log.push(entry);

    // Trim to max size (remove oldest entries)
    while app_state.debug_log.len() > MAX_LOG_ENTRIES {
        app_state.debug_log.remove(0);
    }

    // Write to file if path is set
    if let Some(ref path) = app_state.debug_log_file {
        if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] [{}] {}", timestamp, category.label(), message);
        }
    }
}

/// Log a context change
pub fn log_context_change(app_state: &mut AppState, old_context: &str, new_context: &str) {
    if old_context != new_context {
        log_entry(
            app_state,
            DebugCategory::Context,
            format!("{} → {}", old_context, new_context),
        );
    }
}

/// Log a command that was triggered, including the key that triggered it
pub fn log_command_triggered(app_state: &mut AppState, command_name: &str, key_info: Option<&str>) {
    let message = if let Some(key) = key_info {
        format!("{} ({})", command_name, key)
    } else {
        command_name.to_string()
    };
    log_entry(app_state, DebugCategory::Command, message);
}

/// Get context name as string (matches keybindings config section names)
pub fn context_name(ctx: Context) -> &'static str {
    ctx.config_name()
}

/// Clear the in-memory debug log (file is kept)
pub fn clear_log(app_state: &mut AppState) {
    app_state.debug_log.clear();

    // Add a note to the file
    if let Some(ref path) = app_state.debug_log_file {
        if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] [---] === Log cleared in UI ===", timestamp);
        }
    }
}
