//! Platform-agnostic event abstractions
//!
//! Provides unified event types that can be constructed from
//! platform-specific events (web-sys, native touch events, etc.)

use serde::{Deserialize, Serialize};

// ========================================
// Pointer Events (Mouse/Touch)
// ========================================

/// Platform-independent pointer position
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointerPosition {
    /// X coordinate relative to the viewport
    pub client_x: f64,
    /// Y coordinate relative to the viewport
    pub client_y: f64,
    /// X coordinate relative to the page
    pub page_x: f64,
    /// Y coordinate relative to the page
    pub page_y: f64,
}

impl PointerPosition {
    pub fn new(client_x: f64, client_y: f64) -> Self {
        Self {
            client_x,
            client_y,
            page_x: client_x,
            page_y: client_y,
        }
    }

    pub fn with_page(client_x: f64, client_y: f64, page_x: f64, page_y: f64) -> Self {
        Self {
            client_x,
            client_y,
            page_x,
            page_y,
        }
    }
}

/// Platform-independent pointer event
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointerEvent {
    pub position: PointerPosition,
    pub button: PointerButton,
    pub shift_key: bool,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerButton {
    Primary,   // Left mouse button or primary touch
    Secondary, // Right mouse button
    Auxiliary, // Middle mouse button
    None,      // Touch events
}

// Web implementation
#[cfg(target_arch = "wasm32")]
impl From<web_sys::MouseEvent> for PointerPosition {
    fn from(ev: web_sys::MouseEvent) -> Self {
        Self::with_page(
            ev.client_x() as f64,
            ev.client_y() as f64,
            ev.page_x() as f64,
            ev.page_y() as f64,
        )
    }
}

#[cfg(target_arch = "wasm32")]
impl From<web_sys::MouseEvent> for PointerEvent {
    fn from(ev: web_sys::MouseEvent) -> Self {
        let button = match ev.button() {
            0 => PointerButton::Primary,
            1 => PointerButton::Auxiliary,
            2 => PointerButton::Secondary,
            _ => PointerButton::None,
        };

        Self {
            position: PointerPosition::from(ev.clone()),
            button,
            shift_key: ev.shift_key(),
            ctrl_key: ev.ctrl_key(),
            alt_key: ev.alt_key(),
            meta_key: ev.meta_key(),
        }
    }
}

// TouchEvent support for iOS/Android
#[cfg(target_arch = "wasm32")]
impl PointerPosition {
    /// Create from TouchEvent (for iOS/Android support)
    /// Note: Touch support temporarily disabled for WASM compatibility
    pub fn from_touch(_ev: &web_sys::TouchEvent) -> Option<Self> {
        // TODO: Implement touch support without TouchList dependency
        None
    }
}

#[cfg(target_arch = "wasm32")]
impl PointerEvent {
    /// Create from TouchEvent (for iOS/Android support)
    pub fn from_touch(ev: &web_sys::TouchEvent) -> Option<Self> {
        let position = PointerPosition::from_touch(ev)?;

        Some(Self {
            position,
            button: PointerButton::None, // Touch doesn't have buttons
            shift_key: ev.shift_key(),
            ctrl_key: ev.ctrl_key(),
            alt_key: ev.alt_key(),
            meta_key: ev.meta_key(),
        })
    }
}

// ========================================
// Keyboard Events
// ========================================

/// Platform-independent keyboard event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardEvent {
    pub key: String,
    pub code: String,
    pub shift_key: bool,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    pub repeat: bool,
}

#[cfg(target_arch = "wasm32")]
impl From<web_sys::KeyboardEvent> for KeyboardEvent {
    fn from(ev: web_sys::KeyboardEvent) -> Self {
        Self {
            key: ev.key(),
            code: ev.code(),
            shift_key: ev.shift_key(),
            ctrl_key: ev.ctrl_key(),
            alt_key: ev.alt_key(),
            meta_key: ev.meta_key(),
            repeat: ev.repeat(),
        }
    }
}

// ========================================
// Scroll Events
// ========================================

/// Platform-independent scroll position
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScrollPosition {
    pub x: f64,
    pub y: f64,
}

impl ScrollPosition {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

// ========================================
// Element Geometry
// ========================================

/// Platform-independent element bounds
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElementBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[cfg(target_arch = "wasm32")]
impl From<web_sys::DomRect> for ElementBounds {
    fn from(rect: web_sys::DomRect) -> Self {
        Self {
            x: rect.x(),
            y: rect.y(),
            width: rect.width(),
            height: rect.height(),
            top: rect.top(),
            right: rect.right(),
            bottom: rect.bottom(),
            left: rect.left(),
        }
    }
}

// ========================================
// Helper functions
// ========================================

/// Check if a modifier key combination matches
pub fn matches_modifier(
    event_shift: bool,
    event_ctrl: bool,
    event_alt: bool,
    event_meta: bool,
    required_shift: bool,
    required_ctrl: bool,
    required_alt: bool,
    required_meta: bool,
) -> bool {
    event_shift == required_shift
        && event_ctrl == required_ctrl
        && event_alt == required_alt
        && event_meta == required_meta
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pointer_position_creation() {
        let pos = PointerPosition::new(100.0, 200.0);
        assert_eq!(pos.client_x, 100.0);
        assert_eq!(pos.client_y, 200.0);
        assert_eq!(pos.page_x, 100.0);
        assert_eq!(pos.page_y, 200.0);
    }

    #[test]
    fn test_pointer_position_with_page() {
        let pos = PointerPosition::with_page(100.0, 200.0, 150.0, 250.0);
        assert_eq!(pos.client_x, 100.0);
        assert_eq!(pos.client_y, 200.0);
        assert_eq!(pos.page_x, 150.0);
        assert_eq!(pos.page_y, 250.0);
    }

    #[test]
    fn test_scroll_position() {
        let scroll = ScrollPosition::new(10.0, 20.0);
        assert_eq!(scroll.x, 10.0);
        assert_eq!(scroll.y, 20.0);
    }

    #[test]
    fn test_modifier_matching() {
        assert!(matches_modifier(true, false, false, false, true, false, false, false));
        assert!(!matches_modifier(true, false, false, false, false, false, false, false));
        assert!(matches_modifier(true, true, false, false, true, true, false, false));
    }
}
