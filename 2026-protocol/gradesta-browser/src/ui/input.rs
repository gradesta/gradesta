//! Keyboard input capture for UI system
//!
//! Pre-computes which commands were triggered this frame.

use bevy_egui::egui;

use crate::commands::{Command, Context as CmdContext};
use crate::debug_log;
use crate::keybindings::KeybindingResolver;
use crate::state::AppState;

/// All captured keyboard commands for a single frame
#[derive(Clone, Debug, Default)]
pub struct CapturedCommands {
    // Global commands
    pub close_modal: bool,
    pub toggle_fullscreen: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
    pub toggle_bag: bool,
    pub toggle_nav_panel: bool,
    pub open_command_bar: bool,
    pub open_keybindings: bool,
    pub toggle_tts: bool,
    pub refresh: bool,
    pub copy_url: bool,
    pub focus_url_down: bool,
    pub focus_url_released: bool,

    // Graph commands
    pub click_vertex: bool,
    pub yank: bool,
    pub bag_pop: bool,
    pub go_to_bag_top: bool,
    pub paste: bool,
    pub cut_edge: bool,
    pub delete_vertex: bool,
    pub edit_text: bool,
    pub new_text_vertex: bool,
    pub start_recording: bool,

    // Recording commands
    pub recording_save: bool,

    // Text input commands
    pub text_copy: bool,
    pub text_cut: bool,
    pub text_paste: bool,
    pub text_select_all: bool,
    pub text_undo: bool,
    pub text_redo: bool,

    // Direction setting commands
    pub set_dir_north: bool,
    pub set_dir_south: bool,
    pub set_dir_east: bool,
    pub set_dir_west: bool,
    pub set_dir_up: bool,
    pub set_dir_down: bool,

    // Mouse/touch zoom (not from keybindings)
    pub scroll_zoom: Option<f32>,
    pub pinch_zoom: Option<f32>,
}

/// Capture all keyboard commands for the current frame
pub fn capture_keyboard_commands(
    ctx: &egui::Context,
    keybindings: &KeybindingResolver,
    kb_context: CmdContext,
) -> CapturedCommands {
    let mut cmds = CapturedCommands::default();

    // Global commands
    cmds.close_modal = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalCloseModal, i));
    cmds.toggle_fullscreen = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleFullscreen, i));
    cmds.zoom_in = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalZoomIn, i));
    cmds.zoom_out = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalZoomOut, i));
    cmds.zoom_reset = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalZoomReset, i));
    cmds.toggle_bag = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleBag, i));
    cmds.toggle_nav_panel = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleNavPanel, i));
    cmds.open_command_bar = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalOpenCommandBar, i));
    cmds.open_keybindings = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalOpenKeybindings, i));
    cmds.toggle_tts = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleTTS, i));
    cmds.refresh = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalRefresh, i));
    cmds.copy_url = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalCopyUrl, i));
    cmds.focus_url_down = ctx.input(|i| keybindings.command_down(kb_context, &Command::GlobalFocusUrl, i));
    cmds.focus_url_released = ctx.input(|i| keybindings.command_released(kb_context, &Command::GlobalFocusUrl, i));

    // Graph commands
    cmds.click_vertex = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphClickVertex, i));
    cmds.yank = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphYank, i));
    cmds.bag_pop = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::BagPop, i));
    cmds.go_to_bag_top = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphGoToBagTop, i));
    cmds.paste = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphPaste, i));
    cmds.cut_edge = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphCutEdge, i));
    cmds.delete_vertex = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphDeleteVertex, i));
    cmds.edit_text = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphEditText, i));
    cmds.new_text_vertex = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphNewTextVertex, i));
    cmds.start_recording = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphStartRecording, i));

    // Recording commands
    cmds.recording_save = ctx.input(|i| keybindings.command_released(kb_context, &Command::RecordingSave, i));

    // Text input commands
    cmds.text_copy = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputCopy, i));
    cmds.text_cut = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputCut, i));
    cmds.text_paste = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputPaste, i));
    cmds.text_select_all = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputSelectAll, i));
    cmds.text_undo = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputUndo, i));
    cmds.text_redo = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::TextInputRedo, i));

    // Direction setting commands
    cmds.set_dir_north = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionNorth, i));
    cmds.set_dir_south = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionSouth, i));
    cmds.set_dir_east = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionEast, i));
    cmds.set_dir_west = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionWest, i));
    cmds.set_dir_up = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionUp, i));
    cmds.set_dir_down = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GraphSetDirectionDown, i));

    // Mouse wheel and pinch zoom (not bound to keybindings - these are mouse/touch gestures)
    let (scroll_zoom, pinch_zoom) = ctx.input(|i| {
        let scroll = if i.modifiers.ctrl && i.raw_scroll_delta.y != 0.0 {
            Some(i.raw_scroll_delta.y * 0.001)
        } else {
            None
        };
        let pinch = if i.zoom_delta() != 1.0 {
            Some(i.zoom_delta())
        } else {
            None
        };
        (scroll, pinch)
    });
    cmds.scroll_zoom = scroll_zoom;
    cmds.pinch_zoom = pinch_zoom;

    cmds
}

/// Log triggered commands to the debug log
/// Call this after capture_keyboard_commands with mutable app_state access
pub fn log_triggered_commands_to_debug(
    cmds: &CapturedCommands,
    app_state: &mut AppState,
) {
    // Helper function to get key binding string for a command
    fn get_key(keybindings: &KeybindingResolver, cmd: &Command) -> Option<String> {
        let bindings = keybindings.get_bindings(cmd);
        if bindings.is_empty() {
            None
        } else {
            Some(bindings[0].to_string())
        }
    }

    // Build list of triggered commands
    let mut triggered: Vec<Command> = Vec::new();

    if cmds.close_modal { triggered.push(Command::GlobalCloseModal); }
    if cmds.toggle_fullscreen { triggered.push(Command::GlobalToggleFullscreen); }
    if cmds.zoom_in { triggered.push(Command::GlobalZoomIn); }
    if cmds.zoom_out { triggered.push(Command::GlobalZoomOut); }
    if cmds.zoom_reset { triggered.push(Command::GlobalZoomReset); }
    if cmds.toggle_bag { triggered.push(Command::GlobalToggleBag); }
    if cmds.toggle_nav_panel { triggered.push(Command::GlobalToggleNavPanel); }
    if cmds.open_command_bar { triggered.push(Command::GlobalOpenCommandBar); }
    if cmds.open_keybindings { triggered.push(Command::GlobalOpenKeybindings); }
    if cmds.toggle_tts { triggered.push(Command::GlobalToggleTTS); }
    if cmds.refresh { triggered.push(Command::GlobalRefresh); }
    if cmds.copy_url { triggered.push(Command::GlobalCopyUrl); }
    // focus_url_down is logged only on first press, not while held
    // (handled separately since it uses command_down, not command_pressed)
    if cmds.click_vertex { triggered.push(Command::GraphClickVertex); }
    if cmds.yank { triggered.push(Command::GraphYank); }
    if cmds.bag_pop { triggered.push(Command::BagPop); }
    if cmds.go_to_bag_top { triggered.push(Command::GraphGoToBagTop); }
    if cmds.paste { triggered.push(Command::GraphPaste); }
    if cmds.cut_edge { triggered.push(Command::GraphCutEdge); }
    if cmds.delete_vertex { triggered.push(Command::GraphDeleteVertex); }
    if cmds.edit_text { triggered.push(Command::GraphEditText); }
    if cmds.new_text_vertex { triggered.push(Command::GraphNewTextVertex); }
    if cmds.start_recording { triggered.push(Command::GraphStartRecording); }
    if cmds.recording_save { triggered.push(Command::RecordingSave); }
    if cmds.text_copy { triggered.push(Command::TextInputCopy); }
    if cmds.text_cut { triggered.push(Command::TextInputCut); }
    if cmds.text_paste { triggered.push(Command::TextInputPaste); }
    if cmds.text_select_all { triggered.push(Command::TextInputSelectAll); }
    if cmds.text_undo { triggered.push(Command::TextInputUndo); }
    if cmds.text_redo { triggered.push(Command::TextInputRedo); }
    if cmds.set_dir_north { triggered.push(Command::GraphSetDirectionNorth); }
    if cmds.set_dir_south { triggered.push(Command::GraphSetDirectionSouth); }
    if cmds.set_dir_east { triggered.push(Command::GraphSetDirectionEast); }
    if cmds.set_dir_west { triggered.push(Command::GraphSetDirectionWest); }
    if cmds.set_dir_up { triggered.push(Command::GraphSetDirectionUp); }
    if cmds.set_dir_down { triggered.push(Command::GraphSetDirectionDown); }

    // Collect slug and key info before mutating app_state
    let to_log: Vec<(&str, Option<String>)> = triggered
        .iter()
        .map(|cmd| (cmd.slug(), get_key(&app_state.keybindings, cmd)))
        .collect();

    // Now log all collected commands using official slugs
    for (slug, key_info) in to_log {
        debug_log::log_command_triggered(app_state, slug, key_info.as_deref());
    }
}

