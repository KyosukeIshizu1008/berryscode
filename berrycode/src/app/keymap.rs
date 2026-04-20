//! Customizable keyboard shortcut system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyAction {
    Save,
    Undo,
    Redo,
    Find,
    Replace,
    Format,
    DuplicateLine,
    AddCursorNext,
    GizmoMove,
    GizmoRotate,
    GizmoScale,
    RunProject,
    ToggleBreakpoint,
    StartDebug,
    PeekDefinition,
    Rename,
    FoldBlock,
    UnfoldBlock,
    DeleteEntity,
    DuplicateEntity,
    Escape,
}

impl KeyAction {
    pub const ALL: &'static [KeyAction] = &[
        KeyAction::Save,
        KeyAction::Undo,
        KeyAction::Redo,
        KeyAction::Find,
        KeyAction::Replace,
        KeyAction::Format,
        KeyAction::DuplicateLine,
        KeyAction::AddCursorNext,
        KeyAction::GizmoMove,
        KeyAction::GizmoRotate,
        KeyAction::GizmoScale,
        KeyAction::RunProject,
        KeyAction::ToggleBreakpoint,
        KeyAction::StartDebug,
        KeyAction::PeekDefinition,
        KeyAction::Rename,
        KeyAction::FoldBlock,
        KeyAction::UnfoldBlock,
        KeyAction::DeleteEntity,
        KeyAction::DuplicateEntity,
        KeyAction::Escape,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            KeyAction::Save => "Save",
            KeyAction::Undo => "Undo",
            KeyAction::Redo => "Redo",
            KeyAction::Find => "Find",
            KeyAction::Replace => "Replace",
            KeyAction::Format => "Format",
            KeyAction::DuplicateLine => "Duplicate Line",
            KeyAction::AddCursorNext => "Add Cursor at Next",
            KeyAction::GizmoMove => "Gizmo: Move",
            KeyAction::GizmoRotate => "Gizmo: Rotate",
            KeyAction::GizmoScale => "Gizmo: Scale",
            KeyAction::RunProject => "Run Project",
            KeyAction::ToggleBreakpoint => "Toggle Breakpoint",
            KeyAction::StartDebug => "Start Debug",
            KeyAction::PeekDefinition => "Peek Definition",
            KeyAction::Rename => "Rename Symbol",
            KeyAction::FoldBlock => "Fold Block",
            KeyAction::UnfoldBlock => "Unfold Block",
            KeyAction::DeleteEntity => "Delete Entity",
            KeyAction::DuplicateEntity => "Duplicate Entity",
            KeyAction::Escape => "Escape",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub command: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: String,
}

impl KeyBinding {
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.command {
            parts.push("Cmd");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(&self.key);
        parts.join("+")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keymap {
    pub bindings: HashMap<KeyAction, KeyBinding>,
}

impl Default for Keymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        let b = |cmd: bool, shift: bool, alt: bool, key: &str| KeyBinding {
            command: cmd,
            shift,
            alt,
            key: key.to_string(),
        };
        bindings.insert(KeyAction::Save, b(true, false, false, "S"));
        bindings.insert(KeyAction::Undo, b(true, false, false, "Z"));
        bindings.insert(KeyAction::Redo, b(true, true, false, "Z"));
        bindings.insert(KeyAction::Find, b(true, false, false, "F"));
        bindings.insert(KeyAction::Replace, b(true, false, false, "H"));
        bindings.insert(KeyAction::Format, b(true, true, false, "F"));
        bindings.insert(KeyAction::DuplicateLine, b(true, true, false, "D"));
        bindings.insert(KeyAction::AddCursorNext, b(true, false, false, "D"));
        bindings.insert(KeyAction::GizmoMove, b(false, false, false, "W"));
        bindings.insert(KeyAction::GizmoRotate, b(false, false, false, "E"));
        bindings.insert(KeyAction::GizmoScale, b(false, false, false, "R"));
        bindings.insert(KeyAction::RunProject, b(true, false, false, "R"));
        bindings.insert(KeyAction::ToggleBreakpoint, b(false, false, false, "F9"));
        bindings.insert(KeyAction::StartDebug, b(false, false, false, "F5"));
        bindings.insert(KeyAction::PeekDefinition, b(false, false, true, "F12"));
        bindings.insert(KeyAction::Rename, b(false, false, false, "F2"));
        bindings.insert(KeyAction::FoldBlock, b(true, true, false, "OpenBracket"));
        bindings.insert(KeyAction::UnfoldBlock, b(true, true, false, "CloseBracket"));
        bindings.insert(KeyAction::DeleteEntity, b(false, false, false, "Delete"));
        bindings.insert(KeyAction::DuplicateEntity, b(true, false, false, "D"));
        bindings.insert(KeyAction::Escape, b(false, false, false, "Escape"));
        Self { bindings }
    }
}

impl Keymap {
    /// Check if a specific action's keybinding is pressed this frame.
    pub fn is_pressed(&self, action: KeyAction, input: &egui::InputState) -> bool {
        let binding = match self.bindings.get(&action) {
            Some(b) => b,
            None => return false,
        };

        if input.modifiers.command != binding.command {
            return false;
        }
        if input.modifiers.shift != binding.shift {
            return false;
        }
        if input.modifiers.alt != binding.alt {
            return false;
        }

        let key = match binding.key.as_str() {
            "A" => egui::Key::A,
            "B" => egui::Key::B,
            "C" => egui::Key::C,
            "D" => egui::Key::D,
            "E" => egui::Key::E,
            "F" => egui::Key::F,
            "G" => egui::Key::G,
            "H" => egui::Key::H,
            "I" => egui::Key::I,
            "J" => egui::Key::J,
            "K" => egui::Key::K,
            "L" => egui::Key::L,
            "M" => egui::Key::M,
            "N" => egui::Key::N,
            "O" => egui::Key::O,
            "P" => egui::Key::P,
            "Q" => egui::Key::Q,
            "R" => egui::Key::R,
            "S" => egui::Key::S,
            "T" => egui::Key::T,
            "U" => egui::Key::U,
            "V" => egui::Key::V,
            "W" => egui::Key::W,
            "X" => egui::Key::X,
            "Y" => egui::Key::Y,
            "Z" => egui::Key::Z,
            "F1" => egui::Key::F1,
            "F2" => egui::Key::F2,
            "F3" => egui::Key::F3,
            "F4" => egui::Key::F4,
            "F5" => egui::Key::F5,
            "F6" => egui::Key::F6,
            "F7" => egui::Key::F7,
            "F8" => egui::Key::F8,
            "F9" => egui::Key::F9,
            "F10" => egui::Key::F10,
            "F11" => egui::Key::F11,
            "F12" => egui::Key::F12,
            "Escape" => egui::Key::Escape,
            "Delete" => egui::Key::Delete,
            "Backspace" => egui::Key::Backspace,
            "OpenBracket" => egui::Key::OpenBracket,
            "CloseBracket" => egui::Key::CloseBracket,
            _ => return false,
        };

        input.key_pressed(key)
    }

    /// Load from file, falling back to defaults.
    pub fn load() -> Self {
        let path = keymap_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(km) = ron::from_str::<Keymap>(&content) {
                return km;
            }
        }
        Keymap::default()
    }

    /// Save to file.
    pub fn save(&self) {
        let path = keymap_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(&path, s);
        }
    }
}

fn keymap_path() -> String {
    if let Some(home) = dirs::home_dir() {
        format!("{}/.berrycode/keybindings.ron", home.display())
    } else {
        "keybindings.ron".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keymap_has_all_actions() {
        let km = Keymap::default();
        for action in KeyAction::ALL {
            assert!(
                km.bindings.contains_key(action),
                "Missing binding for {:?}",
                action
            );
        }
    }

    #[test]
    fn key_binding_display() {
        let b = KeyBinding {
            command: true,
            shift: false,
            alt: false,
            key: "S".to_string(),
        };
        assert_eq!(b.display(), "Cmd+S");

        let b2 = KeyBinding {
            command: true,
            shift: true,
            alt: false,
            key: "Z".to_string(),
        };
        assert_eq!(b2.display(), "Cmd+Shift+Z");

        let b3 = KeyBinding {
            command: false,
            shift: false,
            alt: true,
            key: "F12".to_string(),
        };
        assert_eq!(b3.display(), "Alt+F12");
    }

    #[test]
    fn keymap_roundtrip_ron() {
        let km = Keymap::default();
        let serialized =
            ron::ser::to_string_pretty(&km, ron::ser::PrettyConfig::default()).expect("serialize");
        let deserialized: Keymap = ron::from_str(&serialized).expect("deserialize");
        assert_eq!(km.bindings.len(), deserialized.bindings.len());
        for action in KeyAction::ALL {
            let orig = km.bindings.get(action).expect("orig");
            let deser = deserialized.bindings.get(action).expect("deser");
            assert_eq!(orig.command, deser.command);
            assert_eq!(orig.shift, deser.shift);
            assert_eq!(orig.alt, deser.alt);
            assert_eq!(orig.key, deser.key);
        }
    }

    #[test]
    fn all_actions_have_labels() {
        for action in KeyAction::ALL {
            let label = action.label();
            assert!(!label.is_empty(), "Empty label for {:?}", action);
        }
    }

    #[test]
    fn load_returns_default_when_no_file() {
        // keymap_path() points to ~/.berrycode/keybindings.ron which may not exist
        // in a test environment -- load() should gracefully return defaults.
        let km = Keymap::load();
        assert_eq!(km.bindings.len(), KeyAction::ALL.len());
    }
}
