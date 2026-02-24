//! Gamepad input handling using gilrs

use gilrs::{Axis, Button, Event, EventType, Gilrs};
use std::collections::HashSet;
use std::sync::Mutex;

use crate::keybindings::key::GamepadKey;

/// Thread-safe snapshot of gamepad input state
/// This can be stored in AppState (which requires Sync)
#[derive(Clone, Default)]
pub struct GamepadSnapshot {
    pub pressed_this_frame: HashSet<GamepadKey>,
    pub released_this_frame: HashSet<GamepadKey>,
    pub held: HashSet<GamepadKey>,
    pub left_stick: (f32, f32),
    pub right_stick: (f32, f32),
}

impl GamepadSnapshot {
    pub fn is_pressed(&self, key: GamepadKey) -> bool {
        self.pressed_this_frame.contains(&key)
    }

    pub fn is_released(&self, key: GamepadKey) -> bool {
        self.released_this_frame.contains(&key)
    }

    pub fn is_held(&self, key: GamepadKey) -> bool {
        self.held.contains(&key)
    }
}

/// Gamepad state tracking (not thread-safe, kept outside ECS)
pub struct GamepadState {
    gilrs: Gilrs,
    pressed_this_frame: HashSet<GamepadKey>,
    released_this_frame: HashSet<GamepadKey>,
    held: HashSet<GamepadKey>,
    left_stick: (f32, f32),
    right_stick: (f32, f32),
}

const STICK_DEAD_ZONE: f32 = 0.3;

impl GamepadState {
    pub fn new() -> Option<Self> {
        match Gilrs::new() {
            Ok(gilrs) => Some(Self {
                gilrs,
                pressed_this_frame: HashSet::new(),
                released_this_frame: HashSet::new(),
                held: HashSet::new(),
                left_stick: (0.0, 0.0),
                right_stick: (0.0, 0.0),
            }),
            Err(e) => {
                eprintln!("Failed to initialize gamepad: {}", e);
                None
            }
        }
    }

    pub fn update(&mut self) {
        self.pressed_this_frame.clear();
        self.released_this_frame.clear();

        while let Some(Event { event, .. }) = self.gilrs.next_event() {
            match event {
                EventType::ButtonPressed(button, _) => {
                    if let Some(key) = button_to_key(button) {
                        self.pressed_this_frame.insert(key);
                        self.held.insert(key);
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(key) = button_to_key(button) {
                        self.released_this_frame.insert(key);
                        self.held.remove(&key);
                    }
                }
                EventType::AxisChanged(axis, value, _) => {
                    self.handle_axis(axis, value);
                }
                _ => {}
            }
        }
    }

    fn handle_axis(&mut self, axis: Axis, value: f32) {
        match axis {
            Axis::LeftStickX => {
                self.left_stick.0 = value;
                self.update_stick_buttons_x(true);
            }
            Axis::LeftStickY => {
                self.left_stick.1 = value;
                self.update_stick_buttons_y(true);
            }
            Axis::RightStickX => {
                self.right_stick.0 = value;
            }
            Axis::RightStickY => {
                self.right_stick.1 = value;
            }
            _ => {}
        }
    }

    fn update_stick_buttons_x(&mut self, is_left: bool) {
        if !is_left {
            return;
        }
        let x = self.left_stick.0;
        let left = GamepadKey::LeftStickLeft;
        let right = GamepadKey::LeftStickRight;

        // Handle X axis - right
        if x > STICK_DEAD_ZONE && !self.held.contains(&right) {
            self.pressed_this_frame.insert(right);
            self.held.insert(right);
        } else if x <= STICK_DEAD_ZONE && self.held.contains(&right) {
            self.released_this_frame.insert(right);
            self.held.remove(&right);
        }

        // Handle X axis - left
        if x < -STICK_DEAD_ZONE && !self.held.contains(&left) {
            self.pressed_this_frame.insert(left);
            self.held.insert(left);
        } else if x >= -STICK_DEAD_ZONE && self.held.contains(&left) {
            self.released_this_frame.insert(left);
            self.held.remove(&left);
        }
    }

    fn update_stick_buttons_y(&mut self, is_left: bool) {
        if !is_left {
            return;
        }
        let y = self.left_stick.1;
        let up = GamepadKey::LeftStickUp;
        let down = GamepadKey::LeftStickDown;

        // Handle Y axis - down (positive Y is typically down on most controllers)
        if y > STICK_DEAD_ZONE && !self.held.contains(&down) {
            self.pressed_this_frame.insert(down);
            self.held.insert(down);
        } else if y <= STICK_DEAD_ZONE && self.held.contains(&down) {
            self.released_this_frame.insert(down);
            self.held.remove(&down);
        }

        // Handle Y axis - up (negative Y)
        if y < -STICK_DEAD_ZONE && !self.held.contains(&up) {
            self.pressed_this_frame.insert(up);
            self.held.insert(up);
        } else if y >= -STICK_DEAD_ZONE && self.held.contains(&up) {
            self.released_this_frame.insert(up);
            self.held.remove(&up);
        }
    }

    /// Create a thread-safe snapshot of the current state
    pub fn snapshot(&self) -> GamepadSnapshot {
        GamepadSnapshot {
            pressed_this_frame: self.pressed_this_frame.clone(),
            released_this_frame: self.released_this_frame.clone(),
            held: self.held.clone(),
            left_stick: self.left_stick,
            right_stick: self.right_stick,
        }
    }
}

fn button_to_key(button: Button) -> Option<GamepadKey> {
    match button {
        Button::South => Some(GamepadKey::South),
        Button::East => Some(GamepadKey::East),
        Button::West => Some(GamepadKey::West),
        Button::North => Some(GamepadKey::North),
        Button::LeftTrigger => Some(GamepadKey::LeftBumper),
        Button::RightTrigger => Some(GamepadKey::RightBumper),
        Button::LeftTrigger2 => Some(GamepadKey::LeftTrigger),
        Button::RightTrigger2 => Some(GamepadKey::RightTrigger),
        Button::LeftThumb => Some(GamepadKey::LeftStick),
        Button::RightThumb => Some(GamepadKey::RightStick),
        Button::Start => Some(GamepadKey::Start),
        Button::Select => Some(GamepadKey::Select),
        Button::DPadUp => Some(GamepadKey::DPadUp),
        Button::DPadDown => Some(GamepadKey::DPadDown),
        Button::DPadLeft => Some(GamepadKey::DPadLeft),
        Button::DPadRight => Some(GamepadKey::DPadRight),
        _ => None,
    }
}

/// Global gamepad state holder (kept outside ECS for thread-safety reasons)
/// Updated by calling update_global_gamepad() before ECS systems run
static GAMEPAD: std::sync::OnceLock<Mutex<Option<GamepadState>>> = std::sync::OnceLock::new();
static GAMEPAD_SNAPSHOT: std::sync::OnceLock<Mutex<GamepadSnapshot>> = std::sync::OnceLock::new();

/// Initialize the global gamepad
pub fn init_global_gamepad() {
    let _ = GAMEPAD.set(Mutex::new(GamepadState::new()));
    let _ = GAMEPAD_SNAPSHOT.set(Mutex::new(GamepadSnapshot::default()));
}

/// Update the global gamepad and snapshot
pub fn update_global_gamepad() {
    if let Some(gamepad_mutex) = GAMEPAD.get() {
        if let Ok(mut gamepad_opt) = gamepad_mutex.lock() {
            if let Some(ref mut gamepad) = *gamepad_opt {
                gamepad.update();
                if let Some(snapshot_mutex) = GAMEPAD_SNAPSHOT.get() {
                    if let Ok(mut snapshot) = snapshot_mutex.lock() {
                        *snapshot = gamepad.snapshot();
                    }
                }
            }
        }
    }
}

/// Get a copy of the current gamepad snapshot
pub fn get_gamepad_snapshot() -> Option<GamepadSnapshot> {
    GAMEPAD_SNAPSHOT.get()
        .and_then(|m| m.lock().ok())
        .map(|s| s.clone())
}

/// Check if any gamepad is connected
pub fn is_gamepad_connected() -> bool {
    if let Some(gamepad_mutex) = GAMEPAD.get() {
        if let Ok(gamepad_opt) = gamepad_mutex.lock() {
            if let Some(ref gamepad) = *gamepad_opt {
                // Check if any gamepad is connected by iterating gamepads
                return gamepad.gilrs.gamepads().next().is_some();
            }
        }
    }
    false
}
