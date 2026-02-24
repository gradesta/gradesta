//! Default keybindings

use crate::commands::{Command, Context};
use crate::keybindings::key::{GamepadKey, KeyBinding, KeyCode, Modifiers};

/// A default keybinding definition
pub struct DefaultBinding {
    pub command: Command,
    pub context: Context,
    pub bindings: &'static [(KeyCode, Modifiers)],
}

// Modifier constants for readability
const NONE: Modifiers = Modifiers { ctrl: false, shift: false, alt: false };
const CTRL: Modifiers = Modifiers { ctrl: true, shift: false, alt: false };
const SHIFT: Modifiers = Modifiers { ctrl: false, shift: true, alt: false };
const CTRL_SHIFT: Modifiers = Modifiers { ctrl: true, shift: true, alt: false };

/// Get all keybindings for a command
pub fn get_default_bindings(command: &Command) -> Vec<KeyBinding> {
    DEFAULTS
        .iter()
        .filter(|d| &d.command == command)
        .flat_map(|d| {
            d.bindings.iter().map(|(key, mods)| KeyBinding {
                key: *key,
                modifiers: *mods,
            })
        })
        .collect()
}

/// Get all default bindings
pub fn all_defaults() -> impl Iterator<Item = (Command, Context, KeyBinding)> {
    DEFAULTS.iter().flat_map(|d| {
        d.bindings.iter().map(move |(key, mods)| {
            (d.command.clone(), d.context, KeyBinding {
                key: *key,
                modifiers: *mods,
            })
        })
    })
}

// All default keybindings
static DEFAULTS: &[DefaultBinding] = &[
    // === Global Commands ===
    DefaultBinding {
        command: Command::GlobalFocusUrl,
        context: Context::Global,
        bindings: &[(KeyCode::L, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalZoomIn,
        context: Context::Global,
        bindings: &[(KeyCode::Plus, CTRL), (KeyCode::Equals, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalZoomOut,
        context: Context::Global,
        bindings: &[(KeyCode::Minus, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalZoomReset,
        context: Context::Global,
        bindings: &[(KeyCode::Num0, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalToggleBag,
        context: Context::Global,
        bindings: &[(KeyCode::B, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalToggleNavPanel,
        context: Context::Global,
        bindings: &[(KeyCode::N, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalToggleElfPanel,
        context: Context::Global,
        bindings: &[(KeyCode::E, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalCloseModal,
        context: Context::Global,
        bindings: &[(KeyCode::Escape, NONE)],
    },
    DefaultBinding {
        command: Command::GlobalCopyUrl,
        context: Context::Global,
        bindings: &[(KeyCode::C, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalRefresh,
        context: Context::Global,
        bindings: &[(KeyCode::F5, NONE)],
    },
    DefaultBinding {
        command: Command::GlobalToggleFullscreen,
        context: Context::Global,
        bindings: &[(KeyCode::Enter, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalOpenCommandBar,
        context: Context::Global,
        bindings: &[(KeyCode::Colon, NONE)],
    },
    DefaultBinding {
        command: Command::GlobalOpenKeybindings,
        context: Context::Global,
        bindings: &[(KeyCode::K, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalToggleTTS,
        context: Context::Global,
        bindings: &[(KeyCode::T, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalToggleGamepadHelp,
        context: Context::Global,
        bindings: &[(KeyCode::G, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalTTSSpeedUp,
        context: Context::Global,
        bindings: &[(KeyCode::BracketRight, CTRL)],
    },
    DefaultBinding {
        command: Command::GlobalTTSSpeedDown,
        context: Context::Global,
        bindings: &[(KeyCode::BracketLeft, CTRL)],
    },

    // === Graph Navigation ===
    DefaultBinding {
        command: Command::GraphNavigateNorth,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowUp, NONE), (KeyCode::W, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNavigateSouth,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowDown, NONE), (KeyCode::S, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNavigateEast,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowRight, NONE), (KeyCode::D, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNavigateWest,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowLeft, NONE), (KeyCode::A, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNavigateUp,
        context: Context::Graph,
        bindings: &[(KeyCode::PageUp, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNavigateDown,
        context: Context::Graph,
        bindings: &[(KeyCode::PageDown, NONE)],
    },
    DefaultBinding {
        command: Command::GraphHistoryBack,
        context: Context::Graph,
        bindings: &[(KeyCode::Backspace, NONE)],
    },

    // === Graph Actions ===
    DefaultBinding {
        command: Command::GraphClickVertex,
        context: Context::Graph,
        bindings: &[(KeyCode::Enter, NONE)],
    },
    DefaultBinding {
        command: Command::GraphEditText,
        context: Context::Graph,
        bindings: &[(KeyCode::I, NONE)],
    },
    DefaultBinding {
        command: Command::GraphNewTextVertex,
        context: Context::Graph,
        bindings: &[(KeyCode::N, NONE)],
    },
    DefaultBinding {
        command: Command::GraphStartRecording,
        context: Context::Graph,
        bindings: &[(KeyCode::Space, NONE)],
    },
    DefaultBinding {
        command: Command::GraphYank,
        context: Context::Graph,
        bindings: &[(KeyCode::Y, NONE)],
    },
    DefaultBinding {
        command: Command::GraphPaste,
        context: Context::Graph,
        bindings: &[(KeyCode::P, NONE)],
    },
    DefaultBinding {
        command: Command::GraphCutEdge,
        context: Context::Graph,
        bindings: &[(KeyCode::C, NONE)],
    },
    DefaultBinding {
        command: Command::GraphGoToBagTop,
        context: Context::Graph,
        bindings: &[(KeyCode::G, NONE)],
    },
    DefaultBinding {
        command: Command::GraphDeleteVertex,
        context: Context::Graph,
        bindings: &[(KeyCode::Delete, NONE)],
    },

    // === Set Direction (Shift+Arrow) ===
    DefaultBinding {
        command: Command::GraphSetDirectionNorth,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowUp, SHIFT), (KeyCode::W, SHIFT)],
    },
    DefaultBinding {
        command: Command::GraphSetDirectionSouth,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowDown, SHIFT), (KeyCode::S, SHIFT)],
    },
    DefaultBinding {
        command: Command::GraphSetDirectionEast,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowRight, SHIFT), (KeyCode::D, SHIFT)],
    },
    DefaultBinding {
        command: Command::GraphSetDirectionWest,
        context: Context::Graph,
        bindings: &[(KeyCode::ArrowLeft, SHIFT), (KeyCode::A, SHIFT)],
    },
    DefaultBinding {
        command: Command::GraphSetDirectionUp,
        context: Context::Graph,
        bindings: &[(KeyCode::PageUp, SHIFT)],
    },
    DefaultBinding {
        command: Command::GraphSetDirectionDown,
        context: Context::Graph,
        bindings: &[(KeyCode::PageDown, SHIFT)],
    },

    // === Bag Commands ===
    DefaultBinding {
        command: Command::BagPop,
        context: Context::Global,
        bindings: &[(KeyCode::Y, CTRL)],
    },
    DefaultBinding {
        command: Command::BagClear,
        context: Context::Bag,
        bindings: &[(KeyCode::C, CTRL_SHIFT)],
    },

    // === Text Input Commands ===
    DefaultBinding {
        command: Command::TextInputSubmit,
        context: Context::TextInput,
        bindings: &[(KeyCode::Enter, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputCancel,
        context: Context::TextInput,
        bindings: &[(KeyCode::Escape, NONE)],
    },
    DefaultBinding {
        command: Command::TextInputCopy,
        context: Context::TextInput,
        bindings: &[(KeyCode::C, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputCut,
        context: Context::TextInput,
        bindings: &[(KeyCode::X, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputPaste,
        context: Context::TextInput,
        bindings: &[(KeyCode::V, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputSelectAll,
        context: Context::TextInput,
        bindings: &[(KeyCode::A, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputUndo,
        context: Context::TextInput,
        bindings: &[(KeyCode::Z, CTRL)],
    },
    DefaultBinding {
        command: Command::TextInputRedo,
        context: Context::TextInput,
        bindings: &[(KeyCode::Z, CTRL_SHIFT), (KeyCode::Y, CTRL)],
    },

    // === Recording Commands ===
    DefaultBinding {
        command: Command::RecordingSave,
        context: Context::Recording,
        bindings: &[(KeyCode::Space, NONE)],
    },
    DefaultBinding {
        command: Command::RecordingCancel,
        context: Context::Recording,
        bindings: &[(KeyCode::Escape, NONE)],
    },

    // === Authentication Commands ===
    DefaultBinding {
        command: Command::AuthCycleIdentity,
        context: Context::Authentication,
        bindings: &[(KeyCode::Tab, NONE)],
    },
    DefaultBinding {
        command: Command::AuthAccept,
        context: Context::Authentication,
        bindings: &[(KeyCode::Enter, NONE)],
    },
    DefaultBinding {
        command: Command::AuthAcceptRemember,
        context: Context::Authentication,
        bindings: &[(KeyCode::Enter, CTRL)],
    },
    DefaultBinding {
        command: Command::AuthRefuse,
        context: Context::Authentication,
        bindings: &[(KeyCode::Escape, NONE)],
    },
];

/// Default gamepad bindings
pub static GAMEPAD_DEFAULTS: &[(Command, GamepadKey)] = &[
    // Navigation - D-pad
    (Command::GraphNavigateNorth, GamepadKey::DPadUp),
    (Command::GraphNavigateSouth, GamepadKey::DPadDown),
    (Command::GraphNavigateWest, GamepadKey::DPadLeft),
    (Command::GraphNavigateEast, GamepadKey::DPadRight),
    // Navigation - Left stick
    (Command::GraphNavigateNorth, GamepadKey::LeftStickUp),
    (Command::GraphNavigateSouth, GamepadKey::LeftStickDown),
    (Command::GraphNavigateWest, GamepadKey::LeftStickLeft),
    (Command::GraphNavigateEast, GamepadKey::LeftStickRight),
    // Layer navigation
    (Command::GraphNavigateUp, GamepadKey::LeftBumper),
    (Command::GraphNavigateDown, GamepadKey::RightBumper),

    // Actions - Face buttons
    (Command::GraphDeleteVertex, GamepadKey::South),     // Cross = Delete
    (Command::GraphClickVertex, GamepadKey::East),       // Circle = Click/Enter
    (Command::GraphEditText, GamepadKey::West),          // Square = Edit text
    (Command::GraphNewTextVertex, GamepadKey::North),    // Triangle = New text

    // Recording - R2 (hold to record, release to save)
    (Command::GraphStartRecording, GamepadKey::RightTrigger),
    (Command::RecordingSave, GamepadKey::RightTrigger),  // Same button - release triggers save

    // Yank/Paste - L2
    (Command::GraphYank, GamepadKey::LeftTrigger),

    // UI toggles
    (Command::GlobalToggleBag, GamepadKey::Start),
    (Command::GraphHistoryBack, GamepadKey::Select),

    // Help overlay - L3 (left stick press)
    (Command::GlobalToggleGamepadHelp, GamepadKey::LeftStick),
];
