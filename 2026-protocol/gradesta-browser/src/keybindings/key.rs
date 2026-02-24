//! Key representation and parsing for keybindings

use bevy_egui::egui;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Key codes supported by the keybinding system
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    // Special keys
    Escape, Enter, Space, Backspace, Tab, Delete,
    // Arrow keys
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    // Page navigation
    PageUp, PageDown, Home, End,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Symbols
    Plus, Minus, Equals, Colon, Semicolon,
    BracketLeft, BracketRight,
    Comma, Period, Slash, Backslash,
    Quote, Backtick,
}

/// Gamepad button codes (PS2-style naming)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadKey {
    // Face buttons (PS2 naming)
    South,      // Cross (X)
    East,       // Circle
    West,       // Square
    North,      // Triangle
    // Shoulder buttons
    LeftBumper,   // L1
    RightBumper,  // R1
    LeftTrigger,  // L2
    RightTrigger, // R2
    // Stick presses
    LeftStick,    // L3
    RightStick,   // R3
    // Special
    Start,
    Select,
    // D-pad
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    // Analog stick directions (virtual buttons from stick position)
    LeftStickUp,
    LeftStickDown,
    LeftStickLeft,
    LeftStickRight,
}

impl GamepadKey {
    /// Parse a gamepad key from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "south" | "cross" | "x" => Ok(GamepadKey::South),
            "east" | "circle" | "o" => Ok(GamepadKey::East),
            "west" | "square" => Ok(GamepadKey::West),
            "north" | "triangle" => Ok(GamepadKey::North),
            "l1" | "leftbumper" => Ok(GamepadKey::LeftBumper),
            "r1" | "rightbumper" => Ok(GamepadKey::RightBumper),
            "l2" | "lefttrigger" => Ok(GamepadKey::LeftTrigger),
            "r2" | "righttrigger" => Ok(GamepadKey::RightTrigger),
            "l3" | "leftstick" => Ok(GamepadKey::LeftStick),
            "r3" | "rightstick" => Ok(GamepadKey::RightStick),
            "start" => Ok(GamepadKey::Start),
            "select" | "back" => Ok(GamepadKey::Select),
            "dpadup" => Ok(GamepadKey::DPadUp),
            "dpaddown" => Ok(GamepadKey::DPadDown),
            "dpadleft" => Ok(GamepadKey::DPadLeft),
            "dpadright" => Ok(GamepadKey::DPadRight),
            "leftstickup" => Ok(GamepadKey::LeftStickUp),
            "leftstickdown" => Ok(GamepadKey::LeftStickDown),
            "leftstickleft" => Ok(GamepadKey::LeftStickLeft),
            "leftstickright" => Ok(GamepadKey::LeftStickRight),
            _ => Err(format!("Unknown gamepad key: {}", s)),
        }
    }

    /// Get the display string for this key
    pub fn as_str(&self) -> &'static str {
        match self {
            GamepadKey::South => "Cross",
            GamepadKey::East => "Circle",
            GamepadKey::West => "Square",
            GamepadKey::North => "Triangle",
            GamepadKey::LeftBumper => "L1",
            GamepadKey::RightBumper => "R1",
            GamepadKey::LeftTrigger => "L2",
            GamepadKey::RightTrigger => "R2",
            GamepadKey::LeftStick => "L3",
            GamepadKey::RightStick => "R3",
            GamepadKey::Start => "Start",
            GamepadKey::Select => "Select",
            GamepadKey::DPadUp => "D-Up",
            GamepadKey::DPadDown => "D-Down",
            GamepadKey::DPadLeft => "D-Left",
            GamepadKey::DPadRight => "D-Right",
            GamepadKey::LeftStickUp => "LS-Up",
            GamepadKey::LeftStickDown => "LS-Down",
            GamepadKey::LeftStickLeft => "LS-Left",
            GamepadKey::LeftStickRight => "LS-Right",
        }
    }
}

impl KeyCode {
    /// Parse a key code from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            // Letters
            "a" => Ok(KeyCode::A),
            "b" => Ok(KeyCode::B),
            "c" => Ok(KeyCode::C),
            "d" => Ok(KeyCode::D),
            "e" => Ok(KeyCode::E),
            "f" => Ok(KeyCode::F),
            "g" => Ok(KeyCode::G),
            "h" => Ok(KeyCode::H),
            "i" => Ok(KeyCode::I),
            "j" => Ok(KeyCode::J),
            "k" => Ok(KeyCode::K),
            "l" => Ok(KeyCode::L),
            "m" => Ok(KeyCode::M),
            "n" => Ok(KeyCode::N),
            "o" => Ok(KeyCode::O),
            "p" => Ok(KeyCode::P),
            "q" => Ok(KeyCode::Q),
            "r" => Ok(KeyCode::R),
            "s" => Ok(KeyCode::S),
            "t" => Ok(KeyCode::T),
            "u" => Ok(KeyCode::U),
            "v" => Ok(KeyCode::V),
            "w" => Ok(KeyCode::W),
            "x" => Ok(KeyCode::X),
            "y" => Ok(KeyCode::Y),
            "z" => Ok(KeyCode::Z),
            // Numbers
            "0" | "num0" => Ok(KeyCode::Num0),
            "1" | "num1" => Ok(KeyCode::Num1),
            "2" | "num2" => Ok(KeyCode::Num2),
            "3" | "num3" => Ok(KeyCode::Num3),
            "4" | "num4" => Ok(KeyCode::Num4),
            "5" | "num5" => Ok(KeyCode::Num5),
            "6" | "num6" => Ok(KeyCode::Num6),
            "7" | "num7" => Ok(KeyCode::Num7),
            "8" | "num8" => Ok(KeyCode::Num8),
            "9" | "num9" => Ok(KeyCode::Num9),
            // Special keys
            "escape" | "esc" => Ok(KeyCode::Escape),
            "enter" | "return" => Ok(KeyCode::Enter),
            "space" => Ok(KeyCode::Space),
            "backspace" => Ok(KeyCode::Backspace),
            "tab" => Ok(KeyCode::Tab),
            "delete" | "del" => Ok(KeyCode::Delete),
            // Arrow keys
            "arrowup" | "up" => Ok(KeyCode::ArrowUp),
            "arrowdown" | "down" => Ok(KeyCode::ArrowDown),
            "arrowleft" | "left" => Ok(KeyCode::ArrowLeft),
            "arrowright" | "right" => Ok(KeyCode::ArrowRight),
            // Page navigation
            "pageup" | "pgup" => Ok(KeyCode::PageUp),
            "pagedown" | "pgdown" => Ok(KeyCode::PageDown),
            "home" => Ok(KeyCode::Home),
            "end" => Ok(KeyCode::End),
            // Function keys
            "f1" => Ok(KeyCode::F1),
            "f2" => Ok(KeyCode::F2),
            "f3" => Ok(KeyCode::F3),
            "f4" => Ok(KeyCode::F4),
            "f5" => Ok(KeyCode::F5),
            "f6" => Ok(KeyCode::F6),
            "f7" => Ok(KeyCode::F7),
            "f8" => Ok(KeyCode::F8),
            "f9" => Ok(KeyCode::F9),
            "f10" => Ok(KeyCode::F10),
            "f11" => Ok(KeyCode::F11),
            "f12" => Ok(KeyCode::F12),
            // Symbols
            "plus" | "+" => Ok(KeyCode::Plus),
            "minus" | "-" => Ok(KeyCode::Minus),
            "equals" | "=" => Ok(KeyCode::Equals),
            "colon" | ":" => Ok(KeyCode::Colon),
            "semicolon" | ";" => Ok(KeyCode::Semicolon),
            "bracketleft" | "[" => Ok(KeyCode::BracketLeft),
            "bracketright" | "]" => Ok(KeyCode::BracketRight),
            "comma" | "," => Ok(KeyCode::Comma),
            "period" | "." => Ok(KeyCode::Period),
            "slash" | "/" => Ok(KeyCode::Slash),
            "backslash" | "\\" => Ok(KeyCode::Backslash),
            "quote" | "'" => Ok(KeyCode::Quote),
            "backtick" | "`" => Ok(KeyCode::Backtick),
            _ => Err(format!("Unknown key: {}", s)),
        }
    }

    /// Get the display string for this key
    pub fn as_str(&self) -> &'static str {
        match self {
            KeyCode::A => "A", KeyCode::B => "B", KeyCode::C => "C", KeyCode::D => "D",
            KeyCode::E => "E", KeyCode::F => "F", KeyCode::G => "G", KeyCode::H => "H",
            KeyCode::I => "I", KeyCode::J => "J", KeyCode::K => "K", KeyCode::L => "L",
            KeyCode::M => "M", KeyCode::N => "N", KeyCode::O => "O", KeyCode::P => "P",
            KeyCode::Q => "Q", KeyCode::R => "R", KeyCode::S => "S", KeyCode::T => "T",
            KeyCode::U => "U", KeyCode::V => "V", KeyCode::W => "W", KeyCode::X => "X",
            KeyCode::Y => "Y", KeyCode::Z => "Z",
            KeyCode::Num0 => "0", KeyCode::Num1 => "1", KeyCode::Num2 => "2",
            KeyCode::Num3 => "3", KeyCode::Num4 => "4", KeyCode::Num5 => "5",
            KeyCode::Num6 => "6", KeyCode::Num7 => "7", KeyCode::Num8 => "8",
            KeyCode::Num9 => "9",
            KeyCode::Escape => "Escape", KeyCode::Enter => "Enter",
            KeyCode::Space => "Space", KeyCode::Backspace => "Backspace",
            KeyCode::Tab => "Tab", KeyCode::Delete => "Delete",
            KeyCode::ArrowUp => "ArrowUp", KeyCode::ArrowDown => "ArrowDown",
            KeyCode::ArrowLeft => "ArrowLeft", KeyCode::ArrowRight => "ArrowRight",
            KeyCode::PageUp => "PageUp", KeyCode::PageDown => "PageDown",
            KeyCode::Home => "Home", KeyCode::End => "End",
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", KeyCode::F3 => "F3",
            KeyCode::F4 => "F4", KeyCode::F5 => "F5", KeyCode::F6 => "F6",
            KeyCode::F7 => "F7", KeyCode::F8 => "F8", KeyCode::F9 => "F9",
            KeyCode::F10 => "F10", KeyCode::F11 => "F11", KeyCode::F12 => "F12",
            KeyCode::Plus => "+", KeyCode::Minus => "-", KeyCode::Equals => "=",
            KeyCode::Colon => ":", KeyCode::Semicolon => ";",
            KeyCode::BracketLeft => "[", KeyCode::BracketRight => "]",
            KeyCode::Comma => ",", KeyCode::Period => ".",
            KeyCode::Slash => "/", KeyCode::Backslash => "\\",
            KeyCode::Quote => "'", KeyCode::Backtick => "`",
        }
    }

    /// Convert from egui::Key
    pub fn from_egui(key: egui::Key) -> Option<Self> {
        match key {
            egui::Key::A => Some(KeyCode::A),
            egui::Key::B => Some(KeyCode::B),
            egui::Key::C => Some(KeyCode::C),
            egui::Key::D => Some(KeyCode::D),
            egui::Key::E => Some(KeyCode::E),
            egui::Key::F => Some(KeyCode::F),
            egui::Key::G => Some(KeyCode::G),
            egui::Key::H => Some(KeyCode::H),
            egui::Key::I => Some(KeyCode::I),
            egui::Key::J => Some(KeyCode::J),
            egui::Key::K => Some(KeyCode::K),
            egui::Key::L => Some(KeyCode::L),
            egui::Key::M => Some(KeyCode::M),
            egui::Key::N => Some(KeyCode::N),
            egui::Key::O => Some(KeyCode::O),
            egui::Key::P => Some(KeyCode::P),
            egui::Key::Q => Some(KeyCode::Q),
            egui::Key::R => Some(KeyCode::R),
            egui::Key::S => Some(KeyCode::S),
            egui::Key::T => Some(KeyCode::T),
            egui::Key::U => Some(KeyCode::U),
            egui::Key::V => Some(KeyCode::V),
            egui::Key::W => Some(KeyCode::W),
            egui::Key::X => Some(KeyCode::X),
            egui::Key::Y => Some(KeyCode::Y),
            egui::Key::Z => Some(KeyCode::Z),
            egui::Key::Num0 => Some(KeyCode::Num0),
            egui::Key::Num1 => Some(KeyCode::Num1),
            egui::Key::Num2 => Some(KeyCode::Num2),
            egui::Key::Num3 => Some(KeyCode::Num3),
            egui::Key::Num4 => Some(KeyCode::Num4),
            egui::Key::Num5 => Some(KeyCode::Num5),
            egui::Key::Num6 => Some(KeyCode::Num6),
            egui::Key::Num7 => Some(KeyCode::Num7),
            egui::Key::Num8 => Some(KeyCode::Num8),
            egui::Key::Num9 => Some(KeyCode::Num9),
            egui::Key::Escape => Some(KeyCode::Escape),
            egui::Key::Enter => Some(KeyCode::Enter),
            egui::Key::Space => Some(KeyCode::Space),
            egui::Key::Backspace => Some(KeyCode::Backspace),
            egui::Key::Tab => Some(KeyCode::Tab),
            egui::Key::Delete => Some(KeyCode::Delete),
            egui::Key::ArrowUp => Some(KeyCode::ArrowUp),
            egui::Key::ArrowDown => Some(KeyCode::ArrowDown),
            egui::Key::ArrowLeft => Some(KeyCode::ArrowLeft),
            egui::Key::ArrowRight => Some(KeyCode::ArrowRight),
            egui::Key::PageUp => Some(KeyCode::PageUp),
            egui::Key::PageDown => Some(KeyCode::PageDown),
            egui::Key::Home => Some(KeyCode::Home),
            egui::Key::End => Some(KeyCode::End),
            egui::Key::F1 => Some(KeyCode::F1),
            egui::Key::F2 => Some(KeyCode::F2),
            egui::Key::F3 => Some(KeyCode::F3),
            egui::Key::F4 => Some(KeyCode::F4),
            egui::Key::F5 => Some(KeyCode::F5),
            egui::Key::F6 => Some(KeyCode::F6),
            egui::Key::F7 => Some(KeyCode::F7),
            egui::Key::F8 => Some(KeyCode::F8),
            egui::Key::F9 => Some(KeyCode::F9),
            egui::Key::F10 => Some(KeyCode::F10),
            egui::Key::F11 => Some(KeyCode::F11),
            egui::Key::F12 => Some(KeyCode::F12),
            egui::Key::Minus => Some(KeyCode::Minus),
            egui::Key::Plus => Some(KeyCode::Plus),
            egui::Key::Equals => Some(KeyCode::Equals),
            egui::Key::Semicolon => Some(KeyCode::Semicolon),
            egui::Key::Colon => Some(KeyCode::Colon),
            egui::Key::OpenBracket => Some(KeyCode::BracketLeft),
            egui::Key::CloseBracket => Some(KeyCode::BracketRight),
            egui::Key::Comma => Some(KeyCode::Comma),
            egui::Key::Period => Some(KeyCode::Period),
            egui::Key::Slash => Some(KeyCode::Slash),
            egui::Key::Backslash => Some(KeyCode::Backslash),
            // Quote and Backtick not available in this egui version
            _ => None,
        }
    }

    /// Convert to egui::Key
    pub fn to_egui(&self) -> Option<egui::Key> {
        match self {
            KeyCode::A => Some(egui::Key::A),
            KeyCode::B => Some(egui::Key::B),
            KeyCode::C => Some(egui::Key::C),
            KeyCode::D => Some(egui::Key::D),
            KeyCode::E => Some(egui::Key::E),
            KeyCode::F => Some(egui::Key::F),
            KeyCode::G => Some(egui::Key::G),
            KeyCode::H => Some(egui::Key::H),
            KeyCode::I => Some(egui::Key::I),
            KeyCode::J => Some(egui::Key::J),
            KeyCode::K => Some(egui::Key::K),
            KeyCode::L => Some(egui::Key::L),
            KeyCode::M => Some(egui::Key::M),
            KeyCode::N => Some(egui::Key::N),
            KeyCode::O => Some(egui::Key::O),
            KeyCode::P => Some(egui::Key::P),
            KeyCode::Q => Some(egui::Key::Q),
            KeyCode::R => Some(egui::Key::R),
            KeyCode::S => Some(egui::Key::S),
            KeyCode::T => Some(egui::Key::T),
            KeyCode::U => Some(egui::Key::U),
            KeyCode::V => Some(egui::Key::V),
            KeyCode::W => Some(egui::Key::W),
            KeyCode::X => Some(egui::Key::X),
            KeyCode::Y => Some(egui::Key::Y),
            KeyCode::Z => Some(egui::Key::Z),
            KeyCode::Num0 => Some(egui::Key::Num0),
            KeyCode::Num1 => Some(egui::Key::Num1),
            KeyCode::Num2 => Some(egui::Key::Num2),
            KeyCode::Num3 => Some(egui::Key::Num3),
            KeyCode::Num4 => Some(egui::Key::Num4),
            KeyCode::Num5 => Some(egui::Key::Num5),
            KeyCode::Num6 => Some(egui::Key::Num6),
            KeyCode::Num7 => Some(egui::Key::Num7),
            KeyCode::Num8 => Some(egui::Key::Num8),
            KeyCode::Num9 => Some(egui::Key::Num9),
            KeyCode::Escape => Some(egui::Key::Escape),
            KeyCode::Enter => Some(egui::Key::Enter),
            KeyCode::Space => Some(egui::Key::Space),
            KeyCode::Backspace => Some(egui::Key::Backspace),
            KeyCode::Tab => Some(egui::Key::Tab),
            KeyCode::Delete => Some(egui::Key::Delete),
            KeyCode::ArrowUp => Some(egui::Key::ArrowUp),
            KeyCode::ArrowDown => Some(egui::Key::ArrowDown),
            KeyCode::ArrowLeft => Some(egui::Key::ArrowLeft),
            KeyCode::ArrowRight => Some(egui::Key::ArrowRight),
            KeyCode::PageUp => Some(egui::Key::PageUp),
            KeyCode::PageDown => Some(egui::Key::PageDown),
            KeyCode::Home => Some(egui::Key::Home),
            KeyCode::End => Some(egui::Key::End),
            KeyCode::F1 => Some(egui::Key::F1),
            KeyCode::F2 => Some(egui::Key::F2),
            KeyCode::F3 => Some(egui::Key::F3),
            KeyCode::F4 => Some(egui::Key::F4),
            KeyCode::F5 => Some(egui::Key::F5),
            KeyCode::F6 => Some(egui::Key::F6),
            KeyCode::F7 => Some(egui::Key::F7),
            KeyCode::F8 => Some(egui::Key::F8),
            KeyCode::F9 => Some(egui::Key::F9),
            KeyCode::F10 => Some(egui::Key::F10),
            KeyCode::F11 => Some(egui::Key::F11),
            KeyCode::F12 => Some(egui::Key::F12),
            KeyCode::Minus => Some(egui::Key::Minus),
            KeyCode::Plus => Some(egui::Key::Plus),
            KeyCode::Equals => Some(egui::Key::Equals),
            KeyCode::Semicolon => Some(egui::Key::Semicolon),
            KeyCode::Colon => Some(egui::Key::Colon),
            KeyCode::BracketLeft => Some(egui::Key::OpenBracket),
            KeyCode::BracketRight => Some(egui::Key::CloseBracket),
            KeyCode::Comma => Some(egui::Key::Comma),
            KeyCode::Period => Some(egui::Key::Period),
            KeyCode::Slash => Some(egui::Key::Slash),
            KeyCode::Backslash => Some(egui::Key::Backslash),
            // Quote and Backtick not available in this egui version
            KeyCode::Quote | KeyCode::Backtick => None,
        }
    }

    /// Convert from bevy::prelude::KeyCode
    pub fn from_bevy(key: bevy::prelude::KeyCode) -> Option<Self> {
        use bevy::prelude::KeyCode as BevyKey;
        match key {
            BevyKey::KeyA => Some(KeyCode::A),
            BevyKey::KeyB => Some(KeyCode::B),
            BevyKey::KeyC => Some(KeyCode::C),
            BevyKey::KeyD => Some(KeyCode::D),
            BevyKey::KeyE => Some(KeyCode::E),
            BevyKey::KeyF => Some(KeyCode::F),
            BevyKey::KeyG => Some(KeyCode::G),
            BevyKey::KeyH => Some(KeyCode::H),
            BevyKey::KeyI => Some(KeyCode::I),
            BevyKey::KeyJ => Some(KeyCode::J),
            BevyKey::KeyK => Some(KeyCode::K),
            BevyKey::KeyL => Some(KeyCode::L),
            BevyKey::KeyM => Some(KeyCode::M),
            BevyKey::KeyN => Some(KeyCode::N),
            BevyKey::KeyO => Some(KeyCode::O),
            BevyKey::KeyP => Some(KeyCode::P),
            BevyKey::KeyQ => Some(KeyCode::Q),
            BevyKey::KeyR => Some(KeyCode::R),
            BevyKey::KeyS => Some(KeyCode::S),
            BevyKey::KeyT => Some(KeyCode::T),
            BevyKey::KeyU => Some(KeyCode::U),
            BevyKey::KeyV => Some(KeyCode::V),
            BevyKey::KeyW => Some(KeyCode::W),
            BevyKey::KeyX => Some(KeyCode::X),
            BevyKey::KeyY => Some(KeyCode::Y),
            BevyKey::KeyZ => Some(KeyCode::Z),
            BevyKey::Digit0 => Some(KeyCode::Num0),
            BevyKey::Digit1 => Some(KeyCode::Num1),
            BevyKey::Digit2 => Some(KeyCode::Num2),
            BevyKey::Digit3 => Some(KeyCode::Num3),
            BevyKey::Digit4 => Some(KeyCode::Num4),
            BevyKey::Digit5 => Some(KeyCode::Num5),
            BevyKey::Digit6 => Some(KeyCode::Num6),
            BevyKey::Digit7 => Some(KeyCode::Num7),
            BevyKey::Digit8 => Some(KeyCode::Num8),
            BevyKey::Digit9 => Some(KeyCode::Num9),
            BevyKey::Escape => Some(KeyCode::Escape),
            BevyKey::Enter => Some(KeyCode::Enter),
            BevyKey::Space => Some(KeyCode::Space),
            BevyKey::Backspace => Some(KeyCode::Backspace),
            BevyKey::Tab => Some(KeyCode::Tab),
            BevyKey::Delete => Some(KeyCode::Delete),
            BevyKey::ArrowUp => Some(KeyCode::ArrowUp),
            BevyKey::ArrowDown => Some(KeyCode::ArrowDown),
            BevyKey::ArrowLeft => Some(KeyCode::ArrowLeft),
            BevyKey::ArrowRight => Some(KeyCode::ArrowRight),
            BevyKey::PageUp => Some(KeyCode::PageUp),
            BevyKey::PageDown => Some(KeyCode::PageDown),
            BevyKey::Home => Some(KeyCode::Home),
            BevyKey::End => Some(KeyCode::End),
            BevyKey::F1 => Some(KeyCode::F1),
            BevyKey::F2 => Some(KeyCode::F2),
            BevyKey::F3 => Some(KeyCode::F3),
            BevyKey::F4 => Some(KeyCode::F4),
            BevyKey::F5 => Some(KeyCode::F5),
            BevyKey::F6 => Some(KeyCode::F6),
            BevyKey::F7 => Some(KeyCode::F7),
            BevyKey::F8 => Some(KeyCode::F8),
            BevyKey::F9 => Some(KeyCode::F9),
            BevyKey::F10 => Some(KeyCode::F10),
            BevyKey::F11 => Some(KeyCode::F11),
            BevyKey::F12 => Some(KeyCode::F12),
            BevyKey::Minus => Some(KeyCode::Minus),
            BevyKey::Equal => Some(KeyCode::Equals),
            BevyKey::Semicolon => Some(KeyCode::Semicolon),
            BevyKey::BracketLeft => Some(KeyCode::BracketLeft),
            BevyKey::BracketRight => Some(KeyCode::BracketRight),
            BevyKey::Comma => Some(KeyCode::Comma),
            BevyKey::Period => Some(KeyCode::Period),
            BevyKey::Slash => Some(KeyCode::Slash),
            BevyKey::Backslash => Some(KeyCode::Backslash),
            BevyKey::Quote => Some(KeyCode::Quote),
            BevyKey::Backquote => Some(KeyCode::Backtick),
            _ => None,
        }
    }

    /// Convert to bevy::prelude::KeyCode
    pub fn to_bevy(&self) -> Option<bevy::prelude::KeyCode> {
        use bevy::prelude::KeyCode as BevyKey;
        match self {
            KeyCode::A => Some(BevyKey::KeyA),
            KeyCode::B => Some(BevyKey::KeyB),
            KeyCode::C => Some(BevyKey::KeyC),
            KeyCode::D => Some(BevyKey::KeyD),
            KeyCode::E => Some(BevyKey::KeyE),
            KeyCode::F => Some(BevyKey::KeyF),
            KeyCode::G => Some(BevyKey::KeyG),
            KeyCode::H => Some(BevyKey::KeyH),
            KeyCode::I => Some(BevyKey::KeyI),
            KeyCode::J => Some(BevyKey::KeyJ),
            KeyCode::K => Some(BevyKey::KeyK),
            KeyCode::L => Some(BevyKey::KeyL),
            KeyCode::M => Some(BevyKey::KeyM),
            KeyCode::N => Some(BevyKey::KeyN),
            KeyCode::O => Some(BevyKey::KeyO),
            KeyCode::P => Some(BevyKey::KeyP),
            KeyCode::Q => Some(BevyKey::KeyQ),
            KeyCode::R => Some(BevyKey::KeyR),
            KeyCode::S => Some(BevyKey::KeyS),
            KeyCode::T => Some(BevyKey::KeyT),
            KeyCode::U => Some(BevyKey::KeyU),
            KeyCode::V => Some(BevyKey::KeyV),
            KeyCode::W => Some(BevyKey::KeyW),
            KeyCode::X => Some(BevyKey::KeyX),
            KeyCode::Y => Some(BevyKey::KeyY),
            KeyCode::Z => Some(BevyKey::KeyZ),
            KeyCode::Num0 => Some(BevyKey::Digit0),
            KeyCode::Num1 => Some(BevyKey::Digit1),
            KeyCode::Num2 => Some(BevyKey::Digit2),
            KeyCode::Num3 => Some(BevyKey::Digit3),
            KeyCode::Num4 => Some(BevyKey::Digit4),
            KeyCode::Num5 => Some(BevyKey::Digit5),
            KeyCode::Num6 => Some(BevyKey::Digit6),
            KeyCode::Num7 => Some(BevyKey::Digit7),
            KeyCode::Num8 => Some(BevyKey::Digit8),
            KeyCode::Num9 => Some(BevyKey::Digit9),
            KeyCode::Escape => Some(BevyKey::Escape),
            KeyCode::Enter => Some(BevyKey::Enter),
            KeyCode::Space => Some(BevyKey::Space),
            KeyCode::Backspace => Some(BevyKey::Backspace),
            KeyCode::Tab => Some(BevyKey::Tab),
            KeyCode::Delete => Some(BevyKey::Delete),
            KeyCode::ArrowUp => Some(BevyKey::ArrowUp),
            KeyCode::ArrowDown => Some(BevyKey::ArrowDown),
            KeyCode::ArrowLeft => Some(BevyKey::ArrowLeft),
            KeyCode::ArrowRight => Some(BevyKey::ArrowRight),
            KeyCode::PageUp => Some(BevyKey::PageUp),
            KeyCode::PageDown => Some(BevyKey::PageDown),
            KeyCode::Home => Some(BevyKey::Home),
            KeyCode::End => Some(BevyKey::End),
            KeyCode::F1 => Some(BevyKey::F1),
            KeyCode::F2 => Some(BevyKey::F2),
            KeyCode::F3 => Some(BevyKey::F3),
            KeyCode::F4 => Some(BevyKey::F4),
            KeyCode::F5 => Some(BevyKey::F5),
            KeyCode::F6 => Some(BevyKey::F6),
            KeyCode::F7 => Some(BevyKey::F7),
            KeyCode::F8 => Some(BevyKey::F8),
            KeyCode::F9 => Some(BevyKey::F9),
            KeyCode::F10 => Some(BevyKey::F10),
            KeyCode::F11 => Some(BevyKey::F11),
            KeyCode::F12 => Some(BevyKey::F12),
            KeyCode::Minus => Some(BevyKey::Minus),
            KeyCode::Plus => None, // Bevy doesn't have a Plus key, it's Shift+Equals
            KeyCode::Equals => Some(BevyKey::Equal),
            KeyCode::Colon => None, // Colon is Shift+Semicolon
            KeyCode::Semicolon => Some(BevyKey::Semicolon),
            KeyCode::BracketLeft => Some(BevyKey::BracketLeft),
            KeyCode::BracketRight => Some(BevyKey::BracketRight),
            KeyCode::Comma => Some(BevyKey::Comma),
            KeyCode::Period => Some(BevyKey::Period),
            KeyCode::Slash => Some(BevyKey::Slash),
            KeyCode::Backslash => Some(BevyKey::Backslash),
            KeyCode::Quote => Some(BevyKey::Quote),
            KeyCode::Backtick => Some(BevyKey::Backquote),
        }
    }
}

/// Modifier keys
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Modifiers {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl Modifiers {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn ctrl() -> Self {
        Self { ctrl: true, ..Default::default() }
    }

    pub fn shift() -> Self {
        Self { shift: true, ..Default::default() }
    }

    pub fn alt() -> Self {
        Self { alt: true, ..Default::default() }
    }

    pub fn from_egui(mods: &egui::Modifiers) -> Self {
        Self {
            ctrl: mods.ctrl || mods.command,
            shift: mods.shift,
            alt: mods.alt,
        }
    }

    pub fn from_bevy(keys: &bevy::prelude::ButtonInput<bevy::prelude::KeyCode>) -> Self {
        use bevy::prelude::KeyCode as BevyKey;
        Self {
            ctrl: keys.pressed(BevyKey::ControlLeft) || keys.pressed(BevyKey::ControlRight),
            shift: keys.pressed(BevyKey::ShiftLeft) || keys.pressed(BevyKey::ShiftRight),
            alt: keys.pressed(BevyKey::AltLeft) || keys.pressed(BevyKey::AltRight),
        }
    }
}

/// A complete key binding (key + modifiers)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyBinding {
    pub fn new(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    pub fn simple(key: KeyCode) -> Self {
        Self { key, modifiers: Modifiers::none() }
    }

    /// Parse from string format like "Ctrl+Shift+N"
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('+').collect();
        let mut modifiers = Modifiers::default();
        let mut key = None;

        for part in parts {
            let part = part.trim();
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                "alt" => modifiers.alt = true,
                _ => {
                    if key.is_some() {
                        return Err(format!("Multiple keys specified: {}", s));
                    }
                    key = Some(KeyCode::from_str(part)?);
                }
            }
        }

        Ok(KeyBinding {
            key: key.ok_or_else(|| format!("No key specified in: {}", s))?,
            modifiers,
        })
    }

    /// Check if this key binding was pressed this frame
    pub fn is_pressed(&self, input: &egui::InputState) -> bool {
        let modifiers = Modifiers::from_egui(&input.modifiers);
        if modifiers != self.modifiers {
            return false;
        }

        if let Some(egui_key) = self.key.to_egui() {
            input.key_pressed(egui_key)
        } else {
            false
        }
    }

    /// Check if this key binding is currently held down
    pub fn is_down(&self, input: &egui::InputState) -> bool {
        let modifiers = Modifiers::from_egui(&input.modifiers);
        if modifiers != self.modifiers {
            return false;
        }

        if let Some(egui_key) = self.key.to_egui() {
            input.key_down(egui_key)
        } else {
            false
        }
    }

    /// Check if this key binding was released this frame (ignores modifiers)
    pub fn is_released(&self, input: &egui::InputState) -> bool {
        if let Some(egui_key) = self.key.to_egui() {
            input.key_released(egui_key)
        } else {
            false
        }
    }

    /// Create KeyBinding from egui key and current modifiers
    pub fn from_egui_key(key: egui::Key, mods: &egui::Modifiers) -> Option<Self> {
        KeyCode::from_egui(key).map(|key_code| {
            KeyBinding::new(key_code, Modifiers::from_egui(mods))
        })
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.modifiers.ctrl { parts.push("Ctrl"); }
        if self.modifiers.shift { parts.push("Shift"); }
        if self.modifiers.alt { parts.push("Alt"); }
        parts.push(self.key.as_str());
        write!(f, "{}", parts.join("+"))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let kb = KeyBinding::parse("A").unwrap();
        assert_eq!(kb.key, KeyCode::A);
        assert!(!kb.modifiers.ctrl);
    }

    #[test]
    fn test_parse_with_modifiers() {
        let kb = KeyBinding::parse("Ctrl+Shift+N").unwrap();
        assert_eq!(kb.key, KeyCode::N);
        assert!(kb.modifiers.ctrl);
        assert!(kb.modifiers.shift);
        assert!(!kb.modifiers.alt);
    }

    #[test]
    fn test_display() {
        let kb = KeyBinding::new(KeyCode::N, Modifiers { ctrl: true, shift: true, alt: false });
        assert_eq!(kb.to_string(), "Ctrl+Shift+N");
    }
}
