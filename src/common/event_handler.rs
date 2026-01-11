//! Common event handling utilities
//!
//! Provides reusable event handling patterns to avoid duplication.

use leptos::ev::{KeyboardEvent, MouseEvent};

/// Key combination for shortcuts
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub key: String,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl KeyCombo {
    /// Create a new key combination
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
        }
    }

    /// Add Ctrl modifier
    pub fn with_ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    /// Add Shift modifier
    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    /// Add Alt modifier
    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// Add Meta (Cmd/Win) modifier
    pub fn with_meta(mut self) -> Self {
        self.meta = true;
        self
    }

    /// Check if a KeyboardEvent matches this combo
    pub fn matches(&self, event: &KeyboardEvent) -> bool {
        self.key == event.key()
            && self.ctrl == event.ctrl_key()
            && self.shift == event.shift_key()
            && self.alt == event.alt_key()
            && self.meta == event.meta_key()
    }
}

/// Mouse button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    /// Get mouse button from MouseEvent
    pub fn from_event(event: &MouseEvent) -> Option<Self> {
        match event.button() {
            0 => Some(Self::Left),
            1 => Some(Self::Middle),
            2 => Some(Self::Right),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_key_combo_creation() {
        let combo = KeyCombo::new("F2").with_ctrl().with_shift();
        assert_eq!(combo.key, "F2");
        assert!(combo.ctrl);
        assert!(combo.shift);
        assert!(!combo.alt);
        assert!(!combo.meta);
    }

    #[wasm_bindgen_test]
    fn test_mouse_button_variants() {
        assert_eq!(MouseButton::Left as i32, 0);
        assert_eq!(MouseButton::Right as i32, 2);
    }
}
