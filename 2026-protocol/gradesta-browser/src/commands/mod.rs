//! Command system for gradesta-browser
//!
//! Commands are identified by slugs using OOP-like syntax:
//! - `global.toggle_bag` - Global commands available everywhere
//! - `graph.navigate_north` - Graph context commands
//! - `bag.pop` - Bag sidebar commands
//! - `auth.accept` - Authentication context commands

use serde::{Deserialize, Serialize};

/// Input contexts for keybinding resolution
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Context {
    /// Global commands available in all contexts
    Global,
    /// Graph navigation and manipulation
    Graph,
    /// Bag/clipboard sidebar
    Bag,
    /// Text input mode
    TextInput,
    /// Audio recording mode
    Recording,
    /// Authentication/identification dialog
    Authentication,
    /// Navigation panel sidebar
    NavPanel,
}

impl Context {
    pub fn name(&self) -> &'static str {
        match self {
            Context::Global => "Global",
            Context::Graph => "Graph",
            Context::Bag => "Bag",
            Context::TextInput => "Text Input",
            Context::Recording => "Recording",
            Context::Authentication => "Authentication",
            Context::NavPanel => "Navigation Panel",
        }
    }

    pub fn all() -> &'static [Context] {
        &[
            Context::Global,
            Context::Graph,
            Context::Bag,
            Context::TextInput,
            Context::Recording,
            Context::Authentication,
            Context::NavPanel,
        ]
    }
}

/// All available commands in the application
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    // === Global Commands ===
    /// Focus the URL input bar
    GlobalFocusUrl,
    /// Zoom in the graph view
    GlobalZoomIn,
    /// Zoom out the graph view
    GlobalZoomOut,
    /// Reset zoom to 100%
    GlobalZoomReset,
    /// Toggle the bag/clipboard sidebar
    GlobalToggleBag,
    /// Toggle the navigation panel sidebar
    GlobalToggleNavPanel,
    /// Close current modal or panel
    GlobalCloseModal,
    /// Copy current URL to clipboard
    GlobalCopyUrl,
    /// Refresh connection
    GlobalRefresh,
    /// Toggle fullscreen mode
    GlobalToggleFullscreen,
    /// Open the command bar
    GlobalOpenCommandBar,
    /// Open keybindings editor
    GlobalOpenKeybindings,
    /// Toggle text-to-speech mode
    GlobalToggleTTS,
    /// Increase TTS playback speed
    GlobalTTSSpeedUp,
    /// Decrease TTS playback speed
    GlobalTTSSpeedDown,

    // === Graph Context Commands ===
    /// Navigate north in the graph
    GraphNavigateNorth,
    /// Navigate south in the graph
    GraphNavigateSouth,
    /// Navigate east in the graph
    GraphNavigateEast,
    /// Navigate west in the graph
    GraphNavigateWest,
    /// Navigate up in the graph (stack)
    GraphNavigateUp,
    /// Navigate down in the graph (stack)
    GraphNavigateDown,
    /// Go back in navigation history
    GraphHistoryBack,
    /// Click/activate the current vertex
    GraphClickVertex,
    /// Edit text at current vertex
    GraphEditText,
    /// Create new text vertex in last nav direction
    GraphNewTextVertex,
    /// Start audio recording
    GraphStartRecording,
    /// Yank current vertex to bag
    GraphYank,
    /// Paste from bag in last nav direction
    GraphPaste,
    /// Cut edge in last nav direction
    GraphCutEdge,
    /// Go to top item in bag
    GraphGoToBagTop,
    /// Delete current vertex
    GraphDeleteVertex,
    /// Set direction to north without moving
    GraphSetDirectionNorth,
    /// Set direction to south without moving
    GraphSetDirectionSouth,
    /// Set direction to east without moving
    GraphSetDirectionEast,
    /// Set direction to west without moving
    GraphSetDirectionWest,
    /// Set direction to up without moving
    GraphSetDirectionUp,
    /// Set direction to down without moving
    GraphSetDirectionDown,

    // === Bag Context Commands ===
    /// Pop top item from bag
    BagPop,
    /// Clear all items from bag
    BagClear,

    // === Text Input Context Commands ===
    /// Submit/save text input
    TextInputSubmit,
    /// Cancel text input
    TextInputCancel,

    // === Recording Context Commands ===
    /// Save recording (stop and save)
    RecordingSave,
    /// Cancel recording
    RecordingCancel,

    // === Authentication Context Commands ===
    /// Cycle through available identities
    AuthCycleIdentity,
    /// Accept identification request
    AuthAccept,
    /// Accept and remember server
    AuthAcceptRemember,
    /// Refuse identification request
    AuthRefuse,
}

impl Command {
    /// Get the command slug (e.g., "graph.navigate_north")
    pub fn slug(&self) -> &'static str {
        match self {
            // Global
            Command::GlobalFocusUrl => "global.focus_url",
            Command::GlobalZoomIn => "global.zoom_in",
            Command::GlobalZoomOut => "global.zoom_out",
            Command::GlobalZoomReset => "global.zoom_reset",
            Command::GlobalToggleBag => "global.toggle_bag",
            Command::GlobalToggleNavPanel => "global.toggle_nav_panel",
            Command::GlobalCloseModal => "global.close_modal",
            Command::GlobalCopyUrl => "global.copy_url",
            Command::GlobalRefresh => "global.refresh",
            Command::GlobalToggleFullscreen => "global.toggle_fullscreen",
            Command::GlobalOpenCommandBar => "global.open_command_bar",
            Command::GlobalOpenKeybindings => "global.open_keybindings",
            Command::GlobalToggleTTS => "global.toggle_tts",
            Command::GlobalTTSSpeedUp => "global.tts_speed_up",
            Command::GlobalTTSSpeedDown => "global.tts_speed_down",
            // Graph
            Command::GraphNavigateNorth => "graph.navigate_north",
            Command::GraphNavigateSouth => "graph.navigate_south",
            Command::GraphNavigateEast => "graph.navigate_east",
            Command::GraphNavigateWest => "graph.navigate_west",
            Command::GraphNavigateUp => "graph.navigate_up",
            Command::GraphNavigateDown => "graph.navigate_down",
            Command::GraphHistoryBack => "graph.history_back",
            Command::GraphClickVertex => "graph.click_vertex",
            Command::GraphEditText => "graph.edit_text",
            Command::GraphNewTextVertex => "graph.new_text_vertex",
            Command::GraphStartRecording => "graph.start_recording",
            Command::GraphYank => "graph.yank",
            Command::GraphPaste => "graph.paste",
            Command::GraphCutEdge => "graph.cut_edge",
            Command::GraphGoToBagTop => "graph.go_to_bag_top",
            Command::GraphDeleteVertex => "graph.delete_vertex",
            Command::GraphSetDirectionNorth => "graph.set_direction_north",
            Command::GraphSetDirectionSouth => "graph.set_direction_south",
            Command::GraphSetDirectionEast => "graph.set_direction_east",
            Command::GraphSetDirectionWest => "graph.set_direction_west",
            Command::GraphSetDirectionUp => "graph.set_direction_up",
            Command::GraphSetDirectionDown => "graph.set_direction_down",
            // Bag
            Command::BagPop => "bag.pop",
            Command::BagClear => "bag.clear",
            // TextInput
            Command::TextInputSubmit => "text_input.submit",
            Command::TextInputCancel => "text_input.cancel",
            // Recording
            Command::RecordingSave => "recording.save",
            Command::RecordingCancel => "recording.cancel",
            // Auth
            Command::AuthCycleIdentity => "auth.cycle_identity",
            Command::AuthAccept => "auth.accept",
            Command::AuthAcceptRemember => "auth.accept_remember",
            Command::AuthRefuse => "auth.refuse",
        }
    }

    /// Parse a command from its slug
    pub fn from_slug(s: &str) -> Option<Command> {
        match s {
            // Global
            "global.focus_url" => Some(Command::GlobalFocusUrl),
            "global.zoom_in" => Some(Command::GlobalZoomIn),
            "global.zoom_out" => Some(Command::GlobalZoomOut),
            "global.zoom_reset" => Some(Command::GlobalZoomReset),
            "global.toggle_bag" => Some(Command::GlobalToggleBag),
            "global.toggle_nav_panel" => Some(Command::GlobalToggleNavPanel),
            "global.close_modal" => Some(Command::GlobalCloseModal),
            "global.copy_url" => Some(Command::GlobalCopyUrl),
            "global.refresh" => Some(Command::GlobalRefresh),
            "global.toggle_fullscreen" => Some(Command::GlobalToggleFullscreen),
            "global.open_command_bar" => Some(Command::GlobalOpenCommandBar),
            "global.open_keybindings" => Some(Command::GlobalOpenKeybindings),
            "global.toggle_tts" => Some(Command::GlobalToggleTTS),
            "global.tts_speed_up" => Some(Command::GlobalTTSSpeedUp),
            "global.tts_speed_down" => Some(Command::GlobalTTSSpeedDown),
            // Graph
            "graph.navigate_north" => Some(Command::GraphNavigateNorth),
            "graph.navigate_south" => Some(Command::GraphNavigateSouth),
            "graph.navigate_east" => Some(Command::GraphNavigateEast),
            "graph.navigate_west" => Some(Command::GraphNavigateWest),
            "graph.navigate_up" => Some(Command::GraphNavigateUp),
            "graph.navigate_down" => Some(Command::GraphNavigateDown),
            "graph.history_back" => Some(Command::GraphHistoryBack),
            "graph.click_vertex" => Some(Command::GraphClickVertex),
            "graph.edit_text" => Some(Command::GraphEditText),
            "graph.new_text_vertex" => Some(Command::GraphNewTextVertex),
            "graph.start_recording" => Some(Command::GraphStartRecording),
            "graph.yank" => Some(Command::GraphYank),
            "graph.paste" => Some(Command::GraphPaste),
            "graph.cut_edge" => Some(Command::GraphCutEdge),
            "graph.go_to_bag_top" => Some(Command::GraphGoToBagTop),
            "graph.delete_vertex" => Some(Command::GraphDeleteVertex),
            "graph.set_direction_north" => Some(Command::GraphSetDirectionNorth),
            "graph.set_direction_south" => Some(Command::GraphSetDirectionSouth),
            "graph.set_direction_east" => Some(Command::GraphSetDirectionEast),
            "graph.set_direction_west" => Some(Command::GraphSetDirectionWest),
            "graph.set_direction_up" => Some(Command::GraphSetDirectionUp),
            "graph.set_direction_down" => Some(Command::GraphSetDirectionDown),
            // Bag
            "bag.pop" => Some(Command::BagPop),
            "bag.clear" => Some(Command::BagClear),
            // TextInput
            "text_input.submit" => Some(Command::TextInputSubmit),
            "text_input.cancel" => Some(Command::TextInputCancel),
            // Recording
            "recording.save" => Some(Command::RecordingSave),
            "recording.cancel" => Some(Command::RecordingCancel),
            // Auth
            "auth.cycle_identity" => Some(Command::AuthCycleIdentity),
            "auth.accept" => Some(Command::AuthAccept),
            "auth.accept_remember" => Some(Command::AuthAcceptRemember),
            "auth.refuse" => Some(Command::AuthRefuse),
            _ => None,
        }
    }

    /// Get human-readable description of the command
    pub fn description(&self) -> &'static str {
        match self {
            // Global
            Command::GlobalFocusUrl => "Focus the URL input bar",
            Command::GlobalZoomIn => "Zoom in the graph view",
            Command::GlobalZoomOut => "Zoom out the graph view",
            Command::GlobalZoomReset => "Reset zoom to 100%",
            Command::GlobalToggleBag => "Toggle the bag/clipboard sidebar",
            Command::GlobalToggleNavPanel => "Toggle the navigation panel",
            Command::GlobalCloseModal => "Close current modal or panel",
            Command::GlobalCopyUrl => "Copy current URL to clipboard",
            Command::GlobalRefresh => "Refresh connection",
            Command::GlobalToggleFullscreen => "Toggle fullscreen mode",
            Command::GlobalOpenCommandBar => "Open the command bar",
            Command::GlobalOpenKeybindings => "Open keybindings editor",
            Command::GlobalToggleTTS => "Toggle text-to-speech mode",
            Command::GlobalTTSSpeedUp => "Increase TTS speed",
            Command::GlobalTTSSpeedDown => "Decrease TTS speed",
            // Graph
            Command::GraphNavigateNorth => "Navigate north in the graph",
            Command::GraphNavigateSouth => "Navigate south in the graph",
            Command::GraphNavigateEast => "Navigate east in the graph",
            Command::GraphNavigateWest => "Navigate west in the graph",
            Command::GraphNavigateUp => "Navigate up in the graph (stack)",
            Command::GraphNavigateDown => "Navigate down in the graph (stack)",
            Command::GraphHistoryBack => "Go back in navigation history",
            Command::GraphClickVertex => "Click/activate the current vertex",
            Command::GraphEditText => "Edit text at current vertex",
            Command::GraphNewTextVertex => "Create new text vertex",
            Command::GraphStartRecording => "Start audio recording",
            Command::GraphYank => "Yank current vertex to bag",
            Command::GraphPaste => "Paste from bag",
            Command::GraphCutEdge => "Cut edge in current direction",
            Command::GraphGoToBagTop => "Go to top item in bag",
            Command::GraphDeleteVertex => "Delete current vertex",
            Command::GraphSetDirectionNorth => "Set direction to north",
            Command::GraphSetDirectionSouth => "Set direction to south",
            Command::GraphSetDirectionEast => "Set direction to east",
            Command::GraphSetDirectionWest => "Set direction to west",
            Command::GraphSetDirectionUp => "Set direction to up",
            Command::GraphSetDirectionDown => "Set direction to down",
            // Bag
            Command::BagPop => "Pop top item from bag",
            Command::BagClear => "Clear all items from bag",
            // TextInput
            Command::TextInputSubmit => "Submit/save text input",
            Command::TextInputCancel => "Cancel text input",
            // Recording
            Command::RecordingSave => "Save recording",
            Command::RecordingCancel => "Cancel recording",
            // Auth
            Command::AuthCycleIdentity => "Cycle through identities",
            Command::AuthAccept => "Accept identification",
            Command::AuthAcceptRemember => "Accept and remember server",
            Command::AuthRefuse => "Refuse identification",
        }
    }

    /// Get the primary context for this command
    pub fn context(&self) -> Context {
        match self {
            Command::GlobalFocusUrl
            | Command::GlobalZoomIn
            | Command::GlobalZoomOut
            | Command::GlobalZoomReset
            | Command::GlobalToggleBag
            | Command::GlobalToggleNavPanel
            | Command::GlobalCloseModal
            | Command::GlobalCopyUrl
            | Command::GlobalRefresh
            | Command::GlobalToggleFullscreen
            | Command::GlobalOpenCommandBar
            | Command::GlobalOpenKeybindings
            | Command::GlobalToggleTTS
            | Command::GlobalTTSSpeedUp
            | Command::GlobalTTSSpeedDown => Context::Global,

            Command::GraphNavigateNorth
            | Command::GraphNavigateSouth
            | Command::GraphNavigateEast
            | Command::GraphNavigateWest
            | Command::GraphNavigateUp
            | Command::GraphNavigateDown
            | Command::GraphHistoryBack
            | Command::GraphClickVertex
            | Command::GraphEditText
            | Command::GraphNewTextVertex
            | Command::GraphStartRecording
            | Command::GraphYank
            | Command::GraphPaste
            | Command::GraphCutEdge
            | Command::GraphGoToBagTop
            | Command::GraphDeleteVertex
            | Command::GraphSetDirectionNorth
            | Command::GraphSetDirectionSouth
            | Command::GraphSetDirectionEast
            | Command::GraphSetDirectionWest
            | Command::GraphSetDirectionUp
            | Command::GraphSetDirectionDown => Context::Graph,

            Command::BagPop | Command::BagClear => Context::Bag,

            Command::TextInputSubmit | Command::TextInputCancel => Context::TextInput,

            Command::RecordingSave | Command::RecordingCancel => Context::Recording,

            Command::AuthCycleIdentity
            | Command::AuthAccept
            | Command::AuthAcceptRemember
            | Command::AuthRefuse => Context::Authentication,
        }
    }

    /// Get all commands
    pub fn all() -> Vec<Command> {
        vec![
            // Global
            Command::GlobalFocusUrl,
            Command::GlobalZoomIn,
            Command::GlobalZoomOut,
            Command::GlobalZoomReset,
            Command::GlobalToggleBag,
            Command::GlobalToggleNavPanel,
            Command::GlobalCloseModal,
            Command::GlobalCopyUrl,
            Command::GlobalRefresh,
            Command::GlobalToggleFullscreen,
            Command::GlobalOpenCommandBar,
            Command::GlobalOpenKeybindings,
            Command::GlobalToggleTTS,
            Command::GlobalTTSSpeedUp,
            Command::GlobalTTSSpeedDown,
            // Graph
            Command::GraphNavigateNorth,
            Command::GraphNavigateSouth,
            Command::GraphNavigateEast,
            Command::GraphNavigateWest,
            Command::GraphNavigateUp,
            Command::GraphNavigateDown,
            Command::GraphHistoryBack,
            Command::GraphClickVertex,
            Command::GraphEditText,
            Command::GraphNewTextVertex,
            Command::GraphStartRecording,
            Command::GraphYank,
            Command::GraphPaste,
            Command::GraphCutEdge,
            Command::GraphGoToBagTop,
            Command::GraphDeleteVertex,
            Command::GraphSetDirectionNorth,
            Command::GraphSetDirectionSouth,
            Command::GraphSetDirectionEast,
            Command::GraphSetDirectionWest,
            Command::GraphSetDirectionUp,
            Command::GraphSetDirectionDown,
            // Bag
            Command::BagPop,
            Command::BagClear,
            // TextInput
            Command::TextInputSubmit,
            Command::TextInputCancel,
            // Recording
            Command::RecordingSave,
            Command::RecordingCancel,
            // Auth
            Command::AuthCycleIdentity,
            Command::AuthAccept,
            Command::AuthAcceptRemember,
            Command::AuthRefuse,
        ]
    }

    /// Get all commands for a specific context
    pub fn for_context(ctx: Context) -> Vec<Command> {
        Command::all()
            .into_iter()
            .filter(|c| c.context() == ctx)
            .collect()
    }

    /// Fuzzy match this command's slug against a query
    pub fn fuzzy_matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let slug = self.slug();
        let query_lower = query.to_lowercase();
        let slug_lower = slug.to_lowercase();

        // Check if all characters of query appear in order in slug
        let mut query_chars = query_lower.chars().peekable();
        for c in slug_lower.chars() {
            if query_chars.peek() == Some(&c) {
                query_chars.next();
            }
        }
        query_chars.peek().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_roundtrip() {
        for cmd in Command::all() {
            let slug = cmd.slug();
            let parsed = Command::from_slug(slug);
            assert_eq!(parsed, Some(cmd.clone()), "Failed roundtrip for {:?}", cmd);
        }
    }

    #[test]
    fn test_fuzzy_match() {
        let cmd = Command::GraphNavigateNorth;
        assert!(cmd.fuzzy_matches(""));
        assert!(cmd.fuzzy_matches("nav"));
        assert!(cmd.fuzzy_matches("gnn"));
        assert!(cmd.fuzzy_matches("north"));
        assert!(!cmd.fuzzy_matches("xyz"));
    }
}
