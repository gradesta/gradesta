//! Keyboard and gamepad input capture for UI system
//!
//! Pre-computes which commands were triggered this frame.

use std::collections::HashSet;

use bevy_egui::egui;

use crate::commands::{Command, Context as CmdContext};
use crate::debug_log;
use crate::gamepad::GamepadSnapshot;
use crate::keybindings::key::GamepadKey;
use crate::keybindings::KeybindingResolver;
use crate::state::AppState;

/// All captured commands for a single frame
#[derive(Clone, Debug, Default)]
pub struct CapturedCommands {
    /// Set of commands triggered this frame
    pub commands: HashSet<Command>,

    /// Mouse/touch zoom delta (not from keybindings)
    pub scroll_zoom: Option<f32>,
    pub pinch_zoom: Option<f32>,

    /// Special state: focus_url key is held down (not just pressed)
    pub focus_url_down: bool,
    /// Special state: focus_url key was released this frame
    pub focus_url_released: bool,

    // Voice command UI state (not general commands)
    pub voice_command_start: bool,  // L2+R2 held long enough this frame
    pub voice_command_stop: bool,   // L2 or R2 released while in voice command mode
    pub voice_command_cancel_burst: bool, // L2 released < 1 sec = cancel voice + trigger burst
    pub voice_select_up: bool,      // Right stick up
    pub voice_select_down: bool,    // Right stick down
    pub voice_confirm: bool,        // Right stick click or A button
    pub voice_cancel: bool,         // B button

    // Permission dialog navigation (right stick X-axis + L3)
    pub permission_select_left: bool,   // Right stick left
    pub permission_select_right: bool,  // Right stick right
    pub permission_confirm: bool,       // Right stick click (R3)

    // Context menu navigation (right stick + face buttons)
    pub context_menu_up: bool,
    pub context_menu_down: bool,
    pub context_menu_left: bool,
    pub context_menu_right: bool,
    pub context_menu_select: bool,   // A button / Cross
    pub context_menu_back: bool,     // B button / Circle
}

impl CapturedCommands {
    /// Check if a command was triggered
    #[inline]
    pub fn has(&self, cmd: Command) -> bool {
        self.commands.contains(&cmd)
    }

    /// Add a command
    #[inline]
    pub fn add(&mut self, cmd: Command) {
        self.commands.insert(cmd);
    }
}

/// Capture all keyboard commands for the current frame
pub fn capture_keyboard_commands(
    ctx: &egui::Context,
    keybindings: &KeybindingResolver,
    kb_context: CmdContext,
) -> CapturedCommands {
    let mut cmds = CapturedCommands::default();

    // All commands to check for pressed state
    let commands_to_check = [
        // Global commands
        Command::GlobalCloseModal,
        Command::GlobalToggleFullscreen,
        Command::GlobalZoomIn,
        Command::GlobalZoomOut,
        Command::GlobalZoomReset,
        Command::GlobalToggleBag,
        Command::GlobalToggleNavPanel,
        Command::GlobalToggleElfPanel,
        Command::GlobalOpenCommandBar,
        Command::GlobalOpenKeybindings,
        Command::GlobalToggleTTS,
        Command::GlobalToggleGamepadHelp,
        Command::GlobalToggleVoiceSettings,
        Command::GlobalToggleDebugPanel,
        Command::GlobalToggleIdentityPanel,
        Command::GlobalRefresh,
        Command::GlobalCopyUrl,
        Command::GlobalPlaybackSpeedBoost,
        // Graph commands
        Command::GraphClickVertex,
        Command::GraphYank,
        Command::BagPop,
        Command::GraphGoToBagTop,
        Command::GraphPaste,
        Command::GraphCutEdge,
        Command::GraphDeleteVertex,
        Command::GraphEditText,
        Command::GraphNewTextVertex,
        Command::GraphStartRecording,
        // Note: Navigation commands (GraphNavigate*, GraphHistoryBack) are NOT captured here
        // for keyboard input. Keyboard navigation is handled by `handle_navigation` in main.rs
        // which uses raw keyboard input with key repeat logic. Gamepad navigation uses
        // capture_gamepad_commands which adds these commands to CapturedCommands for
        // execute_commands to process. Voice commands also add navigation via scripts.
        // Direction setting
        Command::GraphSetDirectionNorth,
        Command::GraphSetDirectionSouth,
        Command::GraphSetDirectionEast,
        Command::GraphSetDirectionWest,
        Command::GraphSetDirectionUp,
        Command::GraphSetDirectionDown,
        // Text input commands
        Command::TextInputCopy,
        Command::TextInputCut,
        Command::TextInputPaste,
        Command::TextInputSelectAll,
        Command::TextInputUndo,
        Command::TextInputRedo,
        Command::TextInputSubmit,
    ];

    // Check all commands
    for cmd in commands_to_check {
        if ctx.input(|i| keybindings.command_pressed(kb_context, &cmd, i)) {
            cmds.add(cmd);
        }
    }

    // Special: focus_url tracks down/released state separately
    cmds.focus_url_down = ctx.input(|i| keybindings.command_down(kb_context, &Command::GlobalFocusUrl, i));
    cmds.focus_url_released = ctx.input(|i| keybindings.command_released(kb_context, &Command::GlobalFocusUrl, i));

    // Recording save uses released, not pressed
    if ctx.input(|i| keybindings.command_released(kb_context, &Command::RecordingSave, i)) {
        cmds.add(Command::RecordingSave);
    }

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

    // Helper to check and add command
    let mut check = |cmd: Command| {
        if keybindings.command_pressed_gamepad(&cmd, gp) {
            cmds.add(cmd);
        }
    };

    // Note: Navigation commands (GraphNavigate*, GraphHistoryBack) are NOT captured here.
    // Gamepad navigation is handled by `handle_navigation` in main.rs which has proper
    // key repeat logic. Voice commands can still use navigation via scripts.

    // Actions
    check(Command::GraphClickVertex);
    check(Command::GraphDeleteVertex);
    check(Command::GraphEditText);
    check(Command::GraphNewTextVertex);
    check(Command::GraphYank);
    check(Command::GlobalToggleBag);
    check(Command::GlobalToggleGamepadHelp);
    check(Command::GlobalOpenContextMenu);

    // Recording - pressed to start, released to save
    check(Command::GraphStartRecording);
    if keybindings.command_released_gamepad(&Command::RecordingSave, gp) {
        cmds.add(Command::RecordingSave);
    }

    // Note: Playback speed boost (L2) is handled in capture_voice_command_gamepad
    // because L2 is dual-purpose: tap = boost, hold = voice command
}

/// Capture text input mode gamepad commands (Circle to cancel)
/// Call this when in TextInput mode to allow gamepad escape
pub fn capture_text_input_gamepad(
    cmds: &mut CapturedCommands,
    gamepad: &Option<GamepadSnapshot>,
) {
    let Some(gp) = gamepad else { return };

    // Circle/B button cancels text input
    if gp.is_pressed(GamepadKey::East) {
        cmds.add(Command::TextInputCancel);
    }

    // Cross/A button submits text input
    if gp.is_pressed(GamepadKey::South) {
        cmds.add(Command::TextInputSubmit);
    }
}

use std::time::{Duration, Instant};

/// Duration L2 must be held before voice command mode activates
const L2_HOLD_THRESHOLD: Duration = Duration::from_millis(100);

/// Duration voice command must be held to count as a real command (not a burst)
const VOICE_COMMAND_MIN_DURATION: Duration = Duration::from_secs(1);

/// Capture voice command gamepad inputs (L2 hold for voice command mode)
/// L2 tap = playback speed boost, L2 hold (100ms+) = voice command
/// L2 release < 1 sec while recording = cancel voice + burst
/// This needs to be called separately with access to the current input mode
pub fn capture_voice_command_gamepad(
    cmds: &mut CapturedCommands,
    gamepad: &Option<GamepadSnapshot>,
    is_in_voice_command_mode: bool,
    l2_press_start: &mut Option<Instant>,
    recording_start: Option<Instant>,
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
            // In voice command mode: check if it was held long enough
            if let Some(start) = recording_start {
                if start.elapsed() < VOICE_COMMAND_MIN_DURATION {
                    // Too short - cancel voice command and trigger burst instead
                    cmds.voice_command_cancel_burst = true;
                } else {
                    // Long enough - finalize the voice command
                    cmds.voice_command_stop = true;
                }
            } else {
                // No recording start time - just stop
                cmds.voice_command_stop = true;
            }
        } else if let Some(start) = *l2_press_start {
            // Not in voice command mode: check if it was a quick tap
            if start.elapsed() < L2_HOLD_THRESHOLD {
                // Quick tap = playback speed boost
                cmds.add(Command::GlobalPlaybackSpeedBoost);
            }
        }
        *l2_press_start = None;
    }

    // Right stick for selection (during voice command mode)
    const STICK_DEADZONE: f32 = 0.5;
    let (_rx, ry) = gp.right_stick;

    // Y axis: positive = up (move selection up), negative = down (move selection down)
    cmds.voice_select_up = ry > STICK_DEADZONE;
    cmds.voice_select_down = ry < -STICK_DEADZONE;

    // R3 (right stick click) or A button to confirm selection
    cmds.voice_confirm = gp.is_pressed(GamepadKey::RightStick); // R3
    cmds.voice_confirm |= gp.is_pressed(GamepadKey::South); // A / Cross

    // B button to cancel (legacy, Cancel is now a menu option)
    cmds.voice_cancel = gp.is_pressed(GamepadKey::East); // B / Circle

    // Right stick X-axis for permission dialog button selection (when not in voice command mode)
    let (rx, _ry) = gp.right_stick;
    cmds.permission_select_left = rx < -STICK_DEADZONE;
    cmds.permission_select_right = rx > STICK_DEADZONE;

    // R3 (right stick click) to confirm permission dialog selection
    cmds.permission_confirm = gp.is_pressed(GamepadKey::RightStick);
}

/// Capture context menu gamepad inputs (right stick + face buttons)
/// Only captures when context menu is open
pub fn capture_context_menu_gamepad(
    cmds: &mut CapturedCommands,
    gamepad: &Option<GamepadSnapshot>,
    context_menu_open: bool,
) {
    let Some(gp) = gamepad else { return };

    if !context_menu_open {
        return;
    }

    const STICK_DEADZONE: f32 = 0.5;
    let (rx, ry) = gp.right_stick;

    // Right stick for navigation
    cmds.context_menu_up = ry > STICK_DEADZONE;
    cmds.context_menu_down = ry < -STICK_DEADZONE;
    cmds.context_menu_left = rx < -STICK_DEADZONE;
    cmds.context_menu_right = rx > STICK_DEADZONE;

    // R3 (right stick click) to select
    cmds.context_menu_select = gp.is_pressed(GamepadKey::RightStick);

    // ○ (Circle) to go back
    cmds.context_menu_back = gp.is_pressed(GamepadKey::East);
}

/// Sidebar gamepad input state
#[derive(Clone, Debug, Default)]
pub struct SidebarGamepadInput {
    /// Right stick Y > 0.5 (move selection up)
    pub nav_up: bool,
    /// Right stick Y < -0.5 (move selection down)
    pub nav_down: bool,
    /// Right stick X > 0.5 (move right / next section)
    pub nav_right: bool,
    /// Right stick X < -0.5 (move left / prev section)
    pub nav_left: bool,
    /// R3 pressed (select/activate item)
    pub select: bool,
    /// Circle pressed (close panel / go back)
    pub back: bool,
    /// Cross pressed (alternative select)
    pub cross: bool,
}

/// Capture sidebar-specific gamepad inputs
/// Only call when a sidebar panel is open
pub fn capture_sidebar_gamepad(
    gamepad: &Option<GamepadSnapshot>,
) -> SidebarGamepadInput {
    let mut input = SidebarGamepadInput::default();

    let Some(gp) = gamepad else { return input };

    const STICK_DEADZONE: f32 = 0.5;
    let (rx, ry) = gp.right_stick;

    // Right stick for navigation
    input.nav_up = ry > STICK_DEADZONE;
    input.nav_down = ry < -STICK_DEADZONE;
    input.nav_right = rx > STICK_DEADZONE;
    input.nav_left = rx < -STICK_DEADZONE;

    // R3 (right stick click) to select
    input.select = gp.is_pressed(GamepadKey::RightStick);

    // Cross (A/South) also selects
    input.cross = gp.is_pressed(GamepadKey::South);

    // Circle (B/East) to go back/close
    input.back = gp.is_pressed(GamepadKey::East);

    input
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

    // Collect slug and key info before mutating app_state
    let to_log: Vec<(&str, Option<String>)> = cmds.commands
        .iter()
        .map(|cmd| (cmd.slug(), get_key(&app_state.keybindings, cmd)))
        .collect();

    // Now log all collected commands using official slugs
    for (slug, key_info) in to_log {
        debug_log::log_command_triggered(app_state, slug, key_info.as_deref());
    }
}

