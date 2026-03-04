//! Keyboard and gamepad input capture for UI system
//!
//! Pre-computes which commands were triggered this frame.

use bevy_egui::egui;

use crate::commands::{Command, Context as CmdContext};
use crate::debug_log;
use crate::gamepad::GamepadSnapshot;
use crate::keybindings::key::GamepadKey;
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
    pub toggle_elf_panel: bool,
    pub open_command_bar: bool,
    pub open_keybindings: bool,
    pub toggle_tts: bool,
    pub toggle_gamepad_help: bool,
    pub toggle_voice_settings: bool,
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

    // Playback speed boost
    pub playback_speed_boost: bool,

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

    // Navigation commands (for gamepad support)
    pub nav_north: bool,
    pub nav_south: bool,
    pub nav_east: bool,
    pub nav_west: bool,
    pub nav_up: bool,
    pub nav_down: bool,
    pub history_back: bool,

    // Voice command inputs (gamepad L2+R2)
    pub voice_command_start: bool,  // L2+R2 both pressed this frame
    pub voice_command_stop: bool,   // L2 or R2 released while in voice command mode
    pub voice_select_up: bool,      // Right stick up
    pub voice_select_down: bool,    // Right stick down
    pub voice_confirm: bool,        // Right stick right (or A button)
    pub voice_cancel: bool,         // Right stick left (or B button)
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
    cmds.toggle_elf_panel = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleElfPanel, i));
    cmds.open_command_bar = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalOpenCommandBar, i));
    cmds.open_keybindings = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalOpenKeybindings, i));
    cmds.toggle_tts = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleTTS, i));
    cmds.toggle_gamepad_help = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleGamepadHelp, i));
    cmds.toggle_voice_settings = ctx.input(|i| keybindings.command_pressed(kb_context, &Command::GlobalToggleVoiceSettings, i));
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

/// Merge gamepad commands into captured commands
/// Call this after capture_keyboard_commands to add gamepad input
pub fn capture_gamepad_commands(
    cmds: &mut CapturedCommands,
    gamepad: &Option<GamepadSnapshot>,
    keybindings: &KeybindingResolver,
) {
    let Some(gp) = gamepad else { return };

    // Navigation
    cmds.nav_north |= keybindings.command_pressed_gamepad(&Command::GraphNavigateNorth, gp);
    cmds.nav_south |= keybindings.command_pressed_gamepad(&Command::GraphNavigateSouth, gp);
    cmds.nav_east |= keybindings.command_pressed_gamepad(&Command::GraphNavigateEast, gp);
    cmds.nav_west |= keybindings.command_pressed_gamepad(&Command::GraphNavigateWest, gp);
    cmds.nav_up |= keybindings.command_pressed_gamepad(&Command::GraphNavigateUp, gp);
    cmds.nav_down |= keybindings.command_pressed_gamepad(&Command::GraphNavigateDown, gp);
    cmds.history_back |= keybindings.command_pressed_gamepad(&Command::GraphHistoryBack, gp);

    // Actions
    cmds.click_vertex |= keybindings.command_pressed_gamepad(&Command::GraphClickVertex, gp);
    cmds.delete_vertex |= keybindings.command_pressed_gamepad(&Command::GraphDeleteVertex, gp);
    cmds.edit_text |= keybindings.command_pressed_gamepad(&Command::GraphEditText, gp);
    cmds.new_text_vertex |= keybindings.command_pressed_gamepad(&Command::GraphNewTextVertex, gp);
    cmds.yank |= keybindings.command_pressed_gamepad(&Command::GraphYank, gp);
    cmds.toggle_bag |= keybindings.command_pressed_gamepad(&Command::GlobalToggleBag, gp);
    cmds.toggle_gamepad_help |= keybindings.command_pressed_gamepad(&Command::GlobalToggleGamepadHelp, gp);

    // Recording - R2 hold to record, release to save (matches keyboard Space behavior)
    cmds.start_recording |= keybindings.command_pressed_gamepad(&Command::GraphStartRecording, gp);
    cmds.recording_save |= keybindings.command_released_gamepad(&Command::RecordingSave, gp);

    // Note: Playback speed boost (L2) is handled in capture_voice_command_gamepad
    // because L2 is dual-purpose: tap = boost, hold = voice command
}

use std::time::{Duration, Instant};

/// Duration L2 must be held before voice command mode activates
const L2_HOLD_THRESHOLD: Duration = Duration::from_millis(100);

/// Capture voice command gamepad inputs (L2 hold for voice command mode)
/// L2 tap = playback speed boost, L2 hold (100ms+) = voice command
/// This needs to be called separately with access to the current input mode
pub fn capture_voice_command_gamepad(
    cmds: &mut CapturedCommands,
    gamepad: &Option<GamepadSnapshot>,
    is_in_voice_command_mode: bool,
    l2_press_start: &mut Option<Instant>,
) {
    let Some(gp) = gamepad else { return };

    let l2_held = gp.is_held(GamepadKey::LeftTrigger);
    let l2_pressed = gp.is_pressed(GamepadKey::LeftTrigger);
    let l2_released = gp.is_released(GamepadKey::LeftTrigger);

    // Track when L2 was first pressed
    if l2_pressed {
        *l2_press_start = Some(Instant::now());
    }

    // Check if L2 has been held long enough for voice command
    // Once triggered, clear l2_press_start to avoid triggering every frame
    if l2_held && !is_in_voice_command_mode {
        if let Some(start) = *l2_press_start {
            if start.elapsed() >= L2_HOLD_THRESHOLD {
                cmds.voice_command_start = true;
                // Clear to prevent re-triggering every frame
                *l2_press_start = None;
            }
        }
    }

    // When L2 is released
    if l2_released {
        if is_in_voice_command_mode {
            // In voice command mode: stop recording
            cmds.voice_command_stop = true;
        } else if let Some(start) = *l2_press_start {
            // Not in voice command mode: check if it was a quick tap
            if start.elapsed() < L2_HOLD_THRESHOLD {
                // Quick tap = playback speed boost
                cmds.playback_speed_boost = true;
            }
        }
        *l2_press_start = None;
    }

    // Right stick for selection (during voice command mode)
    const STICK_DEADZONE: f32 = 0.5;
    let (_rx, ry) = gp.right_stick;

    // Y axis: negative = up, positive = down
    cmds.voice_select_up = ry < -STICK_DEADZONE;
    cmds.voice_select_down = ry > STICK_DEADZONE;

    // R3 (right stick click) or A button to confirm selection
    cmds.voice_confirm = gp.is_pressed(GamepadKey::RightStick); // R3
    cmds.voice_confirm |= gp.is_pressed(GamepadKey::South); // A / Cross

    // B button to cancel (legacy, Cancel is now a menu option)
    cmds.voice_cancel = gp.is_pressed(GamepadKey::East); // B / Circle
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
    if cmds.toggle_elf_panel { triggered.push(Command::GlobalToggleElfPanel); }
    if cmds.open_command_bar { triggered.push(Command::GlobalOpenCommandBar); }
    if cmds.open_keybindings { triggered.push(Command::GlobalOpenKeybindings); }
    if cmds.toggle_tts { triggered.push(Command::GlobalToggleTTS); }
    if cmds.toggle_voice_settings { triggered.push(Command::GlobalToggleVoiceSettings); }
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
    if cmds.playback_speed_boost { triggered.push(Command::GlobalPlaybackSpeedBoost); }

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

