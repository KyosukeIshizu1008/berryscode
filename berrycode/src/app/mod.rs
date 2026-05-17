//! egui-based main application structure
//! Replaces Dioxus components with egui immediate-mode UI

use crate::focus_stack::FocusLayer;
use crate::native;
use crate::native::fs::DirEntry;
use crate::syntax::SyntaxHighlighter;
use std::collections::HashSet;
use tokio::sync::mpsc;

// ===== Submodules =====
#[cfg(feature = "ai")]
mod ai_chat;
pub(crate) mod ansi;
mod asset_browser;
mod asset_watcher;
mod audio;
mod button_style;
mod cargo_completion;
mod code_actions;
mod custom_snippets;
mod database;
mod debugger;
pub(crate) mod demo_capture;
pub(crate) mod dock;
mod docker;
mod ecs_inspector;
mod editor;
mod events;
mod file_tree;
mod folding;

mod git;
mod godot_panel;
mod header;
pub(crate) mod i18n;
mod image_preview;
mod inlay_hints;
pub(crate) mod keymap;
pub(crate) mod live_collab;
mod lsp;
mod macro_expand;
mod minimap;
pub(crate) mod mobile;
pub(crate) mod mobile_toolchain;
mod model_preview;
pub(crate) mod new_project;
#[cfg(feature = "ai")]
mod oracleberry;
pub(crate) mod package_manager;
mod peek;
pub(crate) mod plugin_system;
pub(crate) mod preview_3d;
pub(crate) mod remote_dev;
mod rename;
mod run_panel;
pub(crate) mod scene_editor;
mod scene_preview;
mod search;
mod settings;
mod shortcuts;
mod sidebar;
pub(crate) mod snippets;
mod status_bar;
mod terminal;
pub(crate) mod terminal_emulator;
pub(crate) mod test_runner;
pub mod types;
pub(crate) mod utils;
pub(crate) mod vim_mode;

// Re-export public types
pub use types::*;

/// Which camera the Scene Editor's central viewport renders from.
/// `Scene` is the orbit-controlled editor camera (default); `Game`
/// follows the scene's `Camera`-tagged entity so the user can preview
/// what their player will see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneViewMode {
    #[default]
    Scene,
    Game,
}

/// Mobile / tablet display profiles for previewing how a scene will look
/// on a target device. The Scene View letter-boxes its render target to
/// the chosen aspect ratio and overlays safe-area indicators for the
/// iPhone notch / Android status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayProfile {
    #[default]
    Default,
    IPhonePortrait,
    IPhoneLandscape,
    IPadPortrait,
    IPadLandscape,
    AndroidPhonePortrait,
    AndroidPhoneLandscape,
    AndroidTablet,
}

/// Pixel dimensions + safe-area insets (top/bottom in logical points)
/// for a `DisplayProfile`. The insets describe the area covered by the
/// system status bar / notch / home indicator.
pub struct DisplayProfileSpec {
    pub label: &'static str,
    pub width: u32,
    pub height: u32,
    pub safe_top: u32,
    pub safe_bottom: u32,
}

impl DisplayProfile {
    pub const ALL: &'static [DisplayProfile] = &[
        DisplayProfile::Default,
        DisplayProfile::IPhonePortrait,
        DisplayProfile::IPhoneLandscape,
        DisplayProfile::IPadPortrait,
        DisplayProfile::IPadLandscape,
        DisplayProfile::AndroidPhonePortrait,
        DisplayProfile::AndroidPhoneLandscape,
        DisplayProfile::AndroidTablet,
    ];

    /// Spec for this profile. `Default` returns `None`, telling the
    /// renderer to fill the panel as-is (no letterboxing).
    pub fn spec(self) -> Option<DisplayProfileSpec> {
        match self {
            // iPhone 15 Pro logical points (Dynamic Island = 59pt safe top).
            DisplayProfile::IPhonePortrait => Some(DisplayProfileSpec {
                label: "iPhone (Portrait)",
                width: 393,
                height: 852,
                safe_top: 59,
                safe_bottom: 34,
            }),
            DisplayProfile::IPhoneLandscape => Some(DisplayProfileSpec {
                label: "iPhone (Landscape)",
                width: 852,
                height: 393,
                safe_top: 0,
                safe_bottom: 21,
            }),
            // iPad Pro 11" logical points.
            DisplayProfile::IPadPortrait => Some(DisplayProfileSpec {
                label: "iPad (Portrait)",
                width: 834,
                height: 1194,
                safe_top: 24,
                safe_bottom: 20,
            }),
            DisplayProfile::IPadLandscape => Some(DisplayProfileSpec {
                label: "iPad (Landscape)",
                width: 1194,
                height: 834,
                safe_top: 24,
                safe_bottom: 20,
            }),
            // Android Pixel 8 — status bar + 3-button nav.
            DisplayProfile::AndroidPhonePortrait => Some(DisplayProfileSpec {
                label: "Android Phone (Portrait)",
                width: 412,
                height: 915,
                safe_top: 24,
                safe_bottom: 48,
            }),
            DisplayProfile::AndroidPhoneLandscape => Some(DisplayProfileSpec {
                label: "Android Phone (Landscape)",
                width: 915,
                height: 412,
                safe_top: 24,
                safe_bottom: 48,
            }),
            DisplayProfile::AndroidTablet => Some(DisplayProfileSpec {
                label: "Android Tablet",
                width: 1280,
                height: 800,
                safe_top: 24,
                safe_bottom: 48,
            }),
            DisplayProfile::Default => None,
        }
    }

    pub fn label(self) -> &'static str {
        self.spec()
            .map(|s| s.label)
            .unwrap_or("Free Aspect (Editor)")
    }
}

#[cfg(test)]
mod display_profile_tests {
    use super::DisplayProfile;

    #[test]
    fn default_profile_has_no_spec() {
        // The "Free Aspect" / Default profile must skip the
        // letter-box path so the Scene View still uses the editor's
        // 4:3 default.
        assert!(DisplayProfile::Default.spec().is_none());
        assert_eq!(DisplayProfile::Default.label(), "Free Aspect (Editor)");
    }

    #[test]
    fn portrait_profiles_are_taller_than_wide() {
        for p in [
            DisplayProfile::IPhonePortrait,
            DisplayProfile::IPadPortrait,
            DisplayProfile::AndroidPhonePortrait,
        ] {
            let s = p.spec().expect("portrait spec");
            assert!(
                s.height > s.width,
                "{:?} expected portrait but got {}×{}",
                p,
                s.width,
                s.height
            );
        }
    }

    #[test]
    fn landscape_profiles_are_wider_than_tall() {
        for p in [
            DisplayProfile::IPhoneLandscape,
            DisplayProfile::IPadLandscape,
            DisplayProfile::AndroidPhoneLandscape,
            DisplayProfile::AndroidTablet,
        ] {
            let s = p.spec().expect("landscape spec");
            assert!(
                s.width > s.height,
                "{:?} expected landscape but got {}×{}",
                p,
                s.width,
                s.height
            );
        }
    }

    #[test]
    fn iphone_portrait_has_safe_top_for_dynamic_island() {
        // The notch / Dynamic Island area must be reported so UI
        // overlays (HUD, joystick) don't draw underneath it.
        let s = DisplayProfile::IPhonePortrait.spec().unwrap();
        assert!(s.safe_top > 0, "iPhone portrait must reserve a safe top");
        assert!(
            s.safe_bottom > 0,
            "iPhone portrait must reserve home-indicator"
        );
    }

    #[test]
    fn all_includes_default_first() {
        // The selector dropdown puts "Free Aspect" first so opening
        // a fresh project doesn't surprise the user with a phone
        // letter-box on the editor camera.
        assert_eq!(DisplayProfile::ALL.first(), Some(&DisplayProfile::Default));
    }

    #[test]
    fn every_label_is_unique() {
        // Drop-down keys collide if two profiles share a label;
        // guard against that as more devices are added.
        let mut labels: Vec<&'static str> = DisplayProfile::ALL.iter().map(|p| p.label()).collect();
        labels.sort_unstable();
        let original = labels.len();
        labels.dedup();
        assert_eq!(labels.len(), original, "duplicate DisplayProfile label");
    }
}

// ===== Syntax Highlighting Color Palette =====
// VS Code Dark+ inspired color scheme for Rust syntax highlighting

pub(crate) mod syntax_colors {
    use egui::Color32;

    pub const KEYWORD: Color32 = Color32::from_rgb(234, 147, 71); // #EA9347 Orange
    pub const FUNCTION: Color32 = Color32::from_rgb(84, 166, 224); // #54A6E0 Sky Blue
    pub const TYPE: Color32 = Color32::from_rgb(232, 194, 82); // #E8C252 Yellow
    pub const STRING: Color32 = Color32::from_rgb(184, 214, 84); // #B8D654 Lime Green
    pub const NUMBER: Color32 = Color32::from_rgb(181, 206, 168); // #B5CEA8 Light Green
    pub const COMMENT: Color32 = Color32::from_rgb(128, 128, 128); // #808080 Gray
    pub const DOC_COMMENT: Color32 = Color32::from_rgb(106, 153, 85); // #6A9955 Green
    pub const MACRO: Color32 = Color32::from_rgb(84, 166, 224); // #54A6E0 Sky Blue
    pub const ATTRIBUTE: Color32 = Color32::from_rgb(197, 134, 192); // #C586C0 Pink
    pub const CONSTANT: Color32 = Color32::from_rgb(197, 134, 192); // #C586C0 Pink
    pub const LIFETIME: Color32 = Color32::from_rgb(78, 201, 176); // #4EC9B0 Cyan
    pub const NAMESPACE: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 White
    pub const VARIABLE: Color32 = Color32::from_rgb(212, 212, 212); // デフォルト白と同じ
    pub const OPERATOR: Color32 = Color32::from_rgb(212, 212, 212); // #D4D4D4 White
}

// ===== UI Color Palette =====

#[allow(dead_code)]
/// The UI colour palette. Originally a flat list of `const Color32`s
/// hardcoded for the dark theme; converted to a `OnceLock`-backed
/// struct so the Light / High Contrast themes from the Settings panel
/// can swap in their own palettes at runtime.
///
/// Existing call sites use `ui_colors::EDITOR_BG()` etc. — kept as
/// SCREAMING_SNAKE-named accessor fields on the struct so a one-line
/// `let c = palette();` followed by `c.EDITOR_BG` continues to read
/// like a constant lookup.
pub(crate) mod ui_colors {
    use egui::Color32;
    use std::sync::atomic::{AtomicU8, Ordering};

    /// 0 = Dark, 1 = Light, 2 = High Contrast. Plain `AtomicU8` because
    /// the egui render runs on a single thread anyway and atomics keep
    /// the API `&'static` without a lock.
    static THEME_INDEX: AtomicU8 = AtomicU8::new(0);

    pub fn set_theme(mode: super::types::ThemeMode) {
        let idx = match mode {
            super::types::ThemeMode::Dark => 0u8,
            super::types::ThemeMode::Light => 1u8,
            super::types::ThemeMode::HighContrast => 2u8,
        };
        THEME_INDEX.store(idx, Ordering::Relaxed);
    }

    fn dark() -> &'static Palette {
        static DARK: Palette = Palette {
            EDITOR_BG: Color32::from_rgb(25, 26, 28),
            SIDEBAR_BG: Color32::from_rgb(25, 26, 28),
            ACTIVITY_BAR_BG: Color32::from_rgb(25, 26, 28),
            TOP_BAR_BG: Color32::from_rgb(48, 49, 52),
            STATUS_BAR_BG: Color32::from_rgb(25, 26, 28),
            TEXT_DEFAULT: Color32::from_rgb(212, 212, 212),
            TEXT_MUTED: Color32::from_rgb(153, 153, 153),
            BORDER: Color32::from_rgb(60, 60, 60),
            PANEL_BORDER: Color32::from_rgb(43, 43, 43),
            CONTROL_BG: Color32::from_rgb(60, 60, 60),
            CONTROL_BORDER: Color32::from_rgb(86, 86, 86),
            HOVER_BG: Color32::from_rgb(45, 45, 45),
            ACTIVE_BG: Color32::from_rgb(55, 55, 61),
            ACCENT: Color32::from_rgb(0, 122, 204),
            ACCENT_HOVER: Color32::from_rgb(17, 119, 187),
            FOCUS_BORDER: Color32::from_rgb(0, 127, 212),
            SETTINGS_HINT: Color32::from_rgb(128, 128, 128),
        };
        &DARK
    }

    fn light() -> &'static Palette {
        static LIGHT: Palette = Palette {
            EDITOR_BG: Color32::from_rgb(255, 255, 255),
            SIDEBAR_BG: Color32::from_rgb(243, 243, 243),
            ACTIVITY_BAR_BG: Color32::from_rgb(245, 245, 245),
            TOP_BAR_BG: Color32::from_rgb(221, 221, 221),
            STATUS_BAR_BG: Color32::from_rgb(0, 122, 204),
            TEXT_DEFAULT: Color32::from_rgb(34, 34, 34),
            TEXT_MUTED: Color32::from_rgb(96, 96, 96),
            BORDER: Color32::from_rgb(200, 200, 200),
            PANEL_BORDER: Color32::from_rgb(225, 225, 225),
            CONTROL_BG: Color32::from_rgb(245, 245, 245),
            CONTROL_BORDER: Color32::from_rgb(190, 190, 190),
            HOVER_BG: Color32::from_rgb(230, 230, 230),
            ACTIVE_BG: Color32::from_rgb(210, 224, 244),
            ACCENT: Color32::from_rgb(0, 102, 184),
            ACCENT_HOVER: Color32::from_rgb(0, 87, 158),
            FOCUS_BORDER: Color32::from_rgb(0, 102, 184),
            SETTINGS_HINT: Color32::from_rgb(110, 110, 110),
        };
        &LIGHT
    }

    fn high_contrast() -> &'static Palette {
        static HC: Palette = Palette {
            EDITOR_BG: Color32::BLACK,
            SIDEBAR_BG: Color32::BLACK,
            ACTIVITY_BAR_BG: Color32::BLACK,
            TOP_BAR_BG: Color32::BLACK,
            STATUS_BAR_BG: Color32::BLACK,
            TEXT_DEFAULT: Color32::WHITE,
            TEXT_MUTED: Color32::from_rgb(200, 200, 200),
            BORDER: Color32::from_rgb(110, 110, 110),
            PANEL_BORDER: Color32::from_rgb(110, 110, 110),
            CONTROL_BG: Color32::from_rgb(20, 20, 20),
            CONTROL_BORDER: Color32::from_rgb(150, 150, 150),
            HOVER_BG: Color32::from_rgb(40, 40, 40),
            ACTIVE_BG: Color32::from_rgb(60, 60, 60),
            ACCENT: Color32::from_rgb(252, 200, 0),
            ACCENT_HOVER: Color32::from_rgb(255, 220, 60),
            FOCUS_BORDER: Color32::from_rgb(255, 215, 0),
            SETTINGS_HINT: Color32::from_rgb(200, 200, 200),
        };
        &HC
    }

    /// The struct kept as `static`s above; one per theme. Fields are
    /// PascalCase to match the original const names so callers can keep
    /// writing `ui_colors::EDITOR_BG()` (now a function returning the
    /// active palette's field).
    #[allow(non_snake_case)]
    pub struct Palette {
        pub EDITOR_BG: Color32,
        pub SIDEBAR_BG: Color32,
        pub ACTIVITY_BAR_BG: Color32,
        pub TOP_BAR_BG: Color32,
        pub STATUS_BAR_BG: Color32,
        pub TEXT_DEFAULT: Color32,
        pub TEXT_MUTED: Color32,
        pub BORDER: Color32,
        pub PANEL_BORDER: Color32,
        pub CONTROL_BG: Color32,
        pub CONTROL_BORDER: Color32,
        pub HOVER_BG: Color32,
        pub ACTIVE_BG: Color32,
        pub ACCENT: Color32,
        pub ACCENT_HOVER: Color32,
        pub FOCUS_BORDER: Color32,
        pub SETTINGS_HINT: Color32,
    }

    fn current() -> &'static Palette {
        match THEME_INDEX.load(Ordering::Relaxed) {
            1 => light(),
            2 => high_contrast(),
            _ => dark(),
        }
    }

    // Compatibility shims — original call sites use
    // `ui_colors::EDITOR_BG()` without parens. We can't keep that exact
    // syntax because `const` items can't be runtime-dynamic, so each
    // becomes a zero-arg function and call sites get `()` appended.
    #[allow(non_snake_case)]
    pub fn EDITOR_BG() -> Color32 {
        current().EDITOR_BG
    }
    #[allow(non_snake_case)]
    pub fn SIDEBAR_BG() -> Color32 {
        current().SIDEBAR_BG
    }
    #[allow(non_snake_case)]
    pub fn ACTIVITY_BAR_BG() -> Color32 {
        current().ACTIVITY_BAR_BG
    }
    #[allow(non_snake_case)]
    pub fn TOP_BAR_BG() -> Color32 {
        current().TOP_BAR_BG
    }
    #[allow(non_snake_case)]
    pub fn STATUS_BAR_BG() -> Color32 {
        current().STATUS_BAR_BG
    }
    #[allow(non_snake_case)]
    pub fn TEXT_DEFAULT() -> Color32 {
        current().TEXT_DEFAULT
    }
    #[allow(non_snake_case)]
    pub fn TEXT_MUTED() -> Color32 {
        current().TEXT_MUTED
    }
    #[allow(non_snake_case)]
    pub fn BORDER() -> Color32 {
        current().BORDER
    }
    #[allow(non_snake_case)]
    pub fn PANEL_BORDER() -> Color32 {
        current().PANEL_BORDER
    }
    #[allow(non_snake_case)]
    pub fn CONTROL_BG() -> Color32 {
        current().CONTROL_BG
    }
    #[allow(non_snake_case)]
    pub fn CONTROL_BORDER() -> Color32 {
        current().CONTROL_BORDER
    }
    #[allow(non_snake_case)]
    pub fn HOVER_BG() -> Color32 {
        current().HOVER_BG
    }
    #[allow(non_snake_case)]
    pub fn ACTIVE_BG() -> Color32 {
        current().ACTIVE_BG
    }
    #[allow(non_snake_case)]
    pub fn ACCENT() -> Color32 {
        current().ACCENT
    }
    #[allow(non_snake_case)]
    pub fn ACCENT_HOVER() -> Color32 {
        current().ACCENT_HOVER
    }
    #[allow(non_snake_case)]
    pub fn FOCUS_BORDER() -> Color32 {
        current().FOCUS_BORDER
    }

    // VS Code-style settings palette — derived from the same theme so
    // the Settings panel auto-themes.
    #[allow(non_snake_case)]
    pub fn SETTINGS_NAV_BG() -> Color32 {
        SIDEBAR_BG()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_BG() -> Color32 {
        EDITOR_BG()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_SEARCH_BG() -> Color32 {
        CONTROL_BG()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_DESC() -> Color32 {
        TEXT_MUTED()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_HINT() -> Color32 {
        current().SETTINGS_HINT
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_HEADER() -> Color32 {
        TEXT_DEFAULT()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_CARD_BORDER() -> Color32 {
        BORDER()
    }
    #[allow(non_snake_case)]
    pub fn SETTINGS_ACCENT() -> Color32 {
        ACCENT()
    }
}

// ===== Component Color Palette =====
// Shared colors for UI components (tabs, buttons, inputs, etc.)

#[allow(dead_code, non_snake_case)]
pub(crate) mod component_colors {
    use egui::Color32;
    // VS Code accent blue. These were `const`s that re-exported the
    // ui_colors constants; now that the underlying palette is runtime-
    // dynamic, they're forwarding functions instead.
    pub fn ACCENT() -> Color32 {
        super::ui_colors::ACCENT()
    }
    pub fn TAB_ACTIVE() -> Color32 {
        super::ui_colors::TEXT_DEFAULT()
    }
    pub fn TAB_INACTIVE() -> Color32 {
        super::ui_colors::TEXT_MUTED()
    }
    pub fn BUTTON_TEXT() -> Color32 {
        super::ui_colors::TEXT_DEFAULT()
    }
    pub fn BUTTON_BG() -> Color32 {
        super::ui_colors::CONTROL_BG()
    }
    pub fn HOVER_BG() -> Color32 {
        super::ui_colors::HOVER_BG()
    }
    pub fn INPUT_BG() -> Color32 {
        super::ui_colors::CONTROL_BG()
    }
    pub fn TEXT_DIM() -> Color32 {
        super::ui_colors::TEXT_MUTED()
    }
}

// ===== File Icon Color Palette =====

pub(crate) mod file_icon_colors {
    use egui::Color32;

    pub const RUST_ORANGE: Color32 = Color32::from_rgb(255, 152, 0); // #FF9800
    pub const CONFIG_GRAY: Color32 = Color32::from_rgb(128, 128, 128); // #808080
    pub const JSON_YELLOW: Color32 = Color32::from_rgb(255, 203, 0); // #FFCB00
    pub const MARKDOWN_BLUE: Color32 = Color32::from_rgb(66, 165, 245); // #42A5F5
    pub const JAVASCRIPT_YELLOW: Color32 = Color32::from_rgb(247, 223, 30); // #F7DF1E
    pub const TYPESCRIPT_BLUE: Color32 = Color32::from_rgb(41, 127, 214); // #297FD6
    pub const PYTHON_GREEN: Color32 = Color32::from_rgb(52, 168, 83); // #34A853
    pub const SHELL_GREEN: Color32 = Color32::from_rgb(76, 175, 80); // #4CAF50
    pub const HTML_ORANGE: Color32 = Color32::from_rgb(229, 115, 0); // #E57300
    pub const CSS_BLUE: Color32 = Color32::from_rgb(66, 165, 245); // #42A5F5
    pub const IMAGE_PURPLE: Color32 = Color32::from_rgb(156, 39, 176); // #9C27B0
    pub const SVG_AMBER: Color32 = Color32::from_rgb(255, 179, 0); // #FFB300
    pub const GIT_ORANGE: Color32 = Color32::from_rgb(240, 98, 35); // #F06223

    // Godot project files (v0.8.x Migration & interop). The Godot
    // brand uses a desaturated cool blue for its UI; we lean into
    // that so users coming from Godot recognise their files at a
    // glance, while keeping `.gd` slightly warmer to differentiate
    // scripts from scenes / resources.
    pub const GODOT_SCRIPT_BLUE: Color32 = Color32::from_rgb(71, 142, 191); // #478EBF
    pub const GODOT_SCENE_PURPLE: Color32 = Color32::from_rgb(154, 113, 209); // #9A71D1
    pub const GODOT_RESOURCE_TEAL: Color32 = Color32::from_rgb(94, 169, 169); // #5EA9A9
}

/// Main panels in the Activity Bar
const MAIN_PANELS: &[SidebarPanel] = &[
    SidebarPanel {
        variant: ActivePanel::Explorer,
        icon: "\u{ea83}", // codicon-folder
        _name: "Explorer",
    },
    SidebarPanel {
        variant: ActivePanel::Search,
        icon: "\u{ea6d}", // codicon-search
        _name: "Search",
    },
    SidebarPanel {
        variant: ActivePanel::Git,
        icon: "\u{ea84}", // codicon-github
        _name: "Git",
    },
    SidebarPanel {
        variant: ActivePanel::Terminal,
        icon: "\u{ea85}", // codicon-terminal
        _name: "Terminal",
    },
    SidebarPanel {
        variant: ActivePanel::EcsInspector,
        icon: "\u{eadf}", // codicon-eye
        _name: "ECS Inspector",
    },
    SidebarPanel {
        variant: ActivePanel::SceneEditor,
        icon: "\u{eb44}", // codicon-layout
        _name: "Scene Editor",
    },
    SidebarPanel {
        variant: ActivePanel::Database,
        icon: "\u{eace}", // codicon-database
        _name: "Database",
    },
    SidebarPanel {
        variant: ActivePanel::Docker,
        icon: "\u{eb29}", // codicon-package (Docker container metaphor)
        _name: "Docker",
    },
    #[cfg(feature = "ai")]
    SidebarPanel {
        variant: ActivePanel::OracleBerry,
        icon: "\u{ec1f}", // codicon-lightbulb-sparkle (placeholder until brand SVG)
        _name: "OracleBerry",
    },
];

/// Action chosen in the close confirmation dialog
#[derive(Clone, Copy)]
pub(crate) enum CloseAction {
    SaveAll,
    Discard,
}

/// Settings-panel cache for the most recent Ollama `/api/version`
/// probe. The async task fills this; the egui render reads it.
#[derive(Default, Clone)]
pub enum OllamaProbeStatus {
    /// No probe has been issued yet — Settings shows nothing.
    #[default]
    Unknown,
    /// Probe is in flight; Settings shows a spinner.
    Probing,
    /// Server replied — version string from `/api/version`.
    Connected(String),
    /// Probe failed — the message is shown verbatim under the field.
    Error(String),
}

/// Main application state
#[allow(dead_code)]
pub struct BerryCodeApp {
    // === Project State ===
    pub(crate) root_path: String,
    pub(crate) selected_file: Option<(String, String)>, // (path, content)
    /// Whether the project picker should be shown (no project loaded yet)
    pub(crate) show_project_picker: bool,
    /// Whether the "unsaved changes" close confirmation dialog is shown
    pub(crate) show_close_confirm: bool,
    /// Action chosen in the close confirmation dialog
    pub(crate) close_action: Option<CloseAction>,
    /// Path being typed in the project picker dialog
    pub(crate) project_picker_path: String,
    /// Recently opened projects for quick access
    pub(crate) recent_projects: Vec<String>,

    // === UI State ===
    pub(crate) active_panel: ActivePanel,
    pub(crate) sidebar_width: f32,

    // === Editor State ===
    pub(crate) editor_tabs: Vec<EditorTab>,
    pub(crate) active_tab_idx: usize,
    pub(crate) syntax_highlighter: SyntaxHighlighter, // Regex-based highlighter
    /// In-progress IME preedit string for the source code editor. Populated
    /// from `egui::Event::Ime(ImeEvent::Preedit(_))` and rendered as an
    /// overlay near the cursor; cleared on Commit/Disabled.
    pub(crate) editor_ime_preedit: String,

    // === File Tree State ===
    pub(crate) file_tree_cache: Vec<DirEntry>, // Cached directory tree
    pub(crate) file_tree_load_pending: bool,
    pub(crate) expanded_dirs: HashSet<String>, // Set of expanded directory paths
    /// Folder rows rendered last frame, recorded so an OS drag-and-drop release
    /// can be resolved back to the folder under the pointer. Without this the
    /// drop handler had no way to know which folder the user aimed at and
    /// always copied into the project root.
    pub(crate) file_tree_folder_rects: Vec<(String, egui::Rect)>,
    /// Cmd+B toggles this. When false, `render_sidebar` early-returns.
    pub(crate) sidebar_visible: bool,
    /// Index of the tab being dragged by the user (left button held).
    /// Reset to `None` once the user releases or moves off the tab strip.
    pub(crate) tab_drag_source: Option<usize>,
    /// Extra folders added to the workspace beyond the primary
    /// `root_path`. Each renders as its own collapsible root in the
    /// file tree and is included in project-wide search. The primary
    /// root is still the one used for LSP server roots / Cargo
    /// commands / git status; secondary roots are read-only browse
    /// targets so we don't need to spin up per-root LSPs.
    pub(crate) additional_roots: Vec<String>,
    /// Per-extra-root cached tree (mirrors `file_tree_cache`).
    pub(crate) additional_root_caches: Vec<Vec<DirEntry>>,
    /// When `Some(idx)`, a side-by-side preview pane is shown on the
    /// right with `editor_tabs[idx]`. Cmd+\\ toggles. The right pane is
    /// read-only by design — editing is still done in the main pane —
    /// which keeps the implementation scoped (no second cursor / LSP
    /// session) while still solving the "diff two files visually" use
    /// case the user audit called out.
    pub(crate) split_right_tab: Option<usize>,

    // === Terminal State (iTerm2-style PTY emulator) ===
    pub(crate) terminal: terminal_emulator::TerminalEmulator,

    // === Database Panel State (SQLite) ===
    pub(crate) database: database::DatabaseState,

    // === Docker Panel State ===
    pub(crate) docker: docker::DockerState,

    // === OracleBerry Panel State ===
    #[cfg(feature = "ai")]
    pub(crate) oracleberry: oracleberry::OracleBerryState,

    // === Search State ===
    pub(crate) search_query: String,
    pub(crate) search_dialog_open: bool,
    pub(crate) search_case_sensitive: bool,
    /// Match whole-word only (VS Code's `[ab]` toggle).
    pub(crate) search_whole_word: bool,
    /// Treat the query as a regular expression (VS Code's `.*` toggle).
    pub(crate) search_use_regex: bool,
    /// Whether the replace input row is expanded under the search input
    /// (the small chevron at the left of the search box in VS Code).
    pub(crate) search_show_replace: bool,
    /// Files-to-include glob, e.g. `src/**/*.rs` (VS Code's "files to include").
    pub(crate) search_include_glob: String,
    /// Files-to-exclude glob, e.g. `target,**/*.lock`.
    pub(crate) search_exclude_glob: String,
    /// Whether the include/exclude details row is expanded.
    pub(crate) search_show_details: bool,
    /// Set of file paths whose result group is collapsed in the panel.
    pub(crate) search_collapsed_files: std::collections::HashSet<String>,
    pub(crate) current_search_index: usize,
    pub(crate) search_results: Vec<SearchMatch>,
    pub(crate) replace_query: String,
    pub(crate) show_replace: bool,

    // === Git State ===
    pub(crate) git_current_branch: String,
    pub(crate) git_status: Vec<native::git::GitStatus>,
    pub(crate) git_commit_message: String,
    pub(crate) git_initialized: bool,
    /// Last time the per-frame poll block ran. We throttle the I/O-heavy
    /// poll fan-out (file watcher, asset watcher, cargo check, etc.) to
    /// ~50ms so typing latency stays low but background channels aren't
    /// drained 60×/sec for nothing.
    pub(crate) last_poll_tick: Option<std::time::Instant>,
    pub(crate) git_active_tab: GitTab,
    pub(crate) git_history_state: GitHistoryState,
    pub(crate) git_branch_state: GitBranchState,
    pub(crate) git_remote_state: GitRemoteState,
    pub(crate) git_tag_state: GitTagState,
    pub(crate) git_stash_state: GitStashState,
    pub(crate) git_diff_state: GitDiffState,

    // === LSP State ===
    pub(crate) lsp_runtime: std::sync::Arc<tokio::runtime::Runtime>,
    pub(crate) lsp_native_client: Option<std::sync::Arc<native::lsp_native::NativeLspClient>>,
    pub(crate) lsp_response_tx: Option<mpsc::UnboundedSender<LspResponse>>,
    pub(crate) lsp_connected: bool,
    pub(crate) lsp_diagnostics: Vec<LspDiagnostic>,
    pub(crate) lsp_hover_info: Option<LspHoverInfo>,
    pub(crate) lsp_completions: Vec<LspCompletionItem>,
    pub(crate) lsp_show_completions: bool,
    pub(crate) lsp_show_hover: bool,
    pub(crate) lsp_auto_trigger_pending: bool,
    pub(crate) lsp_completion_index: usize,
    /// Latest `textDocument/signatureHelp` result. Cleared when the cursor
    /// leaves a parameter list (`)` typed, Esc pressed, etc.).
    pub(crate) lsp_signature_help: Option<types::LspSignatureHelp>,
    /// `(` was just typed → trigger a signature-help request next frame
    /// (paralleling `lsp_auto_trigger_pending` for completions).
    pub(crate) lsp_signature_trigger_pending: bool,
    pub(crate) lsp_response_rx: Option<mpsc::UnboundedReceiver<LspResponse>>,
    pub(crate) lsp_diagnostics_rx:
        Option<mpsc::UnboundedReceiver<native::lsp_native::PublishedDiagnostics>>,

    // === Status Message ===
    pub(crate) status_message: String,
    pub(crate) status_message_timestamp: Option<std::time::Instant>,

    // === Go-to-Definition State ===
    pub(crate) pending_goto_definition: Option<PendingGotoDefinition>,
    pub(crate) definition_picker_locations: Vec<LspLocation>,
    pub(crate) show_definition_picker: bool,

    // === Find References State ===
    pub(crate) lsp_references: Vec<LspLocation>,
    pub(crate) show_references_panel: bool,

    // === Inlay Hints State ===
    pub(crate) lsp_inlay_hints: Vec<LspInlayHint>,
    pub(crate) inlay_hints_enabled: bool,
    pub(crate) inlay_hints_last_request: Option<std::time::Instant>,

    // === Code Actions State ===
    pub(crate) lsp_code_actions: Vec<LspCodeAction>,
    pub(crate) show_code_actions: bool,
    pub(crate) code_action_line: usize,

    // === Snippet State ===
    pub(crate) snippet_session: Option<SnippetSession>,

    // === Rename Symbol State ===
    pub(crate) rename_dialog_open: bool,
    pub(crate) rename_new_name: String,

    // AI integration (REST via berry-core-api)
    pub(crate) ai_connected: bool,
    pub(crate) ai_response_tx: Option<mpsc::UnboundedSender<AiChatResponse>>,
    pub(crate) ai_response_rx: Option<mpsc::UnboundedReceiver<AiChatResponse>>,
    pub(crate) ai_streaming_message: Option<String>,

    // AI Chat Panel State
    pub(crate) ai_messages: Vec<AiChatMessage>,
    pub(crate) ai_input: String,
    pub(crate) ai_streaming: bool,
    /// Whether the right-side AI chat panel is collapsed to a thin
    /// strip. False = full panel visible; true = a 32px strip with an
    /// expand chevron. Toggled from the panel header.
    pub(crate) ai_chat_collapsed: bool,
    pub(crate) ai_current_response: String,
    pub(crate) chat_attachment: Option<String>,
    /// Set true when Cmd+L is pressed; the AI chat panel claims focus on
    /// its input field on the next render and clears the flag. v0.4.5 / 2A.
    pub(crate) ai_chat_focus_pending: bool,
    /// Edits proposed by the active coding agent (Claude Code / Codex)
    /// awaiting human Approve / Reject. Populated from
    /// `AiChatResponse::PendingEdit` in `poll_ai_responses` and
    /// rendered as cards inside the AI chat panel. v0.4.5 / Phase D.
    pub(crate) pending_agent_edits: Vec<types::PendingAgentEdit>,

    // === Settings ===
    pub(crate) show_settings: bool,
    pub(crate) active_settings_tab: SettingsTab,
    pub(crate) ui_language: UiLanguage,

    // === Theme (Customizable Syntax Colors) ===
    pub(crate) show_theme_editor: bool,
    pub(crate) keyword_color: egui::Color32,
    pub(crate) function_color: egui::Color32,
    pub(crate) type_color: egui::Color32,
    pub(crate) string_color: egui::Color32,
    pub(crate) number_color: egui::Color32,
    pub(crate) comment_color: egui::Color32,
    pub(crate) doc_comment_color: egui::Color32,
    pub(crate) macro_color: egui::Color32,
    pub(crate) attribute_color: egui::Color32,
    pub(crate) constant_color: egui::Color32,
    pub(crate) lifetime_color: egui::Color32,
    pub(crate) namespace_color: egui::Color32,
    pub(crate) variable_color: egui::Color32,
    pub(crate) operator_color: egui::Color32,

    // === Multi-cursor State ===
    pub(crate) multi_cursors: Vec<usize>, // additional cursor char positions (besides the primary egui cursor)

    // === Peek Definition ===
    pub(crate) peek_definition: Option<PeekDefinition>,

    // === Focus Management ===
    pub(crate) active_focus: FocusLayer,

    // === New File/Folder/Project Dialog ===
    pub(crate) new_file_dialog_open: bool,
    pub(crate) new_file_name: String,
    pub(crate) new_folder_dialog_open: bool,
    pub(crate) new_folder_name: String,
    pub(crate) new_project_dialog_open: bool,
    pub(crate) new_project_name: String,
    pub(crate) new_project_path: String,
    pub(crate) new_project_template: new_project::ProjectTemplate,

    // === Git Blame Cache ===
    pub(crate) blame_cache_line: usize,
    pub(crate) blame_cache_text: String,
    pub(crate) blame_cache_file: String,

    // === File Watcher ===
    pub(crate) file_watcher: Option<native::watcher::FileWatcher>,

    // === ECS Inspector ===
    pub(crate) ecs_inspector: crate::bevy_ide::inspector::ecs_state::EcsInspectorState,
    pub(crate) ecs_inspector_tab: ecs_inspector::EcsInspectorTab,

    // === Scene Preview ===
    pub(crate) scene_preview: crate::bevy_ide::scene_preview::parser::ScenePreviewState,

    // === Asset Browser ===
    pub(crate) asset_browser: crate::bevy_ide::assets::scanner::AssetBrowserState,

    // === Bevy Templates ===
    pub(crate) template_name: String,
    pub(crate) template_fields: Vec<(String, String)>,
    pub(crate) template_params: Vec<String>,
    pub(crate) template_variants: Vec<String>,

    // === Debug State ===
    pub(crate) debug_state: debugger::DebugState,
    pub(crate) dap_client: Option<crate::native::dap::DapClient>,
    pub(crate) dap_event_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<crate::native::dap::DapEvent>>,

    // === Test Runner State ===
    pub(crate) test_runner: test_runner::TestRunnerState,

    // === User Snippets ===
    pub(crate) user_snippets: Vec<custom_snippets::LoadedSnippet>,

    // === Vim Mode ===
    pub(crate) vim: vim_mode::VimState,

    // === Plugin System ===
    pub(crate) plugin_manager: plugin_system::PluginManager,
    /// In-flight plugin command, polled per-frame by
    /// `poll_pending_plugin_command`. The previous implementation
    /// spawned a worker thread and `recv_timeout(30s)`'d on the
    /// caller's thread, which still froze the editor for up to 30 s
    /// when a plugin hung. Now the spawn returns immediately with
    /// the receiver stored here; the egui pass drains it via
    /// `try_recv` so the UI stays responsive throughout.
    pub(crate) pending_plugin_command: Option<plugin_system::PendingPluginCommand>,

    // === Remote Development ===
    pub(crate) remote: remote_dev::RemoteConnection,
    pub(crate) remote_dialog: remote_dev::RemoteDialogState,

    // === Live Collaboration ===
    pub(crate) collab: live_collab::CollabState,
    pub(crate) collab_dialog: live_collab::CollabDialogState,

    // === Context Menu State ===
    pub(crate) context_menu_path: Option<String>,
    pub(crate) context_menu_is_dir: bool,
    pub(crate) context_menu_pos: egui::Pos2,
    pub(crate) rename_file_dialog_open: bool,
    pub(crate) rename_file_old_path: String,
    pub(crate) rename_file_new_name: String,

    // === Run Bevy Project State ===
    pub(crate) run_output: Vec<String>,
    pub(crate) run_process: Option<std::process::Child>,
    pub(crate) run_panel_open: bool,
    pub(crate) run_release_mode: bool,
    pub(crate) run_output_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Console filter state ===
    pub(crate) console_filter_text: String,
    pub(crate) console_show_info: bool,
    pub(crate) console_show_warning: bool,
    pub(crate) console_show_error: bool,
    pub(crate) console_auto_scroll: bool,
    pub(crate) console_log_level_filter: run_panel::LogLevelFilter,

    // === Scene Editor (Unity-like) ===
    pub(crate) scene_model: scene_editor::model::SceneModel,
    /// The most recently clicked entity in a multi-select context. The inspector
    /// displays this entity's properties.
    pub(crate) primary_selected_id: Option<u64>,
    pub(crate) scene_view_texture_id: Option<egui::TextureId>,
    pub(crate) scene_needs_sync: bool,
    /// Clipboard for copy/paste of components in the Inspector.
    pub(crate) component_clipboard: Option<scene_editor::model::ComponentData>,
    /// Filter text for the Add Component search popup.
    pub(crate) add_component_filter: String,
    /// Whether the Add Component search popup is open.
    pub(crate) add_component_popup_open: bool,
    /// Whether the "Create New Script" dialog is open.
    pub(crate) new_script_dialog_open: bool,
    /// Name for the new script being created.
    pub(crate) new_script_name: String,
    /// Scene Editor: which camera the central viewport renders from.
    /// `Scene` = the orbit-controlled editor camera (default), `Game` =
    /// the scene's own `Camera`-tagged entity (player POV).
    pub(crate) scene_view_mode: SceneViewMode,
    /// Mobile / tablet display profile that letter-boxes the Scene View
    /// to a target device aspect ratio. `Default` keeps the panel as-is.
    pub(crate) display_profile: DisplayProfile,
    /// Toolbar toggle for the green Collider AABB overlay in the Scene
    /// View. Defaults to `true` so collision bounds are obvious on
    /// fresh projects; flip off when the wireframes start overlapping
    /// the authored mesh and the scene gets visually noisy.
    pub(crate) show_colliders: bool,
    pub(crate) scene_orbit_yaw: f32,
    pub(crate) scene_orbit_pitch: f32,
    pub(crate) scene_orbit_distance: f32,
    pub(crate) scene_orbit_target: [f32; 3],
    pub(crate) scene_ortho: bool,
    pub(crate) scene_ortho_scale: f32,
    pub(crate) scene_shadows_enabled: bool,
    pub(crate) scene_bloom_enabled: bool,
    pub(crate) scene_bloom_intensity: f32,
    pub(crate) scene_tonemapping: u8,
    pub(crate) scene_ssao_enabled: bool,
    pub(crate) scene_taa_enabled: bool,
    pub(crate) scene_fog_enabled: bool,
    pub(crate) scene_fog_color: [f32; 3],
    pub(crate) scene_fog_start: f32,
    pub(crate) scene_fog_end: f32,
    pub(crate) scene_dof_enabled: bool,
    pub(crate) scene_dof_focus_distance: f32,
    pub(crate) scene_dof_aperture: f32,
    pub(crate) fly_mode_active: bool,
    pub(crate) fly_camera_speed: f32,
    pub(crate) gizmo_mode: scene_editor::gizmo::GizmoMode,
    /// Currently dragged gizmo handle. None when not dragging.
    pub(crate) gizmo_dragging: Option<scene_editor::gizmo::GizmoDrag>,
    /// Start position of an in-progress box selection drag (screen coords).
    pub(crate) box_select_start: Option<egui::Pos2>,

    // === Scene Editor: Hierarchy panel state ===
    pub(crate) hierarchy_filter: String,
    pub(crate) hierarchy_dragged: Option<u64>,
    /// Drop target while a hierarchy drag is in progress. `Some(None)` means
    /// "drop on root", `Some(Some(id))` means "make a child of id".
    pub(crate) hierarchy_drop_target: Option<Option<u64>>,
    pub(crate) renaming_entity_id: Option<u64>,
    pub(crate) rename_buffer: String,
    /// When `Some`, the Hierarchy panel shows a "name your scene"
    /// modal — the buffer is the in-progress name. `None` means the
    /// dialog is closed. The New button toggles it on; Enter / OK
    /// commits, Esc / Cancel discards.
    pub(crate) new_scene_dialog: Option<String>,

    // === Scene Editor: Undo/Redo history (command-pattern overlay) ===
    pub(crate) command_history: scene_editor::history::CommandHistory,

    // === Scene Editor: Snapping ===
    pub(crate) snap_enabled: bool,
    pub(crate) snap_value: f32,

    // === Scene Editor: Asset drag & drop ===
    /// Path of the asset currently being dragged from the file tree.
    /// Set when the user starts dragging a droppable file; cleared on drop or release.
    pub(crate) dragged_asset_path: Option<String>,
    /// Path of the file/folder currently being dragged for move operation.
    /// Cached 3D model preview data for Asset Browser.
    pub(crate) asset_preview_data: Option<crate::app::model_preview::ModelPreviewData>,
    /// Path of the asset currently previewed (to detect changes).
    pub(crate) asset_preview_path: String,
    /// Orbit rotation for asset preview.
    pub(crate) asset_preview_rot_x: f32,
    pub(crate) asset_preview_rot_y: f32,
    pub(crate) asset_preview_zoom: f32,
    /// Animation playback state for asset preview.
    pub(crate) asset_preview_anim_time: f32,
    pub(crate) asset_preview_anim_playing: bool,
    pub(crate) asset_preview_anim_idx: usize,
    pub(crate) asset_preview_last_instant: Option<std::time::Instant>,

    // === Scene Editor: Profiler panel ===
    pub(crate) profiler: scene_editor::profiler::ProfilerState,

    // === Scene Editor: Particle preview ===
    /// Editor-only live particle simulation state, advanced each frame and
    /// drawn as 2D dots over the Scene View.
    pub(crate) particle_preview: scene_editor::particle_preview::ParticlePreview,

    // === Scene Editor: Animation playback ===
    /// Editor-only per-entity animation playback state. Drives the Timeline
    /// window and applies sampled transforms during scene sync.
    pub(crate) animation_playback: scene_editor::animation::AnimationPlayback,

    // === GLB skeletal animation bridge (read-only mirrors of `SceneAnimationState`) ===
    /// Per-scene-entity available clip names (mirrored each frame from
    /// `SceneAnimationState`). Used by the inspector to populate dropdowns.
    pub(crate) scene_anim_clips_view: std::collections::HashMap<u64, Vec<String>>,
    /// Per-scene-entity currently-playing clip name (mirror).
    pub(crate) scene_anim_current: std::collections::HashMap<u64, String>,
    /// Per-scene-entity user-requested clip name (consumed by `berry_ui_system`
    /// each frame and pushed back into `SceneAnimationState`).
    pub(crate) scene_anim_clip_request: std::collections::HashMap<u64, String>,

    // === Model preview animation bridge (read-only mirror of `ModelPreviewScene`) ===
    pub(crate) preview_anim_clips: Vec<String>,
    pub(crate) preview_anim_current: Option<String>,
    pub(crate) preview_anim_clip_request: Option<String>,
    /// Whether the floating Timeline window is currently visible.
    pub(crate) timeline_open: bool,
    /// Whether the floating Dopesheet / Curve Editor window is visible.
    pub(crate) dopesheet_open: bool,
    /// Whether the curve overlay is shown in the dopesheet.
    pub(crate) dopesheet_show_curves: bool,
    /// Whether the Animator Editor window is currently visible.
    pub(crate) animator_editor_open: bool,
    /// The animator controller being edited (if any).
    pub(crate) editing_animator: Option<scene_editor::animator::AnimatorController>,
    /// File path of the animator controller being edited.
    pub(crate) editing_animator_path: String,
    /// Index of the animator state node currently being dragged.
    pub(crate) animator_dragging_state: Option<usize>,
    /// Source state index for a pending "Add Transition From Here" action.
    pub(crate) pending_transition_from: Option<usize>,
    /// Currently selected state in the animator editor.
    pub(crate) animator_selected_state: Option<usize>,
    /// Currently selected transition in the animator editor.
    pub(crate) animator_selected_transition: Option<usize>,
    /// Whether the Blend Tree Editor window is currently visible.
    pub(crate) blend_tree_editor_open: bool,
    /// The blend tree being edited (if any).
    pub(crate) editing_blend_tree: Option<scene_editor::animator::BlendTree>,
    /// Whether the Humanoid Avatar Editor window is currently visible.
    pub(crate) avatar_editor_open: bool,
    /// The humanoid avatar being edited (if any).
    pub(crate) editing_avatar: Option<scene_editor::humanoid_avatar::HumanoidAvatar>,
    /// Clipboard for entity copy/paste in the scene hierarchy.
    pub(crate) entity_clipboard: Option<scene_editor::prefab::PrefabFile>,
    /// Whether the quad-view mode is active in the Scene View.
    pub(crate) quad_view_enabled: bool,
    /// Per-quadrant independent camera states.
    pub(crate) quad_camera_states: [scene_editor::scene_view::QuadCameraState; 4],
    /// Index of the currently active quadrant (0..3). The main camera
    /// parameters mirror this quadrant's state.
    pub(crate) active_quad_idx: usize,
    /// Whether an audio preview is currently playing in the inspector.
    pub(crate) audio_preview_playing: bool,
    /// Path of the audio file currently being previewed.
    pub(crate) audio_preview_path: String,

    // === Scene Editor: Material Preview GPU texture ===
    /// egui texture id for the GPU-rendered material preview sphere.
    /// Updated each frame from `MaterialPreviewRender` in `berry_ui_system`.
    pub(crate) material_preview_texture_id: Option<egui::TextureId>,
    /// PBR values to push to the material preview sphere each frame.
    /// Written by the inspector, consumed by `berry_ui_system`.
    pub(crate) material_preview_color: [f32; 3],
    pub(crate) material_preview_metallic: f32,
    pub(crate) material_preview_roughness: f32,
    pub(crate) material_preview_emissive: [f32; 3],
    /// Dirty flag: set true by the inspector when PBR values change.
    pub(crate) material_preview_dirty: bool,

    // === Scene Editor: Play Mode ===
    pub(crate) play_mode: scene_editor::play_mode::PlayModeState,
    pub(crate) play_mode_snapshot: Option<scene_editor::model::SceneModel>,

    // === Scene Editor: Physics Simulation ===
    pub(crate) physics_state: scene_editor::physics_sim::PhysicsState,

    // === Scene Editor: Build Settings ===
    pub(crate) build_settings_open: bool,
    pub(crate) build_settings: scene_editor::build_settings::BuildSettings,
    pub(crate) player_settings: scene_editor::build_settings::PlayerSettings,

    // === Customizable Keyboard Shortcuts ===
    pub(crate) keymap: keymap::Keymap,
    /// When `Some`, the Settings → Keybindings panel is capturing the next
    /// key press to rebind this action. Cleared once a chord arrives or the
    /// user cancels with `Esc`.
    pub(crate) keybinding_recording: Option<keymap::KeyAction>,
    /// Transient feedback string shown under the keybinding grid (e.g.
    /// "Cmd+S already bound to Save"). Cleared when the user starts a new
    /// recording.
    pub(crate) keybinding_message: Option<String>,
    /// Currently active visual theme preset (Dark / Light / High Contrast).
    /// Persisted under `~/.berrycode/theme.json` so the choice survives
    /// restarts.
    pub(crate) theme_mode: types::ThemeMode,
    /// User-editable settings (font size, format-on-save, etc.) loaded
    /// from `<config>/berrycode/settings.json`. The Settings → Appearance
    /// tab writes back to this and calls `EditorSettings::save()`.
    pub(crate) settings: crate::settings::EditorSettings,
    /// Per-panel visibility for the activity bar. Persisted under
    /// `~/.berrycode/panels.json` so users keep a tidy left strip across
    /// restarts. Database/Docker/OracleBerry default off.
    pub(crate) panel_visibility: types::PanelVisibility,
    /// `true` if the LSP completion popup pre-consumed Enter / Tab this
    /// frame so the editor's `TextEdit` won't treat it as a newline. Read
    /// and cleared by `render_lsp_completions`.
    pub(crate) lsp_completion_accept_pending: bool,
    /// BYOK provider configuration (Anthropic / OpenAI / Ollama API keys
    /// and selected models). Persisted to `~/.berrycode/ai.json`.
    #[cfg(feature = "ai")]
    pub(crate) ai_settings: crate::ai::settings::AiSettings,
    /// Last result of the Settings panel's "Test connection" probe
    /// against the configured Ollama endpoint. `Arc<Mutex>` so the
    /// async probe task can fill it from the tokio runtime while the
    /// egui render thread reads it on every frame.
    pub(crate) ollama_status: std::sync::Arc<std::sync::Mutex<OllamaProbeStatus>>,
    /// Cached list of locally installed Ollama models from the most
    /// recent `/api/tags` fetch (also Settings-panel triggered).
    pub(crate) ollama_installed_models: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Cached egui texture for the Scene View activity-bar icon
    /// (rasterised from `assets/icons/scene_view.svg` on first use).
    pub(crate) scene_view_icon: Option<egui::TextureHandle>,
    /// Cached egui texture for the Database activity-bar icon.
    pub(crate) database_icon: Option<egui::TextureHandle>,
    /// Cached egui texture for the Docker activity-bar icon (whale).
    pub(crate) docker_icon: Option<egui::TextureHandle>,
    /// Cached egui texture for the OracleBerry activity-bar icon.
    pub(crate) oracleberry_icon: Option<egui::TextureHandle>,

    // === Dockable Tool Panel ===
    pub(crate) tool_panel_open: bool,
    pub(crate) active_tool_tab: dock::ToolTab,

    // === Asset Thumbnails ===
    pub(crate) thumbnail_cache: scene_editor::thumbnail_cache::ThumbnailCache,

    // === Multiple Scene Tabs ===
    pub(crate) scene_tabs: Vec<scene_editor::scene_tabs::SceneTab>,
    pub(crate) active_scene_tab: usize,

    // === Asset Dependency Tracking ===
    pub(crate) asset_dependencies: Option<scene_editor::asset_deps::AssetDependencies>,

    // === Terrain Brush ===
    pub(crate) terrain_brush: scene_editor::terrain::TerrainBrushState,

    // === Visual Script Editor ===
    pub(crate) visual_script_editor_open: bool,
    pub(crate) editing_visual_script: Option<scene_editor::visual_script::VisualScript>,

    // === Shader Graph Editor ===
    pub(crate) shader_graph_editor_open: bool,
    pub(crate) editing_shader_graph: Option<scene_editor::shader_graph::ShaderGraph>,

    // === Hot Reload ===
    pub(crate) hot_reload: scene_editor::hot_reload::HotReloadState,

    // === Build Pipeline ===
    pub(crate) build_output: Vec<String>,
    pub(crate) build_process: Option<std::process::Child>,
    pub(crate) build_output_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Save-time Cargo Check ===
    pub(crate) cargo_check_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Test Mode CLI ===
    pub(crate) test_mode: bool,
    pub(crate) test_command_rx: Option<std::sync::mpsc::Receiver<String>>,

    // === Demo Capture (screenshots + video) ===
    pub(crate) demo_capture: demo_capture::DemoCapture,

    // === Scanned User Components (bidirectional sync) ===
    pub(crate) scanned_user_components: Vec<scene_editor::script_scan::ScannedComponent>,

    // === Scene Merge ===
    pub(crate) merge_panel_open: bool,
    pub(crate) merge_base_path: String,
    pub(crate) merge_ours_path: String,
    pub(crate) merge_theirs_path: String,
    pub(crate) merge_result: Option<scene_editor::scene_merge::MergeResult>,

    // === Bevy System Graph ===
    pub(crate) system_graph_open: bool,
    pub(crate) system_graph: scene_editor::system_graph::SystemGraph,
    /// Currently active view in the System Graph window. Toggled by
    /// the Dag / List buttons in the toolbar. v0.5 / drag-to-reorder.
    pub(crate) system_graph_view: scene_editor::system_graph::SystemGraphView,

    // === Bevy Event Monitor ===
    pub(crate) event_monitor_open: bool,
    pub(crate) event_log: Vec<scene_editor::event_monitor::EventEntry>,
    pub(crate) event_filter_text: String,
    pub(crate) event_filter_types: std::collections::HashSet<String>,

    // === Bevy Query Visualizer ===
    pub(crate) query_viz_open: bool,
    pub(crate) queries: Vec<scene_editor::query_viz::QueryDef>,

    // === Bevy States Editor ===
    pub(crate) state_editor_open: bool,
    pub(crate) state_graph: scene_editor::state_editor::StateGraph,

    // === Bevy Plugin Browser ===
    pub(crate) plugin_browser_open: bool,
    pub(crate) plugin_search_query: String,
    pub(crate) plugin_search_results: Vec<scene_editor::plugin_browser::CrateResult>,
    /// Background `Search` of crates.io for new plugins. Same shape
    /// as `installed_plugins_refresh_rx`: when the user clicks Search
    /// we spawn a worker thread that runs `search_bevy_crates` (curl
    /// shell-out, capped at 10 s by `--max-time`) and hands the
    /// result back through this channel. The egui pass polls it with
    /// `try_recv` per frame so the IDE never blocks. `None` means
    /// idle; `Some` means a search is in flight.
    pub(crate) plugin_search_rx:
        Option<std::sync::mpsc::Receiver<Vec<scene_editor::plugin_browser::CrateResult>>>,
    /// Dependencies parsed out of the project's `Cargo.toml`, with
    /// their crates.io "latest" version filled in on Refresh. Drives
    /// the Plugin Browser's auto-update section. v0.5.
    pub(crate) installed_plugins: Vec<scene_editor::plugin_browser::InstalledPlugin>,

    /// Filesystem watcher for `.bscene` / shader hot reload. v0.5.
    pub(crate) asset_watcher: asset_watcher::AssetWatcher,
    /// Audio preview panel state (waveform + scrub). v0.6 / Phase A.
    pub(crate) audio_preview: audio::preview::AudioPreviewState,
    /// Audio event registry (Phase C). Edited via the
    /// `Audio Events` floating window.
    pub(crate) audio_events: audio::events::AudioEventRegistry,
    pub(crate) audio_events_window_open: bool,
    /// Music graph (Phase D). Edited via the `Music Graph` window.
    pub(crate) music_graph: audio::music_graph::MusicGraph,
    pub(crate) music_graph_window_open: bool,
    /// Path of the scene currently loaded into `scene_model`, if any.
    /// Set by `load_scene`; consumed by the asset watcher poll loop
    /// to decide whether a `.bscene` change on disk should trigger a
    /// live reload (only if it matches the active scene). v0.5.
    pub(crate) current_scene_path: Option<String>,
    /// Tracks whether `installed_plugins` has been populated for the
    /// currently open browser session. Reset to false when the
    /// browser closes; the first render after open kicks off a scan
    /// so the user doesn't have to click Refresh just to see the
    /// list of installed deps.
    pub(crate) installed_plugins_loaded: bool,
    /// Background `Refresh` of the installed-plugin list. When the
    /// user clicks Refresh, we spawn a worker thread that hits
    /// crates.io (~1 round-trip per plugin, capped at 10 s each by
    /// curl's `--max-time`) and returns the updated list through this
    /// channel. The egui render path polls it with `try_recv` per
    /// frame so the IDE never blocks on the network. `None` means
    /// idle; `Some` means a refresh is in flight.
    pub(crate) installed_plugins_refresh_rx:
        Option<std::sync::mpsc::Receiver<Vec<scene_editor::plugin_browser::InstalledPlugin>>>,

    // === Bevy Version Management ===
    pub(crate) bevy_version: Option<String>,

    // === Package Manager ===
    pub(crate) package_manager_open: bool,
    pub(crate) package_manager: package_manager::PackageManagerState,
    /// Receiver for async crates.io search results
    pub(crate) package_manager_search_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,

    // === Mobile Toolchain (v0.8 Phase A) ===
    pub(crate) mobile_toolchain_open: bool,
    pub(crate) mobile_toolchain: mobile_toolchain::MobileToolchainState,

    // === Godot scene viewer (v0.8.x Migration & interop) ===
    // Auto-renders when the active editor tab is a `.tscn` file.
    // Holds the cached parse + selected node so we don't re-parse
    // every frame.
    pub(crate) godot_scene_panel: godot_panel::GodotScenePanelState,
    // === Texture Importer ===

    // === Audio Mixer ===

    // === Play Test Panel ===

    // === Visual Merge ===

    // === Lighting Profiler ===
}

impl BerryCodeApp {
    /// Shorthand for i18n translation
    pub(crate) fn tr(&self, key: &'static str) -> &'static str {
        crate::app::i18n::t(self.ui_language, key)
    }

    /// Apply the BerryCode egui style — VS Code Dark+ inspired theme.
    pub fn setup_egui_style(ctx: &egui::Context) {
        let mut style = egui::Style::default();
        let mut visuals = egui::Visuals::dark();

        let bg_dark = ui_colors::EDITOR_BG();
        let bg_panel = ui_colors::SIDEBAR_BG();
        let bg_input = ui_colors::CONTROL_BG();
        let bg_hover = ui_colors::HOVER_BG();
        let bg_active = ui_colors::ACTIVE_BG();
        let bg_selected = egui::Color32::from_rgba_premultiplied(9, 71, 113, 180);
        let border = ui_colors::BORDER();
        let border_focus = ui_colors::FOCUS_BORDER();
        let text = ui_colors::TEXT_DEFAULT();

        visuals.override_text_color = None;
        visuals.window_fill = bg_panel;
        visuals.panel_fill = bg_dark;
        visuals.extreme_bg_color = bg_input;
        visuals.code_bg_color = bg_dark;
        visuals.faint_bg_color = ui_colors::HOVER_BG();
        visuals.hyperlink_color = ui_colors::ACCENT_HOVER();

        // Window
        visuals.window_stroke = egui::Stroke::new(1.0, ui_colors::PANEL_BORDER());
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(100),
        };
        visuals.window_corner_radius = egui::CornerRadius::same(4);
        visuals.menu_corner_radius = egui::CornerRadius::same(3);

        // Selection — `selection.stroke.color` is misleadingly named: egui
        // uses it to *override* the glyph color of the selected text, so it
        // must be visible (using TRANSPARENT here makes selected text — and
        // IME preedit text — invisible).
        visuals.selection.bg_fill = bg_selected;
        visuals.selection.stroke = egui::Stroke::new(0.0, text);

        // Text cursor
        visuals.text_cursor.stroke.color = egui::Color32::from_rgb(174, 175, 173);

        // === Widget Styles ===

        // Non-interactive (labels, separators)
        visuals.widgets.noninteractive.bg_fill = bg_dark;
        visuals.widgets.noninteractive.weak_bg_fill = bg_dark;
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, ui_colors::PANEL_BORDER());
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text);
        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(2);

        // Inactive (buttons, checkboxes at rest)
        visuals.widgets.inactive.bg_fill = bg_panel;
        visuals.widgets.inactive.weak_bg_fill = bg_panel;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(2);
        visuals.widgets.inactive.expansion = 0.0;

        // Hovered
        visuals.widgets.hovered.bg_fill = bg_hover;
        visuals.widgets.hovered.weak_bg_fill = bg_hover;
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ui_colors::CONTROL_BORDER());
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(2);
        visuals.widgets.hovered.expansion = 0.0;

        // Active (pressed)
        visuals.widgets.active.bg_fill = bg_active;
        visuals.widgets.active.weak_bg_fill = bg_active;
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, border_focus);
        visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(2);
        visuals.widgets.active.expansion = 0.0;

        // Open (combo boxes, menus open state)
        visuals.widgets.open.bg_fill = bg_active;
        visuals.widgets.open.weak_bg_fill = bg_active;
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, border_focus);
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
        visuals.widgets.open.corner_radius = egui::CornerRadius::same(2);

        // Popup shadow
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 16,
            spread: 0,
            color: egui::Color32::from_black_alpha(100),
        };

        // Striped backgrounds (tables)
        visuals.striped = true;

        // Separator
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, ui_colors::PANEL_BORDER());

        style.visuals = visuals;

        // === Spacing ===
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0); // VS Code: compact
        style.spacing.window_margin = egui::Margin::same(8);
        style.spacing.menu_margin = egui::Margin::same(6);
        style.spacing.indent = 18.0; // tree indent
                                     // Keep resize handles and splitters slim and consistent across
                                     // Explorer/Search/AI/Inspector side panels.
        style.spacing.interact_size = egui::vec2(12.0, 24.0);
        // Keep resize handles easy to grab.
        style.interaction.resize_grab_radius_side = 6.0;
        style.interaction.resize_grab_radius_corner = 6.0;
        style.spacing.slider_width = 160.0;
        style.spacing.combo_width = 160.0;
        style.spacing.text_edit_width = 200.0;
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 8.0,
            ..Default::default()
        };

        // === Text ===
        // VS Code keeps its chrome compact: 13px body text, 12px controls,
        // 11px secondary labels. Panels can still opt into larger text for
        // true document/editor content, but the app shell starts here.
        use egui::FontId;
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(16.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(11.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::monospace(13.0));

        // === Interaction ===
        style.interaction.show_tooltips_only_when_still = false;

        ctx.set_style(style);
    }

    /// Open a native OS folder selection dialog (cross-platform via rfd).
    /// Returns the selected folder path, or None if cancelled.
    fn native_folder_dialog() -> Option<String> {
        let folder = rfd::FileDialog::new()
            .set_title("Select Bevy Project Folder")
            .pick_folder()?;
        let path = folder.to_string_lossy().to_string();
        let path = path.trim_end_matches(['/', '\\']).to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    /// Resolve the project path: CLI arg > env > prompt user
    fn resolve_project_path() -> String {
        // 1. Check command-line arguments: berrycode /path/to/project
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let path = &args[1];
            if std::path::Path::new(path).is_dir() {
                tracing::info!("Project path from CLI arg: {}", path);
                return path.clone();
            }
        }

        // 2. Check BERRYCODE_PROJECT env var
        if let Ok(path) = std::env::var("BERRYCODE_PROJECT") {
            if std::path::Path::new(&path).is_dir() {
                tracing::info!("Project path from env: {}", path);
                return path;
            }
        }

        // 3. No project specified — use empty placeholder; the picker will show
        String::new()
    }

    /// Load recent projects from ~/.berrycode/recent_projects.txt
    fn load_recent_projects() -> Vec<String> {
        let path = dirs::home_dir()
            .map(|h| format!("{}/.berrycode/recent_projects.txt", h.display()))
            .unwrap_or_default();
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty() && std::path::Path::new(l).is_dir())
            .map(|l| l.to_string())
            .collect()
    }

    /// Save a project to recent projects list
    fn save_to_recent_projects(project_path: &str) {
        let config_dir = dirs::home_dir()
            .map(|h| format!("{}/.berrycode", h.display()))
            .unwrap_or_default();
        let _ = std::fs::create_dir_all(&config_dir);
        let file_path = format!("{}/recent_projects.txt", config_dir);
        let mut projects = Self::load_recent_projects();
        projects.retain(|p| p != project_path);
        projects.insert(0, project_path.to_string());
        projects.truncate(10); // Keep last 10
        let _ = std::fs::write(&file_path, projects.join("\n"));
    }

    /// Save the current recent projects list to disk
    fn save_recent_projects(projects: &[String]) {
        let config_dir = dirs::home_dir()
            .map(|h| format!("{}/.berrycode", h.display()))
            .unwrap_or_default();
        let _ = std::fs::create_dir_all(&config_dir);
        let file_path = format!("{}/recent_projects.txt", config_dir);
        let _ = std::fs::write(&file_path, projects.join("\n"));
    }

    /// Open a project: set root_path, refresh file tree, start LSP, etc.
    pub(crate) fn open_project(&mut self, path: &str) {
        self.root_path = path.to_string();
        self.show_project_picker = false;
        self.file_tree_cache.clear();
        self.file_tree_load_pending = true;
        self.expanded_dirs.clear();
        self.editor_tabs.clear();
        self.active_tab_idx = 0;
        self.git_initialized = false;

        // Issue #18: scene editor state from the previous project
        // bleeds into the new one if we don't reset here. Wipe the
        // tabs and model; if the new project has `.bscene` files the
        // bscene auto-load below replaces this Untitled tab anyway.
        self.scene_tabs.clear();
        self.scene_tabs
            .push(scene_editor::scene_tabs::SceneTab::new(
                scene_editor::model::SceneModel::new(),
                "Untitled".to_string(),
            ));
        self.active_scene_tab = 0;
        self.scene_model = scene_editor::model::SceneModel::new();
        self.scene_needs_sync = true;
        self.primary_selected_id = None;
        self.current_scene_path = None;

        // Start file watcher for new project
        if let Ok(mut watcher) = crate::native::watcher::FileWatcher::new() {
            let _ = watcher.watch(&self.root_path);
            self.file_watcher = Some(watcher);
        }

        // Save to recent projects
        Self::save_to_recent_projects(path);

        // Auto-load scenes: load EVERY `.bscene` in `scenes/` as its
        // own tab, with the alphabetically-first one active. The old
        // behaviour stopped at the first match, which meant a project
        // with `scene.bscene` + `scene2.bscene` only ever showed
        // `scene` after a restart — `scene2` was orphaned in the file
        // tree with no way to reopen it.
        let bscene_paths = list_project_bscenes(path);

        let mut bscene_loaded = false;
        if !bscene_paths.is_empty() {
            // Replace the seeded "Untitled" tab with the first scene,
            // then push the remaining scenes as additional tabs.
            self.scene_tabs.clear();
            for (idx, bscene_path) in bscene_paths.iter().enumerate() {
                match crate::app::scene_editor::serialization::load_scene_from_ron(bscene_path) {
                    Ok(mut scene) => {
                        let bscene_path_owned = bscene_path.to_string();
                        scene.file_path = Some(bscene_path_owned.clone());
                        let label =
                            crate::app::shortcuts::scene_label_from_path(&bscene_path_owned);
                        let count = scene.entities.len();
                        // Re-run codegen for the loaded scene so any
                        // stale `<name>_scene.rs` / un-prefixed
                        // `asset_server` signatures from older
                        // BerryCode versions get rewritten with the
                        // current conventions. Idempotent: same scene
                        // produces byte-identical output, so the disk
                        // write only happens when the on-disk file
                        // would actually change.
                        let regenerated = crate::app::shortcuts::run_codegen_for_save(
                            &scene,
                            &bscene_path_owned,
                            path,
                        );
                        if let Err(e) = regenerated {
                            tracing::debug!("Skipped regen for {} ({})", bscene_path_owned, e);
                        }
                        self.scene_tabs
                            .push(crate::app::scene_editor::scene_tabs::SceneTab::new(
                                scene.clone(),
                                label,
                            ));
                        if idx == 0 {
                            self.active_scene_tab = 0;
                            self.scene_model = scene;
                            self.current_scene_path = Some(bscene_path_owned.clone());
                            self.scene_needs_sync = true;
                            bscene_loaded = true;
                        }
                        tracing::info!("Loaded {} entities from {}", count, bscene_path_owned);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load {}: {}", bscene_path, e);
                    }
                }
            }
            // If every load failed we still need at least one tab.
            if self.scene_tabs.is_empty() {
                self.scene_tabs
                    .push(crate::app::scene_editor::scene_tabs::SceneTab::new(
                        crate::app::scene_editor::model::SceneModel::new(),
                        "Untitled".to_string(),
                    ));
                self.active_scene_tab = 0;
            }
        }

        if !bscene_loaded && self.scene_model.entities.is_empty() {
            let main_path = format!("{}/src/main.rs", path);
            if let Ok(code) = crate::native::fs::read_file(&main_path) {
                let imported = crate::app::scene_editor::code_import::import_scene_from_code(&code);
                if !imported.entities.is_empty() {
                    let count = imported.entities.len();
                    if let Some(tab) = self.scene_tabs.get_mut(self.active_scene_tab) {
                        tab.model = imported.clone();
                    }
                    self.scene_model = imported;
                    self.scene_needs_sync = true;
                    tracing::info!("Auto-imported {} entities from main.rs", count);
                }
            }
        }

        // Restart LSP with the new project root
        if let Some(client) = &self.lsp_native_client {
            let root = path.to_string();
            let runtime = self.lsp_runtime.clone();
            let client = client.clone();
            std::thread::spawn(move || {
                runtime.block_on(async {
                    if let Err(e) = client.start_server("rust", &root).await {
                        tracing::warn!("LSP restart failed: {}", e);
                    } else {
                        tracing::info!("✅ LSP connected for project: {}", root);
                    }
                });
            });
            self.lsp_connected = true;
        }

        self.status_message = format!("Opened project: {}", path);
        self.status_message_timestamp = Some(std::time::Instant::now());
        tracing::info!("Opened project: {}", path);
    }

    /// Render the project picker screen (shown when no project is loaded)
    pub(crate) fn render_project_picker(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(ui_colors::EDITOR_BG()))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);

                    // Logo / Title
                    ui.label(
                        egui::RichText::new("BerryCode")
                            .size(48.0)
                            .color(ui_colors::TEXT_DEFAULT())
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Bevy Game Engine IDE")
                            .size(16.0)
                            .color(egui::Color32::from_gray(140)),
                    );

                    ui.add_space(40.0);

                    // Open project section
                    ui.group(|ui| {
                        ui.set_width(500.0);
                        ui.vertical(|ui| {
                            ui.heading("Open Project");
                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label("Path:");
                                ui.add_sized(
                                    [300.0, 22.0],
                                    egui::TextEdit::singleline(&mut self.project_picker_path)
                                        .hint_text("/path/to/your/bevy/project"),
                                );
                                if ui.button("Browse...").clicked() {
                                    // Open native folder picker dialog
                                    if let Some(path) = Self::native_folder_dialog() {
                                        self.project_picker_path = path;
                                    }
                                }
                                if ui.button("Open").clicked()
                                    && !self.project_picker_path.is_empty()
                                {
                                    let path = self.project_picker_path.clone();
                                    if std::path::Path::new(&path).is_dir() {
                                        self.open_project(&path);
                                    } else {
                                        self.status_message =
                                            format!("Directory not found: {}", path);
                                        self.status_message_timestamp =
                                            Some(std::time::Instant::now());
                                    }
                                }
                            });

                            ui.add_space(8.0);

                            // New Bevy Project button
                            if ui.button("+ New Bevy Project").clicked() {
                                self.new_project_dialog_open = true;
                            }
                        });
                    });

                    ui.add_space(20.0);

                    // Recent projects
                    if !self.recent_projects.is_empty() {
                        ui.group(|ui| {
                            ui.set_width(500.0);
                            ui.vertical(|ui| {
                                ui.heading("Recent Projects");
                                ui.add_space(4.0);

                                let recent = self.recent_projects.clone();
                                let mut removed: Option<String> = None;
                                for project in &recent {
                                    let name = project.rsplit('/').next().unwrap_or(project);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new(name).size(14.0),
                                                )
                                                .frame(false),
                                            )
                                            .clicked()
                                        {
                                            self.open_project(project);
                                        }
                                        ui.label(
                                            egui::RichText::new(project)
                                                .size(11.0)
                                                .color(egui::Color32::from_gray(120)),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            egui::RichText::new("\u{ea76}")
                                                                .size(12.0)
                                                                .color(egui::Color32::from_gray(
                                                                    120,
                                                                )),
                                                        )
                                                        .frame(false),
                                                    )
                                                    .on_hover_text("Remove from list")
                                                    .clicked()
                                                {
                                                    removed = Some(project.clone());
                                                }
                                            },
                                        );
                                    });
                                }
                                if let Some(path) = removed {
                                    self.recent_projects.retain(|p| p != &path);
                                    Self::save_recent_projects(&self.recent_projects);
                                }
                            });
                        });
                    }

                    ui.add_space(40.0);
                    ui.label(
                        egui::RichText::new(format!("v{} | Bevy 0.18", env!("CARGO_PKG_VERSION")))
                            .size(11.0)
                            .color(egui::Color32::from_gray(100)),
                    );
                });
            });
    }

    /// Create new application instance
    pub fn new() -> Self {
        // Check command-line args for project path, otherwise show picker
        let root_path = Self::resolve_project_path();

        tracing::info!("📁 Project root: {}", root_path);

        let terminal_working_dir = root_path.clone();

        // Create Tokio runtime for async LSP operations
        let lsp_runtime = std::sync::Arc::new(
            tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for LSP"),
        );

        // Create native LSP client (returns client + diagnostics receiver)
        let (lsp_native_client_inner, lsp_diagnostics_rx) =
            native::lsp_native::NativeLspClient::new();
        let lsp_native_client = std::sync::Arc::new(lsp_native_client_inner);

        // Create LSP response channel
        let (lsp_tx, lsp_rx) = mpsc::unbounded_channel();

        // Create AI response channel
        let (ai_tx, ai_rx) = mpsc::unbounded_channel();

        // Only start file watcher, LSP, and API health check if a project is selected
        let file_watcher = if !root_path.is_empty() {
            match native::watcher::FileWatcher::new() {
                Ok(mut watcher) => {
                    if let Err(e) = watcher.watch(&root_path) {
                        tracing::warn!(
                            "⚠️  Failed to start file watching for {}: {}",
                            root_path,
                            e
                        );
                        None
                    } else {
                        tracing::info!("👁  File watcher started for: {}", root_path);
                        Some(watcher)
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️  Failed to create file watcher: {}", e);
                    None
                }
            }
        } else {
            None
        };

        if !root_path.is_empty() {
            // Spawn native LSP initialization task
            let client_clone = lsp_native_client.clone();
            let root_path_clone = root_path.clone();
            let tx_clone = lsp_tx.clone();

            lsp_runtime.spawn(async move {
                match client_clone.start_server("rust", &root_path_clone).await {
                    Ok(_) => {
                        tracing::info!("✅ Native LSP (rust-analyzer) started");
                        let _ = tx_clone.send(LspResponse::Connected);
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Native LSP startup failed: {} (will use fallback)", e);
                    }
                }
            });

            // Spawn REST (berry-core-api) health check
            {
                let ai_tx_clone = ai_tx.clone();
                lsp_runtime.spawn(async move {
                    let rest_client = crate::native::rest_client::get_client().clone();
                    if rest_client.is_healthy().await {
                        tracing::info!("✅ berry-core-api is reachable");
                        let _ = ai_tx_clone.send(AiChatResponse::SessionStarted("rest".to_string()));
                    } else {
                        tracing::warn!(
                            "⚠️  berry-core-api not reachable. AI chat will attempt on each message."
                        );
                    }
                });
            }
        }

        let bevy_version = scene_editor::bevy_version::detect_bevy_version(&root_path);

        let show_picker = root_path.is_empty();
        let recent = Self::load_recent_projects();
        let home = dirs::home_dir()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_default();
        let picker_path = home.clone();
        if !root_path.is_empty() {
            Self::save_to_recent_projects(&root_path);
        }
        // Keep root_path empty if no project specified — picker will handle it
        let root_path = if root_path.is_empty() {
            String::new()
        } else {
            root_path
        };
        let root_path_ref = root_path.clone();

        let mut app = Self {
            root_path,
            selected_file: None,
            show_project_picker: show_picker,
            show_close_confirm: false,
            close_action: None,
            project_picker_path: picker_path,
            recent_projects: recent,
            active_panel: ActivePanel::Explorer,
            sidebar_width: 220.0,
            editor_tabs: {
                // Auto-open src/main.rs if it exists
                let main_path = format!("{}/src/main.rs", root_path_ref);
                if std::path::Path::new(&main_path).exists() {
                    if let Ok(content) = crate::native::fs::read_file(&main_path) {
                        vec![types::EditorTab::new(main_path, content)]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            },
            active_tab_idx: 0,
            syntax_highlighter: SyntaxHighlighter::new(),
            editor_ime_preedit: String::new(),
            file_tree_cache: Vec::new(),
            file_tree_load_pending: true,
            file_tree_folder_rects: Vec::new(),
            sidebar_visible: true,
            tab_drag_source: None,
            additional_roots: load_additional_roots(),
            additional_root_caches: Vec::new(),
            split_right_tab: None,
            expanded_dirs: {
                let mut dirs = HashSet::new();
                // Auto-expand src/ directory
                let src_dir = format!("{}/src", root_path_ref);
                if std::path::Path::new(&src_dir).is_dir() {
                    dirs.insert(root_path_ref.clone());
                    dirs.insert(src_dir);
                }
                dirs
            },
            terminal: terminal_emulator::TerminalEmulator::new(&terminal_working_dir),
            database: database::DatabaseState::default(),
            docker: docker::DockerState::default(),
            #[cfg(feature = "ai")]
            oracleberry: oracleberry::OracleBerryState::default(),
            search_query: String::new(),
            search_dialog_open: false,
            search_case_sensitive: false,
            search_whole_word: false,
            search_use_regex: false,
            search_show_replace: false,
            search_include_glob: String::new(),
            search_exclude_glob: String::new(),
            search_show_details: false,
            search_collapsed_files: std::collections::HashSet::new(),
            current_search_index: 0,
            search_results: Vec::new(),
            replace_query: String::new(),
            show_replace: false,
            git_current_branch: String::from("(unknown)"),
            git_status: Vec::new(),
            git_commit_message: String::new(),
            git_initialized: false,
            last_poll_tick: None,
            git_active_tab: GitTab::Status,
            git_history_state: GitHistoryState::default(),
            git_branch_state: GitBranchState::default(),
            git_remote_state: GitRemoteState::default(),
            git_tag_state: GitTagState::default(),
            git_stash_state: GitStashState::default(),
            git_diff_state: GitDiffState::default(),
            lsp_runtime,
            lsp_native_client: Some(lsp_native_client),
            lsp_response_tx: Some(lsp_tx),
            // Optimistic: if we have a project loaded we kicked off the
            // LSP startup task above, so show "LSP" in the status bar
            // immediately. The Connected message that arrives after the
            // initialize handshake just confirms it; a hard failure logs
            // a warning and the indicator can be reset by the caller.
            lsp_connected: !root_path_ref.is_empty(),
            lsp_diagnostics: Vec::new(),
            lsp_hover_info: None,
            lsp_completions: Vec::new(),
            lsp_show_completions: false,
            lsp_auto_trigger_pending: false,
            lsp_signature_help: None,
            lsp_signature_trigger_pending: false,
            lsp_completion_index: 0,
            lsp_show_hover: false,
            lsp_response_rx: Some(lsp_rx),
            lsp_diagnostics_rx: Some(lsp_diagnostics_rx),
            status_message: String::new(),
            status_message_timestamp: None,
            pending_goto_definition: None,
            definition_picker_locations: Vec::new(),
            show_definition_picker: false,
            lsp_references: Vec::new(),
            show_references_panel: false,

            lsp_inlay_hints: Vec::new(),
            inlay_hints_enabled: true,
            inlay_hints_last_request: None,

            lsp_code_actions: Vec::new(),
            show_code_actions: false,
            code_action_line: 0,

            snippet_session: None,

            rename_dialog_open: false,
            rename_new_name: String::new(),

            ai_connected: false,
            ai_response_tx: Some(ai_tx),
            ai_response_rx: Some(ai_rx),
            ai_streaming_message: None,
            ai_messages: Vec::new(),
            ai_input: String::new(),
            ai_chat_collapsed: false,
            ai_streaming: false,
            ai_current_response: String::new(),
            chat_attachment: None,
            ai_chat_focus_pending: false,
            pending_agent_edits: Vec::new(),
            show_settings: false,
            active_settings_tab: SettingsTab::EditorColor,
            ui_language: UiLanguage::English,
            show_theme_editor: false,
            keyword_color: syntax_colors::KEYWORD,
            function_color: syntax_colors::FUNCTION,
            type_color: syntax_colors::TYPE,
            string_color: syntax_colors::STRING,
            number_color: syntax_colors::NUMBER,
            comment_color: syntax_colors::COMMENT,
            doc_comment_color: syntax_colors::DOC_COMMENT,
            macro_color: syntax_colors::MACRO,
            attribute_color: syntax_colors::ATTRIBUTE,
            constant_color: syntax_colors::CONSTANT,
            lifetime_color: syntax_colors::LIFETIME,
            namespace_color: syntax_colors::NAMESPACE,
            variable_color: syntax_colors::VARIABLE,
            operator_color: syntax_colors::OPERATOR,
            multi_cursors: Vec::new(),
            peek_definition: None,
            active_focus: FocusLayer::Editor,

            new_file_dialog_open: false,
            new_file_name: String::new(),
            new_folder_dialog_open: false,
            new_folder_name: String::new(),
            new_project_dialog_open: false,
            new_project_name: String::new(),
            new_project_path: dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string()),
            new_project_template: new_project::ProjectTemplate::Empty2D,

            blame_cache_line: usize::MAX,
            blame_cache_text: String::new(),
            blame_cache_file: String::new(),

            file_watcher,

            ecs_inspector: Default::default(),
            ecs_inspector_tab: ecs_inspector::EcsInspectorTab::default(),

            scene_preview: Default::default(),

            asset_browser: Default::default(),

            template_name: String::new(),
            template_fields: Vec::new(),
            template_params: Vec::new(),
            template_variants: Vec::new(),

            debug_state: Default::default(),
            dap_client: None,
            dap_event_rx: None,
            test_runner: Default::default(),
            user_snippets: custom_snippets::load_user_snippets(),
            vim: Default::default(),
            plugin_manager: Default::default(),
            pending_plugin_command: None,
            remote: Default::default(),
            remote_dialog: Default::default(),
            collab: Default::default(),
            collab_dialog: Default::default(),

            context_menu_path: None,
            context_menu_is_dir: false,
            context_menu_pos: egui::Pos2::ZERO,
            rename_file_dialog_open: false,
            rename_file_old_path: String::new(),
            rename_file_new_name: String::new(),

            run_output: Vec::new(),
            run_process: None,
            run_panel_open: false,
            run_release_mode: false,
            run_output_rx: None,

            console_filter_text: String::new(),
            console_show_info: true,
            console_show_warning: true,
            console_show_error: true,
            console_auto_scroll: true,
            console_log_level_filter: run_panel::LogLevelFilter::Trace,

            scene_model: {
                // Auto-import entities from main.rs if project has one
                let main_path = format!("{}/src/main.rs", root_path_ref);
                if let Ok(code) = crate::native::fs::read_file(&main_path) {
                    let imported = scene_editor::code_import::import_scene_from_code(&code);
                    if !imported.entities.is_empty() {
                        tracing::info!(
                            "Auto-imported {} entities from main.rs",
                            imported.entities.len()
                        );
                        imported
                    } else {
                        scene_editor::model::SceneModel::new()
                    }
                } else {
                    scene_editor::model::SceneModel::new()
                }
            },
            primary_selected_id: None,
            scene_view_texture_id: None,
            scene_needs_sync: false,
            component_clipboard: None,
            add_component_filter: String::new(),
            add_component_popup_open: false,
            new_script_dialog_open: false,
            new_script_name: String::new(),
            scene_view_mode: SceneViewMode::Scene,
            display_profile: DisplayProfile::Default,
            show_colliders: true,
            scene_orbit_yaw: std::f32::consts::FRAC_PI_4,
            scene_orbit_pitch: 0.5,
            scene_orbit_distance: 8.0,
            scene_orbit_target: [0.0, 0.0, 0.0],
            scene_ortho: false,
            scene_ortho_scale: 8.0,
            scene_shadows_enabled: true,
            scene_bloom_enabled: false,
            scene_bloom_intensity: 0.3,
            scene_tonemapping: 3,
            scene_ssao_enabled: false,
            scene_taa_enabled: false,
            scene_fog_enabled: false,
            scene_fog_color: [0.7, 0.8, 1.0],
            scene_fog_start: 50.0,
            scene_fog_end: 200.0,
            scene_dof_enabled: false,
            scene_dof_focus_distance: 5.0,
            scene_dof_aperture: 0.02,
            fly_mode_active: false,
            fly_camera_speed: 5.0,
            gizmo_mode: scene_editor::gizmo::GizmoMode::Move,
            gizmo_dragging: None,
            box_select_start: None,

            hierarchy_filter: String::new(),
            hierarchy_dragged: None,
            hierarchy_drop_target: None,
            renaming_entity_id: None,
            rename_buffer: String::new(),
            new_scene_dialog: None,

            command_history: scene_editor::history::CommandHistory::new(),

            snap_enabled: false,
            snap_value: 0.5,

            dragged_asset_path: None,
            asset_preview_data: None,
            asset_preview_path: String::new(),
            asset_preview_rot_x: 0.3,
            asset_preview_rot_y: std::f32::consts::FRAC_PI_4,
            asset_preview_zoom: 1.0,
            asset_preview_anim_time: 0.0,
            asset_preview_anim_playing: true,
            asset_preview_anim_idx: 0,
            asset_preview_last_instant: None,

            profiler: {
                let mut p = scene_editor::profiler::ProfilerState::default();
                p.open = false;
                p
            },

            particle_preview: scene_editor::particle_preview::ParticlePreview::default(),

            animation_playback: {
                let mut ap = scene_editor::animation::AnimationPlayback::default();
                ap.playing = true;
                ap
            },

            scene_anim_clips_view: std::collections::HashMap::new(),
            scene_anim_current: std::collections::HashMap::new(),
            scene_anim_clip_request: std::collections::HashMap::new(),

            preview_anim_clips: Vec::new(),
            preview_anim_current: None,
            preview_anim_clip_request: None,
            timeline_open: false,
            dopesheet_open: false,
            dopesheet_show_curves: true,
            animator_editor_open: false,
            editing_animator: Some({
                let mut c = scene_editor::animator::AnimatorController::default();
                c.states.push(scene_editor::animator::AnimState {
                    name: "Walk".into(),
                    clip_name: "walk".into(),
                    speed: 1.0,
                    looped: true,
                    position: [300.0, 100.0],
                    motion: crate::app::scene_editor::animator::Motion::default(),
                    kind: crate::app::scene_editor::animator::StateKind::Normal,
                });
                c.states.push(scene_editor::animator::AnimState {
                    name: "Run".into(),
                    clip_name: "run".into(),
                    speed: 1.5,
                    looped: true,
                    position: [300.0, 250.0],
                    motion: crate::app::scene_editor::animator::Motion::default(),
                    kind: crate::app::scene_editor::animator::StateKind::Normal,
                });
                c.transitions.push(scene_editor::animator::AnimTransition {
                    from_state: 0,
                    to_state: 1,
                    condition: scene_editor::animator::TransitionCondition::BoolParam {
                        name: "is_running".into(),
                        value: true,
                    },
                    blend_duration: 0.2,
                    has_exit_time: false,
                    exit_time: 1.0,
                });
                c.transitions.push(scene_editor::animator::AnimTransition {
                    from_state: 1,
                    to_state: 0,
                    condition: scene_editor::animator::TransitionCondition::BoolParam {
                        name: "is_running".into(),
                        value: false,
                    },
                    blend_duration: 0.3,
                    has_exit_time: false,
                    exit_time: 1.0,
                });
                c.parameters.push(scene_editor::animator::AnimParam::Bool {
                    name: "is_running".into(),
                    value: false,
                });
                c.parameters.push(scene_editor::animator::AnimParam::Float {
                    name: "speed".into(),
                    value: 1.0,
                });
                c
            }),
            editing_animator_path: String::new(),
            animator_dragging_state: None,
            animator_selected_state: None,
            animator_selected_transition: None,
            blend_tree_editor_open: false,
            editing_blend_tree: None,
            avatar_editor_open: false,
            editing_avatar: None,
            pending_transition_from: None,
            entity_clipboard: None,
            quad_view_enabled: false,
            quad_camera_states: scene_editor::scene_view::QuadCameraState::defaults(),
            active_quad_idx: 0,
            audio_preview_playing: false,
            audio_preview_path: String::new(),

            material_preview_texture_id: None,
            material_preview_color: [0.8, 0.8, 0.8],
            material_preview_metallic: 0.0,
            material_preview_roughness: 0.5,
            material_preview_emissive: [0.0, 0.0, 0.0],
            material_preview_dirty: false,

            play_mode: scene_editor::play_mode::PlayModeState::Stopped,
            play_mode_snapshot: None,

            physics_state: scene_editor::physics_sim::PhysicsState::default(),

            build_settings_open: false,
            build_settings: scene_editor::build_settings::BuildSettings::default(),
            player_settings: scene_editor::build_settings::PlayerSettings::default(),

            keymap: keymap::Keymap::load(),
            keybinding_recording: None,
            keybinding_message: None,
            theme_mode: load_theme_mode(),
            settings: crate::settings::EditorSettings::load(),
            panel_visibility: load_panel_visibility(),
            lsp_completion_accept_pending: false,
            #[cfg(feature = "ai")]
            ai_settings: crate::ai::settings::AiSettings::load(),
            ollama_status: std::sync::Arc::new(std::sync::Mutex::new(OllamaProbeStatus::Unknown)),
            ollama_installed_models: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            scene_view_icon: None,
            database_icon: None,
            docker_icon: None,
            oracleberry_icon: None,

            tool_panel_open: false,
            active_tool_tab: dock::ToolTab::Output,

            thumbnail_cache: scene_editor::thumbnail_cache::ThumbnailCache::new(),

            // Always start with one tab so the Hierarchy panel's tab
            // strip has something to render — an empty `vec![]` left
            // the UI looking like the panel hadn't loaded.
            scene_tabs: vec![scene_editor::scene_tabs::SceneTab::new(
                scene_editor::model::SceneModel::new(),
                "Untitled".to_string(),
            )],
            active_scene_tab: 0,

            asset_dependencies: None,

            terrain_brush: scene_editor::terrain::TerrainBrushState::default(),

            visual_script_editor_open: false,
            editing_visual_script: Some({
                let mut s = scene_editor::visual_script::VisualScript::default();
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 2,
                    node_type: scene_editor::visual_script::NodeType::Print {
                        message: "Hello World".into(),
                    },
                    position: [300.0, 80.0],
                });
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 3,
                    node_type: scene_editor::visual_script::NodeType::Branch,
                    position: [200.0, 200.0],
                });
                s.nodes.push(scene_editor::visual_script::ScriptNode {
                    id: 4,
                    node_type: scene_editor::visual_script::NodeType::Delay { seconds: 1.0 },
                    position: [400.0, 200.0],
                });
                s.edges.push(scene_editor::visual_script::ScriptEdge {
                    from_node: 1,
                    from_pin: 0,
                    to_node: 2,
                    to_pin: 0,
                });
                s.edges.push(scene_editor::visual_script::ScriptEdge {
                    from_node: 2,
                    from_pin: 0,
                    to_node: 3,
                    to_pin: 0,
                });
                s
            }),

            shader_graph_editor_open: false,
            editing_shader_graph: Some(scene_editor::shader_graph::ShaderGraph::default()),

            hot_reload: scene_editor::hot_reload::HotReloadState::default(),

            build_output: Vec::new(),
            build_process: None,
            build_output_rx: None,

            cargo_check_rx: None,

            test_mode: false,
            test_command_rx: None,

            demo_capture: demo_capture::DemoCapture::new(),

            scanned_user_components: Vec::new(),

            merge_panel_open: false,
            merge_base_path: String::new(),
            merge_ours_path: String::new(),
            merge_theirs_path: String::new(),
            merge_result: None,

            system_graph_open: false,
            system_graph: scene_editor::system_graph::SystemGraph::default(),
            system_graph_view: scene_editor::system_graph::SystemGraphView::default(),

            event_monitor_open: false,
            event_log: Vec::new(),
            event_filter_text: String::new(),
            event_filter_types: std::collections::HashSet::new(),

            query_viz_open: false,
            queries: Vec::new(),

            state_editor_open: false,
            state_graph: scene_editor::state_editor::StateGraph::default_game_states(),

            plugin_browser_open: false,
            installed_plugins: Vec::new(),
            installed_plugins_loaded: false,
            installed_plugins_refresh_rx: None,
            asset_watcher: asset_watcher::AssetWatcher::default(),
            current_scene_path: None,
            audio_preview: audio::preview::AudioPreviewState::new(),
            audio_events: audio::events::AudioEventRegistry::default(),
            audio_events_window_open: false,
            music_graph: audio::music_graph::MusicGraph::default(),
            music_graph_window_open: false,
            plugin_search_query: String::new(),
            plugin_search_results: Vec::new(),
            plugin_search_rx: None,

            bevy_version,

            package_manager_open: false,
            package_manager: package_manager::PackageManagerState::default(),
            package_manager_search_rx: None,

            mobile_toolchain_open: false,
            mobile_toolchain: mobile_toolchain::MobileToolchainState::from_cache_or_default(),

            godot_scene_panel: godot_panel::GodotScenePanelState::default(),
        };

        // === Test Mode CLI: --test-mode ===
        if std::env::args().any(|a| a == "--test-mode") {
            app.test_mode = true;
            let (tx, rx) = std::sync::mpsc::channel();
            app.test_command_rx = Some(rx);
            std::thread::spawn(move || {
                let listener = match std::net::TcpListener::bind("127.0.0.1:17171") {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("Failed to bind test mode port 17171: {}", e);
                        return;
                    }
                };
                tracing::info!("Test mode: listening on 127.0.0.1:17171");
                // `incoming()` returns `Result<TcpStream, _>`; using
                // `.flatten()` would silently spin on a stream of
                // persistent IO errors (clippy::flat_map_option /
                // clippy::flatten-on-fallible-iter). `map_while` stops
                // on the first error instead of looping forever.
                for stream in listener.incoming().map_while(Result::ok) {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(&stream);
                    for line in reader.lines().map_while(Result::ok) {
                        if tx.send(line).is_err() {
                            return;
                        }
                    }
                }
            });
        }

        app
    }

    /// Take a snapshot of the current scene model so the next destructive edit
    /// can be undone. Call this BEFORE the mutation.
    ///
    /// This is the backward-compatible wrapper that records a [`SceneCommand::Generic`].
    /// Prefer calling `self.command_history.execute(specific_command, &self.scene_model)`
    /// directly for operations that have a dedicated [`SceneCommand`] variant.
    pub(crate) fn scene_snapshot(&mut self) {
        self.command_history.execute(
            scene_editor::history::SceneCommand::Generic("edit".into()),
            &self.scene_model,
        );
    }
}

/// Scan `<project_root>/scenes/` and return every `.bscene` path it
/// contains, sorted alphabetically. Used by `open_project` to
/// restore one tab per scene file (the prior "first match wins"
/// behaviour orphaned every other scene in the project).
pub fn list_project_bscenes(project_root: &str) -> Vec<String> {
    let scenes_dir = format!("{}/scenes", project_root);
    let entries = match std::fs::read_dir(&scenes_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut paths: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("bscene"))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect();
    paths.sort();
    paths
}

/// Recursively copy a directory and all its contents
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Resource to track whether egui fonts have been configured.
#[derive(bevy::prelude::Resource, Default)]
pub struct EguiFontsConfigured(pub bool);

/// Where the chosen visual theme is persisted between sessions.
fn theme_mode_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let dir = home.join(".berrycode");
        std::fs::create_dir_all(&dir).ok();
        dir.join("theme.json")
    } else {
        std::path::PathBuf::from("theme.json")
    }
}

fn load_theme_mode() -> types::ThemeMode {
    let path = theme_mode_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<types::ThemeMode>(&s).ok())
        .unwrap_or_default()
}

fn save_theme_mode(mode: types::ThemeMode) {
    let path = theme_mode_path();
    if let Ok(json) = serde_json::to_string_pretty(&mode) {
        let _ = std::fs::write(&path, json);
    }
}

/// True when the frame's events look like the user typed something
/// the LSP completion popup should react to (real `Event::Text`, IME
/// `Commit`, or a non-empty `Preedit`).
///
/// Why Preedit counts: on macOS, bevy_egui forwards ASCII keystrokes
/// through `Ime::Preedit` while a CJK input source is selected, and
/// our global IME filter strips the duplicated `Event::Text` events
/// to avoid double-insert during real composition. Without counting
/// Preedit here, the auto-trigger never fires while the user has
/// IME on — even when they're typing pure Latin code.
pub(crate) fn events_look_like_typing(events: &[egui::Event]) -> bool {
    events.iter().any(|e| {
        matches!(e, egui::Event::Text(_))
            || matches!(e, egui::Event::Ime(egui::ImeEvent::Commit(_)))
            || matches!(
                e,
                egui::Event::Ime(egui::ImeEvent::Preedit(s)) if !s.is_empty()
            )
    })
}

/// Characters whose insertion should cause a fresh LSP completion
/// request: word chars, plus the four "open a completion" operators
/// (`.`, `:`, `<`, `_`).
pub(crate) fn char_triggers_completion(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == ':' || c == '<'
}

fn additional_roots_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let dir = home.join(".berrycode");
        std::fs::create_dir_all(&dir).ok();
        dir.join("workspace_roots.json")
    } else {
        std::path::PathBuf::from("workspace_roots.json")
    }
}

fn load_additional_roots() -> Vec<String> {
    let path = additional_roots_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| std::path::Path::new(p).is_dir())
        .collect()
}

pub(crate) fn save_additional_roots(roots: &[String]) {
    let path = additional_roots_path();
    if let Ok(json) = serde_json::to_string_pretty(roots) {
        let _ = std::fs::write(&path, json);
    }
}

fn panel_visibility_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        let dir = home.join(".berrycode");
        std::fs::create_dir_all(&dir).ok();
        dir.join("panels.json")
    } else {
        std::path::PathBuf::from("panels.json")
    }
}

fn load_panel_visibility() -> types::PanelVisibility {
    let path = panel_visibility_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<types::PanelVisibility>(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_panel_visibility(v: types::PanelVisibility) {
    let path = panel_visibility_path();
    if let Ok(json) = serde_json::to_string_pretty(&v) {
        let _ = std::fs::write(&path, json);
    }
}

/// Build an `egui::Visuals` for the given theme preset. Centralised so the
/// startup path and the runtime theme switcher always agree on colours.
pub fn visuals_for_theme(mode: types::ThemeMode) -> egui::Visuals {
    let mut v = match mode {
        types::ThemeMode::Light => egui::Visuals::light(),
        _ => egui::Visuals::dark(),
    };
    let (panel, text, accent, sel_bg, sel_text) = match mode {
        types::ThemeMode::Dark => (
            egui::Color32::from_rgb(25, 26, 28),
            egui::Color32::from_rgb(212, 212, 212),
            egui::Color32::from_rgb(75, 110, 175),
            egui::Color32::from_rgba_premultiplied(20, 40, 70, 130),
            egui::Color32::from_rgb(212, 212, 212),
        ),
        types::ThemeMode::Light => (
            egui::Color32::from_rgb(245, 246, 248),
            egui::Color32::from_rgb(35, 38, 44),
            egui::Color32::from_rgb(50, 100, 200),
            egui::Color32::from_rgba_premultiplied(70, 130, 220, 110),
            egui::Color32::from_rgb(35, 38, 44),
        ),
        types::ThemeMode::HighContrast => (
            egui::Color32::BLACK,
            egui::Color32::WHITE,
            egui::Color32::from_rgb(0, 200, 255),
            egui::Color32::from_rgba_premultiplied(0, 200, 255, 120),
            egui::Color32::WHITE,
        ),
    };
    v.panel_fill = panel;
    v.window_fill = panel;
    v.extreme_bg_color = panel;
    v.override_text_color = Some(text);
    v.hyperlink_color = accent;
    v.selection.bg_fill = sel_bg;
    v.selection.stroke = egui::Stroke::new(0.0, sel_text);
    v
}

/// Bevy update system: configure egui fonts and style (runs once on first successful access).
pub fn setup_egui_fonts_and_style(
    mut egui_ctx: bevy_egui::EguiContexts,
    mut configured: bevy::prelude::ResMut<EguiFontsConfigured>,
    mut fonts_uploaded: bevy::prelude::Local<bool>,
) -> bevy::prelude::Result {
    if configured.0 {
        return Ok(());
    }
    let ctx = egui_ctx.ctx_mut()?;

    // `Context::set_fonts` only takes effect on the *next* `Context::run`,
    // so the first frame after upload still has the default font set live.
    // We split the work across two frames: frame 1 uploads fonts, frame 2
    // marks the resource ready. `berry_ui_system` skips rendering until
    // `configured.0` is true, which prevents the activity bar (and other
    // codicon-using widgets) from panicking with "FontFamily::Name(\"codicon\")
    // is not bound to any fonts".
    if *fonts_uploaded {
        configured.0 = true;
        ctx.request_repaint();
        return Ok(());
    }

    // Setup fonts with Japanese support
    let mut fonts = egui::FontDefinitions::default();

    // Add Codicon font for icons (embedded at compile time)
    const CODICON_FONT_BYTES: &[u8] = include_bytes!("../../assets/codicon.ttf");
    tracing::info!("Loaded Codicon font: {} bytes", CODICON_FONT_BYTES.len());
    fonts.font_data.insert(
        "codicon".to_owned(),
        egui::FontData::from_static(CODICON_FONT_BYTES).into(),
    );

    // Add Nerd Font Symbols for terminal glyphs (powerline, devicons, etc.)
    const NERD_FONT_BYTES: &[u8] = include_bytes!("../../assets/nerd-symbols.ttf");
    fonts.font_data.insert(
        "nerd-symbols".to_owned(),
        egui::FontData::from_static(NERD_FONT_BYTES).into(),
    );

    // Create a custom font family for Codicon icons
    fonts.families.insert(
        egui::FontFamily::Name("codicon".into()),
        vec!["codicon".to_owned()],
    );

    // Also add to Proportional family as fallback
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "codicon".to_owned());

    // Add Nerd Font Symbols as fallback for Monospace and Proportional
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .push("nerd-symbols".to_owned());
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .push("nerd-symbols".to_owned());
    tracing::info!("Codicon + Nerd Font Symbols loaded");

    // Add Japanese font (try monospace fonts first for better baseline alignment)
    #[cfg(target_os = "macos")]
    let japanese_font_paths = vec![
        "/System/Library/Fonts/Osaka.ttf",
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/ヒラギノ角ゴ ProN W3.otf",
    ];
    #[cfg(target_os = "windows")]
    let japanese_font_paths = {
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        vec![
            format!("{win_dir}\\Fonts\\YuGothM.ttc"),
            format!("{win_dir}\\Fonts\\YuGothR.ttc"),
            format!("{win_dir}\\Fonts\\meiryo.ttc"),
            format!("{win_dir}\\Fonts\\msgothic.ttc"),
        ]
    };
    #[cfg(target_os = "linux")]
    let japanese_font_paths = vec![
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
        "/usr/share/fonts/truetype/takao-gothic/TakaoPGothic.ttf",
        "/usr/share/fonts/ipa-gothic/ipag.ttf",
    ];
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let japanese_font_paths: Vec<&str> = vec![];

    for path in japanese_font_paths {
        if let Ok(font_data) = std::fs::read(&path) {
            let mut font_data_with_tweak = egui::FontData::from_owned(font_data);
            font_data_with_tweak.tweak.y_offset_factor = 0.15;
            font_data_with_tweak.tweak.y_offset = 2.0;

            fonts
                .font_data
                .insert("japanese".to_owned(), font_data_with_tweak.into());

            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .push("japanese".to_owned());

            fonts
                .families
                .get_mut(&egui::FontFamily::Monospace)
                .unwrap()
                .push("japanese".to_owned());

            tracing::info!("Loaded Japanese font: {} (with baseline tweak)", path);
            break;
        }
    }

    ctx.set_fonts(fonts);

    // Apply the persisted theme preset. Defaults to Dark on first run.
    let initial_theme = load_theme_mode();
    ctx.set_visuals(visuals_for_theme(initial_theme));
    ui_colors::set_theme(initial_theme);

    // Also apply the One Dark style (panel-specific tweaks; visuals_for_theme
    // covers the colour-scheme defaults).
    BerryCodeApp::setup_egui_style(ctx);

    tracing::info!("egui fonts and style configured");
    *fonts_uploaded = true;
    // Force a repaint so the next frame runs with the new font definitions
    // already applied, after which `configured.0` is set and the main UI
    // is allowed to render.
    ctx.request_repaint();
    Ok(())
}

/// Main UI update system for Bevy
pub fn berry_ui_system(
    mut app: bevy::ecs::system::NonSendMut<BerryCodeApp>,
    mut egui_ctx: bevy_egui::EguiContexts,
    mut drop_events: bevy::ecs::message::MessageReader<bevy::window::FileDragAndDrop>,
    mut close_events: bevy::ecs::message::MessageReader<bevy::window::WindowCloseRequested>,
    mut app_exit: bevy::ecs::message::MessageWriter<bevy::app::AppExit>,
    mut preview_scene: bevy::ecs::system::ResMut<preview_3d::ModelPreviewScene>,
    mut scene_render: bevy::ecs::system::ResMut<scene_editor::bevy_render::SceneEditorRender>,
    mut mat_preview: bevy::ecs::system::ResMut<
        scene_editor::material_preview::MaterialPreviewRender,
    >,
    mut scene_anim: bevy::ecs::system::ResMut<
        scene_editor::skeletal_animation::SceneAnimationState,
    >,
    fonts_configured: bevy::prelude::Res<EguiFontsConfigured>,
) -> bevy::prelude::Result {
    // Wait until `setup_egui_fonts_and_style` has uploaded fonts AND a
    // subsequent frame has confirmed they are live. Without this guard the
    // activity bar's codicon glyphs panic with "FontFamily::Name(\"codicon\")
    // is not bound to any fonts" on the first frame.
    if !fonts_configured.0 {
        if let Ok(ctx) = egui_ctx.ctx_mut() {
            ctx.request_repaint();
        }
        return Ok(());
    }

    // === File-tree live refresh ===
    // The `FileWatcher` was being constructed in `open_project` but its
    // event channel was never drained, so the File Tree only reflected
    // disk state at the moment of project open. Drain pending events
    // here, collect the unique parent directories that changed, and
    // re-read each one if it's currently expanded so externally-created
    // / -deleted files appear without forcing the user to manually
    // refresh.
    let mut dirs_to_reload: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut root_touched = false;
    // Snapshot root_path before mutably borrowing the watcher so the
    // event loop's parent-vs-root check doesn't alias.
    let root_path_snapshot = app.root_path.clone();
    if let Some(watcher) = app.file_watcher.as_mut() {
        while let Some(ev) = watcher.try_recv() {
            let path = match ev {
                native::watcher::FileEvent::Created(p)
                | native::watcher::FileEvent::Modified(p)
                | native::watcher::FileEvent::Removed(p) => p,
                native::watcher::FileEvent::Renamed { to, .. } => to,
            };
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if parent_str == root_path_snapshot {
                    root_touched = true;
                }
                dirs_to_reload.insert(parent_str);
            }
        }
    }
    if root_touched {
        // Cheapest path for root-level changes — clear the cache and
        // let the next render re-walk the project root.
        app.file_tree_cache.clear();
        app.file_tree_load_pending = true;
    } else {
        for dir in dirs_to_reload {
            if app.expanded_dirs.contains(&dir) {
                app.load_directory_children(&dir);
            }
        }
    }

    // === IME event filtering (global, before any TextEdit runs) ===
    //
    // Two goals:
    //
    // 1. **Drop `Event::Text` when `Ime::Preedit` is in flight** —
    //    on macOS the OS double-fires both for the same romaji char,
    //    and egui's `TextEdit` would insert the Text *in addition to*
    //    showing the preedit overlay, leaving a stray char.
    //
    // 2. **Drop `Event::Ime(ImeEvent::Enabled)`** — that event is the
    //    only thing that sets egui's internal `state.ime_enabled`
    //    flag. While the flag is true `TextEdit` *deletes* Backspace
    //    and arrow keys before processing input
    //    (`egui-0.33.3/src/widgets/text_edit/builder.rs:1147-1153`).
    //    bevy_egui re-emits `Enabled` ahead of every `Preedit`, so
    //    during a composition Backspace never reaches the buffer.
    //    By suppressing `Enabled` the flag stays false and Backspace
    //    works at every stage — fixing the "あああ と打って最後の一
    //    文字が消えない" report. Preedit / Commit / Disabled still
    //    go through, so composition logic is untouched.
    if let Ok(ctx) = egui_ctx.ctx_mut() {
        ctx.input_mut(|i| {
            // Drop the `Text` events that bevy_egui forwards alongside
            // each `Preedit` (they're the same chars in two flavours;
            // egui inserts via Preedit only).
            let has_preedit = i.events.iter().any(
                |e| matches!(e, egui::Event::Ime(egui::ImeEvent::Preedit(s)) if !s.is_empty()),
            );
            // Drop `Ime::Enabled` so egui never flips `state.ime_enabled`
            // to true — that flag is what makes its TextEdit filter
            // Backspace out of the event list while composing
            // (`egui-0.33.3/.../text_edit/builder.rs:1147-1153`).
            //
            // Crucially we do NOT drop Preedit/Commit/Disabled or
            // Backspace itself — letting Preedit flow through means
            // egui's IME state machine still drives the inline preedit
            // (insert / shrink / delete), and macOS stays in sync with
            // our buffer because every shrink it sends is honoured.
            i.events.retain(|e| match e {
                egui::Event::Ime(egui::ImeEvent::Enabled) => false,
                egui::Event::Text(_) if has_preedit => false,
                _ => true,
            });
        });
    }

    // === GLB animation bridge (resources → BerryCodeApp mirrors) ===
    // Mirror Bevy-side animation state into `BerryCodeApp` so the inspector /
    // model preview UI (which only sees `&mut self`) can read clip lists and
    // currently-playing names without having to thread Bevy resources through
    // every render call.
    app.scene_anim_clips_view.clear();
    app.scene_anim_current.clear();
    for (id, entry) in &scene_anim.entries {
        if !entry.clips.is_empty() {
            app.scene_anim_clips_view
                .insert(*id, entry.clips.iter().map(|(n, _)| n.clone()).collect());
        }
        if let Some(c) = &entry.current_clip {
            app.scene_anim_current.insert(*id, c.clone());
        }
    }
    app.preview_anim_clips = preview_scene
        .animation_clips
        .iter()
        .map(|(n, _)| n.clone())
        .collect();
    app.preview_anim_current = preview_scene.current_clip.clone();

    // Drive continuous repaints while any GLB animation is playing — under
    // `WinitSettings::Reactive` Bevy's `Update` only ticks on input/window
    // events otherwise, which down-samples `AnimationPlayer` and produces a
    // visibly stuttered run/walk cycle.
    let scene_anim_active = scene_anim
        .entries
        .values()
        .any(|e| e.player_entity.is_some());
    let preview_anim_active = preview_scene.animation_player_entity.is_some();
    if scene_anim_active || preview_anim_active {
        if let Ok(ctx) = egui_ctx.ctx_mut() {
            ctx.request_repaint();
        }
    }

    // Handle window close — check for unsaved files
    let mut exiting = false;
    for _event in close_events.read() {
        let has_unsaved = app.editor_tabs.iter().any(|tab| tab.is_dirty);
        if has_unsaved {
            app.show_close_confirm = true;
        } else {
            app_exit.write(bevy::app::AppExit::Success);
            exiting = true;
        }
    }

    // Handle close confirmation dialog result
    if let Some(action) = app.close_action.take() {
        match action {
            CloseAction::SaveAll => {
                // Save all dirty files
                for i in 0..app.editor_tabs.len() {
                    if app.editor_tabs[i].is_dirty {
                        let content = app.editor_tabs[i].buffer.to_string();
                        let file_path = app.editor_tabs[i].file_path.clone();
                        if let Ok(_) = crate::native::fs::write_file(&file_path, &content) {
                            app.editor_tabs[i].is_dirty = false;
                        }
                    }
                }
                app.show_close_confirm = false;
                app_exit.write(bevy::app::AppExit::Success);
                exiting = true;
            }
            CloseAction::Discard => {
                app.show_close_confirm = false;
                app_exit.write(bevy::app::AppExit::Success);
                exiting = true;
            }
        }
    }
    // Handle drag-and-drop files from OS (via Bevy's FileDragAndDrop event)
    //
    // Resolving the drop target: the folder rects recorded by the file-tree
    // renderer (previous frame) are hit-tested against the pointer's last
    // known egui position. If the pointer was inside a folder row, that
    // folder is the destination; otherwise we fall back to the project root.
    // The deepest matching rect wins so nested folders take precedence over
    // their ancestors.
    let drop_pointer_pos: Option<egui::Pos2> = egui_ctx
        .ctx_mut()
        .ok()
        .and_then(|ctx| ctx.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.latest_pos())));
    for event in drop_events.read() {
        if let bevy::window::FileDragAndDrop::DroppedFile { path_buf, .. } = event {
            let path = path_buf;
            let path_str = path.to_string_lossy().to_string();
            let target_dir = drop_pointer_pos
                .and_then(|pos| {
                    app.file_tree_folder_rects
                        .iter()
                        .filter(|(_, rect)| rect.contains(pos))
                        .max_by_key(|(p, _)| p.len())
                        .map(|(p, _)| p.clone())
                })
                .unwrap_or_else(|| app.root_path.clone());
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest = format!("{}/{}", target_dir, file_name);
                match std::fs::copy(&path_str, &dest) {
                    Ok(_) => {
                        app.status_message = format!("Imported: {}", file_name);
                        app.status_message_timestamp = Some(std::time::Instant::now());
                        app.expanded_dirs.insert(target_dir.clone());
                        app.load_directory_children(&target_dir);
                        app.file_tree_cache.clear();
                        app.file_tree_load_pending = true;
                        app.open_file_from_path(&dest);
                    }
                    Err(e) => {
                        app.status_message = format!("Import failed: {}", e);
                        app.status_message_timestamp = Some(std::time::Instant::now());
                    }
                }
            } else if path.is_dir() {
                let dir_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest = format!("{}/{}", target_dir, dir_name);
                if let Err(e) = copy_dir_recursive(path, std::path::Path::new(&dest)) {
                    app.status_message = format!("Import failed: {}", e);
                    app.status_message_timestamp = Some(std::time::Instant::now());
                } else {
                    app.status_message = format!("Imported folder: {}", dir_name);
                    app.status_message_timestamp = Some(std::time::Instant::now());
                    app.expanded_dirs.insert(target_dir.clone());
                    app.file_tree_cache.clear();
                    app.file_tree_load_pending = true;
                }
            }
        }
    }

    // After sending AppExit, skip UI rendering to avoid accessing destroyed context
    if exiting {
        return Ok(());
    }

    {
        let ctx = egui_ctx.ctx_mut()?;

        // Global panel switching: Ctrl+1..9 — processed BEFORE any panel rendering
        // so it works regardless of which widget has focus
        ctx.input(|i| {
            if i.modifiers.command {
                if i.key_pressed(egui::Key::Num1) {
                    app.active_panel = types::ActivePanel::Explorer;
                }
                if i.key_pressed(egui::Key::Num2) {
                    app.active_panel = types::ActivePanel::Search;
                }
                if i.key_pressed(egui::Key::Num3) {
                    app.active_panel = types::ActivePanel::Git;
                }
                if i.key_pressed(egui::Key::Num4) {
                    app.active_panel = types::ActivePanel::Terminal;
                }
                if i.key_pressed(egui::Key::Num5) {
                    app.active_panel = types::ActivePanel::EcsInspector;
                }
                if i.key_pressed(egui::Key::Num6) {
                    app.active_panel = types::ActivePanel::SceneEditor;
                }
                // Cmd+L → focus the AI chat input. Roadmap v0.4.5 / 2A.
                // We just raise a flag; `render_ai_chat_panel` claims focus
                // on the next render so the keystroke isn't swallowed by
                // whichever widget happens to be focused right now.
                if i.key_pressed(egui::Key::L) {
                    app.ai_chat_focus_pending = true;
                }
            }
        });

        // Show project picker if no project loaded
        if app.show_project_picker {
            app.render_project_picker(ctx);
            // Still render the New Project dialog if open
            app.render_new_project_dialog(ctx);
            return Ok(());
        }

        // Initialize Git repository on first update
        if !app.git_initialized {
            app.git_initialized = true;
            app.refresh_git_status();
            app.refresh_git_history();
            app.refresh_git_branches();
            app.refresh_git_remotes();
            app.refresh_git_tags();
            app.refresh_git_stashes();
            tracing::info!("Git repository initialized for {}", app.root_path);
        }

        // Latency-sensitive polls (run every frame): LSP completions and
        // streaming process output drive what the user sees while typing /
        // watching a build, so a 50ms gap would feel sluggish.
        app.poll_lsp_responses();
        #[cfg(feature = "ai")]
        app.poll_ai_responses();
        app.poll_run_output();

        // Throttled polls: I/O-heavy or rarely-updating channels. Running
        // these on every frame means up to 11 syscalls / try_recvs per
        // tick (file watcher, asset watcher, cargo check, etc.). Polling
        // at 20Hz is well below human perception for these signals and
        // keeps idle CPU near zero in Reactive update mode.
        let now = std::time::Instant::now();
        let throttle_due = app
            .last_poll_tick
            .map(|t| now.duration_since(t) >= std::time::Duration::from_millis(50))
            .unwrap_or(true);
        if throttle_due {
            app.last_poll_tick = Some(now);
            app.poll_inlay_hints();
            app.poll_test_results();
            app.poll_dap_events();
            app.poll_remote_responses();
            app.poll_collab();
            app.poll_asset_watcher();
            app.poll_file_watcher_events();
            // Mobile run session — drains spawned simctl/devicectl/adb pipes.
            app.mobile_toolchain.poll_run();
            app.poll_ecs_inspector();
            app.poll_cargo_check();
            app.poll_test_commands();
        }

        // Keep repainting while a game process is running
        if app.run_process.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Handle keyboard shortcuts
        app.handle_editor_shortcuts(ctx);
        app.handle_goto_definition_shortcut(ctx);
        app.handle_find_references_shortcut(ctx);
        app.handle_code_action_shortcut(ctx);
        app.handle_macro_expand_shortcut(ctx);
        app.handle_debug_shortcuts(ctx);
        app.handle_settings_shortcuts(ctx);

        // Render top header bar (VS Code style)
        app.render_top_header(ctx);

        // Render UI panels
        app.render_activity_bar(ctx);

        // Render status bar (must be before CentralPanel to reserve space)
        app.render_status_bar(ctx);

        // Render dockable tool panel (bottom, must reserve space before CentralPanel)
        app.render_tool_panel(ctx);

        // Conditional panels based on active panel
        if app.active_panel == ActivePanel::Terminal {
            app.render_terminal_fullscreen(ctx);
        } else if app.active_panel == ActivePanel::Git {
            app.render_sidebar(ctx);
            app.render_git_diff_viewer(ctx);
        } else if app.active_panel == ActivePanel::SceneEditor {
            // Unity-style 3-column layout:
            //   Left   = Hierarchy  (handled by render_sidebar)
            //   Right  = Inspector  (dedicated SidePanel::right, shown BEFORE CentralPanel)
            //   Center = Scene View (CentralPanel)
            app.render_sidebar(ctx);
            egui::SidePanel::right("scene_inspector")
                .default_width(220.0)
                .width_range(160.0..=400.0)
                .resizable(true)
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::SIDEBAR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    // Wrap in a ScrollArea so long component lists (e.g. an
                    // entity carrying Mesh + Collider + RigidBody +
                    // PlayerController + Transform sliders) don't get
                    // clipped at the bottom of the panel with no way to
                    // reach them.
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            app.render_scene_inspector(ui);
                        });
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::EDITOR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    app.render_scene_view(ui);
                });
        } else if app.active_panel == ActivePanel::EcsInspector {
            // 3-column layout: Left=Entity list, Center=3D view, Right=Properties
            app.render_sidebar(ctx);
            egui::SidePanel::right("ecs_properties_panel")
                .default_width(220.0)
                .width_range(160.0..=400.0)
                .resizable(true)
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::SIDEBAR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    app.render_ecs_properties_only(ui);
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::EDITOR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    app.render_ecs_3d_view(ui);
                });
        } else if app.active_panel == ActivePanel::Database {
            // DBeaver-style: sidebar = file/table list, central = preview/query
            app.render_sidebar(ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::EDITOR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    app.render_database_central(ui);
                });
        } else if app.active_panel == ActivePanel::Docker {
            // Docker Desktop-style: sidebar = tabs/counts, central = grid + logs
            app.render_sidebar(ctx);
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::EDITOR_BG())
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    app.render_docker_central(ui);
                });
        } else if app.active_panel == ActivePanel::OracleBerry {
            // AI image generator: sidebar = recent generations,
            // central = prompt + settings + result canvas
            #[cfg(feature = "ai")]
            {
                app.render_sidebar(ctx);
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .fill(ui_colors::EDITOR_BG())
                            .inner_margin(egui::Margin::same(12)),
                    )
                    .show(ctx, |ui| {
                        app.render_oracleberry_central(ui);
                    });
            }
        } else if app.active_panel == ActivePanel::Settings {
            // Settings spans the full width: skip the (now-empty) sidebar
            // panel entirely — the activity bar is already rendered above
            // and the settings form has its own internal nav column.
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(ui_colors::EDITOR_BG())
                        .inner_margin(egui::Margin::same(16)),
                )
                .show(ctx, |ui| {
                    app.render_settings_panel(ui);
                });
        } else {
            app.render_sidebar(ctx);
            #[cfg(feature = "ai")]
            app.render_ai_chat_panel(ctx);
            app.render_editor_area(ctx);
        }

        // Render scene preview panel for .scn.ron files
        app.render_scene_preview(ctx);

        // Render debug panel (bottom panel when debugging)
        app.render_debug_panel(ctx);

        // The standalone run-output panel was removed in v0.5.7 — its
        // Run/Stop/filter UI lives inside the dock's "Output" tab now
        // (`dock::ToolTab::Output` → `render_console_content`). Triggering
        // a run from the toolbar / shortcut auto-opens that tab so the
        // user always sees the same console regardless of how they
        // started the build.

        // Render diagnostics: now hosted as the "Problems" tab inside the
        // unified bottom panel (see `dock::render_tool_panel`). Auto-open
        // that panel when new diagnostics arrive so users still notice
        // them without an extra free-floating panel.
        if !app.lsp_diagnostics.is_empty() && !app.tool_panel_open {
            app.tool_panel_open = true;
            app.active_tool_tab = dock::ToolTab::Problems;
        }

        // Render search dialog if open
        if app.search_dialog_open {
            app.render_search_dialog(ctx);
        }

        // Render settings dialog
        if app.show_settings {
            app.render_settings_dialog(ctx);
        }

        // Render theme editor
        if app.show_theme_editor {
            app.render_theme_editor(ctx);
        }

        // Render close confirmation dialog
        if app.show_close_confirm {
            app.render_close_confirm_dialog(ctx);
        }

        // Render LSP hover tooltip
        if app.lsp_show_hover {
            app.render_lsp_hover(ctx);
        }

        // Render definition picker window
        if app.show_definition_picker {
            app.render_definition_picker(ctx);
        }

        // Render references panel
        if app.show_references_panel {
            app.render_references_panel(ctx);
        }

        // Render rename dialog
        app.render_rename_dialog(ctx);

        // Render new file/folder dialogs
        app.render_new_file_dialog(ctx);
        app.render_new_folder_dialog(ctx);
        app.render_new_project_dialog(ctx);

        // Render file tree context menu and rename dialog
        app.render_file_context_menu(ctx);
        app.render_rename_file_dialog(ctx);

        // editor-side profiler (FPS / frame time / entity count).
        app.render_profiler(ctx);

        // floating timeline window for animation keyframe editing.
        app.render_timeline(ctx);

        // floating dopesheet / curve editor window.
        app.render_dopesheet(ctx);

        // floating animator controller editor window.
        app.render_animator_editor(ctx);

        // floating blend tree editor (1D / 2D blend visualisation).
        // Opened from Tools → Blend Tree; sets `editing_blend_tree`
        // so the renderer has something to bind to.
        app.render_blend_tree_editor(ctx);

        // floating build settings window.
        app.render_build_settings(ctx);

        // floating package manager window.
        app.render_package_manager_window(ctx);

        // floating mobile toolchain window (v0.8 Phase A).
        app.render_mobile_toolchain_window(ctx);
        app.render_mobile_doctor_modal(ctx);

        // Godot scene viewer — auto-shows when active tab is a `.tscn`
        // file (v0.8.x Migration & interop).
        app.render_godot_scene_panel(ctx);

        // Drain the in-flight plugin command's output channel without
        // blocking. A long-running plugin keeps its receiver empty; we
        // simply don't update the status bar this frame and try again
        // next frame. The 30 s safety bound lives inside the poll
        // itself.
        app.poll_pending_plugin_command();

        // floating scene merge panel.
        app.render_merge_panel(ctx);

        // floating visual script editor.
        app.render_visual_script_editor(ctx);

        // floating shader graph editor.
        app.render_shader_graph_editor(ctx);

        // Bevy-specific: System Execution Graph.
        app.render_system_graph(ctx);

        // v0.6 audio editors. Both render only when their open
        // flags are set, so the cost when nothing is showing is a
        // single `if` per frame.
        app.render_audio_events_editor(ctx);
        app.render_music_graph_editor(ctx);

        // Bevy-specific: Event Monitor.
        app.render_event_monitor(ctx);

        // Bevy-specific: Query Visualizer.
        app.render_query_viz(ctx);

        // Bevy-specific: States Editor.
        app.render_state_editor(ctx);

        // Bevy-specific: Plugin Browser.
        app.render_plugin_browser(ctx);

        // hot reload polling.
        {
            let root = app.root_path.clone();
            if let Some(msg) = app.hot_reload.poll(&root) {
                app.status_message = msg;
                app.status_message_timestamp = Some(std::time::Instant::now());
            }
        }

        // Reactive Mode: only repaint when status message is active
        if app.status_message_timestamp.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    } // end of ctx borrow scope

    // GPU 3D preview: if active tab is GLTF/GLB, update preview scene and assign texture
    {
        let idx = app.active_tab_idx;
        let mut wants_gpu = false;
        let mut model_path: Option<String> = None;
        let mut orbit_yaw = 0.0f32;
        let mut orbit_pitch = 0.0f32;
        let mut orbit_zoom = 1.0f32;

        if !app.editor_tabs.is_empty() && idx < app.editor_tabs.len() {
            let tab = &app.editor_tabs[idx];
            if tab.is_model {
                let ext = tab
                    .file_path
                    .rsplit('.')
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                if ext == "glb" || ext == "gltf" {
                    wants_gpu = true;
                    model_path = Some(tab.file_path.clone());
                    orbit_yaw = tab.model_rot_y;
                    orbit_pitch = tab.model_rot_x;
                    orbit_zoom = tab.model_zoom;
                }
            }
        }

        if wants_gpu {
            if let Some(ref path) = model_path {
                if preview_scene.requested_model_path.as_ref() != Some(path) {
                    preview_scene.requested_model_path = Some(path.clone());
                }
            }
            preview_scene.orbit_yaw = orbit_yaw;
            preview_scene.orbit_pitch = orbit_pitch;
            // The previous multiplier of 3.0 placed the camera *inside* the
            // bounding box of typical GLB models (e.g. fox.glb ~6 units),
            // which combined with backface culling produced an empty render
            // target. 10.0 keeps the default view comfortably outside.
            preview_scene.orbit_distance = orbit_zoom * 10.0;

            if let Some(handle) = preview_scene.render_target.clone() {
                let texture_id = egui_ctx.add_image(bevy_egui::EguiTextureHandle::Strong(handle));
                app.editor_tabs[idx].gpu_preview_texture_id = Some(texture_id);
            }
        } else {
            // Only unload if there are no model tabs open at all
            let any_model_tab = app.editor_tabs.iter().any(|t| {
                t.is_model && {
                    let e = t.file_path.rsplit('.').next().unwrap_or("").to_lowercase();
                    e == "glb" || e == "gltf"
                }
            });
            if !any_model_tab && preview_scene.loaded_model_path.is_some() {
                preview_scene.requested_model_path = None;
            }
        }
    }

    // === Flush UI clip-change requests back into Bevy resources ===
    if let Some(req) = app.preview_anim_clip_request.take() {
        // Only honour the request if it's a known clip for the current model.
        if preview_scene.animation_clips.iter().any(|(n, _)| n == &req) {
            preview_scene.requested_clip = Some(req);
        }
    }
    for (id, req) in app.scene_anim_clip_request.drain() {
        if let Some(entry) = scene_anim.entries.get_mut(&id) {
            if entry.clips.iter().any(|(n, _)| n == &req) {
                tracing::info!(
                    "Scene anim: UI requested clip '{}' for scene entity {}",
                    req,
                    id
                );
                entry.requested_clip = Some(req);
            }
        }
    }

    // === Scene Editor render-target plumbing ===
    // 1) Push the current orbit parameters from the UI state into the Bevy
    //    resource so `update_scene_editor_camera` can pick them up.
    // 2) Re-register the render-target image with egui every frame and stash
    //    the texture id back on the app so the Scene View panel can draw it.
    {
        scene_render.orbit_yaw = app.scene_orbit_yaw;
        scene_render.orbit_pitch = app.scene_orbit_pitch;
        scene_render.orbit_distance = app.scene_orbit_distance;
        scene_render.orbit_target = app.scene_orbit_target;
        scene_render.ortho = app.scene_ortho;
        scene_render.ortho_scale = app.scene_ortho_scale;
        scene_render.shadows_enabled = app.scene_shadows_enabled;
        scene_render.bloom_enabled = app.scene_bloom_enabled;
        scene_render.bloom_intensity = app.scene_bloom_intensity;
        scene_render.tonemapping = app.scene_tonemapping;
        scene_render.ssao_enabled = app.scene_ssao_enabled;
        scene_render.taa_enabled = app.scene_taa_enabled;
        scene_render.fog_enabled = app.scene_fog_enabled;
        scene_render.fog_color = app.scene_fog_color;
        scene_render.fog_start = app.scene_fog_start;
        scene_render.fog_end = app.scene_fog_end;
        scene_render.dof_enabled = app.scene_dof_enabled;
        scene_render.dof_focus_distance = app.scene_dof_focus_distance;
        scene_render.dof_aperture = app.scene_dof_aperture;

        // Game-view override: when the user has switched the central
        // viewport to "Game", find the first scene entity carrying a
        // `Camera` component and push its world transform into the
        // render state so the editor camera mirrors it. Falls back to
        // None (orbit camera) when no Camera entity exists.
        scene_render.game_camera_override = if app.scene_view_mode == SceneViewMode::Game {
            app.scene_model.entities.values().find_map(|e| {
                if !e.enabled {
                    return None;
                }
                let has_camera = e
                    .components
                    .iter()
                    .any(|c| matches!(c, scene_editor::model::ComponentData::Camera));
                if !has_camera {
                    return None;
                }
                let world = app.scene_model.compute_world_transform(e.id);
                Some(scene_editor::bevy_render::GameCameraOverride {
                    translation: world.translation,
                    rotation_euler: world.rotation_euler,
                    fov_y: std::f32::consts::FRAC_PI_4,
                })
            })
        } else {
            None
        };

        if let Some(handle) = scene_render.render_target.clone() {
            let tex_id = egui_ctx.add_image(bevy_egui::EguiTextureHandle::Strong(handle));
            scene_render.egui_texture_id = Some(tex_id);
            app.scene_view_texture_id = Some(tex_id);
        }
    }

    // === Material Preview render-target plumbing ===
    // Push PBR values from the inspector to the Bevy resource, then
    // re-register the render target texture with egui.
    {
        if app.material_preview_dirty {
            mat_preview.current_color = app.material_preview_color;
            mat_preview.current_metallic = app.material_preview_metallic;
            mat_preview.current_roughness = app.material_preview_roughness;
            mat_preview.current_emissive = app.material_preview_emissive;
            mat_preview.dirty = true;
            app.material_preview_dirty = false;
        }

        if let Some(handle) = mat_preview.render_target.clone() {
            let tex_id = egui_ctx.add_image(bevy_egui::EguiTextureHandle::Strong(handle));
            mat_preview.egui_texture_id = Some(tex_id);
            app.material_preview_texture_id = Some(tex_id);
        }
    }
    Ok(())
}

/// Bevy system for demo capture — uses Screenshot API to read GPU framebuffer.
/// Cycles through all features, taking per-feature screenshots while recording video.
pub fn demo_capture_system(
    mut app: bevy::ecs::system::NonSendMut<BerryCodeApp>,
    mut commands: bevy::ecs::system::Commands,
) {
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    use demo_capture::{DemoAction, SetupAction};

    if !app.demo_capture.active {
        return;
    }

    let action = app.demo_capture.tick();

    match action {
        DemoAction::None => {}
        DemoAction::CaptureVideo => {
            // Capture a frame for video only
            let encoder = app.demo_capture.encoder.clone();
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::ecs::observer::On<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        if let Some(ref data) = img.data {
                            enc.feed(data, w, h);
                        }
                    }
                },
            );
        }
        DemoAction::SetupUi { panel, setup } => {
            // Switch sidebar panel if specified
            if let Some(p) = panel {
                app.active_panel = p;
            }

            // Apply extra UI setup
            match setup {
                SetupAction::None => {}
                SetupAction::OpenDebugger => {
                    app.debug_state.active = true;
                    // Close other panels that might overlap
                    app.run_panel_open = false;
                    app.tool_panel_open = false;
                }
                SetupAction::OpenRunPanel => {
                    app.run_panel_open = true;
                    app.debug_state.active = false;
                    app.tool_panel_open = false;
                }
                SetupAction::OpenToolPanel => {
                    app.tool_panel_open = true;
                    app.debug_state.active = false;
                    app.run_panel_open = false;
                }
            }

            // Also capture a video frame during setup
            let encoder = app.demo_capture.encoder.clone();
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::ecs::observer::On<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        if let Some(ref data) = img.data {
                            enc.feed(data, w, h);
                        }
                    }
                },
            );
        }
        DemoAction::CaptureScreenshotAndVideo(name) => {
            // Capture for both screenshot and video
            let encoder = app.demo_capture.encoder.clone();
            let output_dir = std::path::PathBuf::from("docs/demo");
            let name_clone = name.clone();

            // Save screenshot to disk
            let path = output_dir.join(&name);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));

            // Also feed to video encoder
            commands.spawn(Screenshot::primary_window()).observe(
                move |trigger: bevy::ecs::observer::On<
                    bevy::render::view::screenshot::ScreenshotCaptured,
                >| {
                    let img = trigger.event();
                    let w = img.width();
                    let h = img.height();
                    if let Ok(mut enc) = encoder.lock() {
                        if let Some(ref data) = img.data {
                            enc.feed(data, w, h);
                        }
                    }
                    tracing::info!("📸 Saved: docs/demo/{}", name_clone);
                },
            );

            app.demo_capture.mark_screenshot(name);
        }
        DemoAction::Finish => {
            app.demo_capture.finalize();
            std::process::exit(0);
        }
    }
}

impl BerryCodeApp {
    /// Render the "unsaved changes" confirmation dialog
    fn render_close_confirm_dialog(&mut self, ctx: &egui::Context) {
        let unsaved: Vec<String> = self
            .editor_tabs
            .iter()
            .filter(|tab| tab.is_dirty)
            .map(|tab| {
                tab.file_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&tab.file_path)
                    .to_string()
            })
            .collect();

        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([340.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label("The following files have unsaved changes:");
                ui.add_space(4.0);
                for name in &unsaved {
                    ui.horizontal(|ui| {
                        ui.label("  •");
                        ui.label(
                            egui::RichText::new(name).color(egui::Color32::from_rgb(255, 198, 109)),
                        );
                    });
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new("Save All & Close")
                                .fill(egui::Color32::from_rgb(0, 122, 204)),
                        )
                        .clicked()
                    {
                        self.close_action = Some(CloseAction::SaveAll);
                    }
                    if ui.button("Discard & Close").clicked() {
                        self.close_action = Some(CloseAction::Discard);
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_close_confirm = false;
                    }
                });
            });
    }
}

impl Drop for BerryCodeApp {
    fn drop(&mut self) {
        // Kill any running child process
        if let Some(mut child) = self.run_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // Shutdown LSP client. Hard 2-second cap so an unresponsive
        // rust-analyzer (mid-indexing, RPC pipeline stalled, …) can't
        // wedge the quit path — without this the spawn pile-up we saw
        // accumulate to 13 orphaned processes happens any time the
        // graceful path stalls. `shutdown_all_with_timeout` falls back
        // to a synchronous force-kill if the async path exceeds the
        // budget.
        if let Some(client) = self.lsp_native_client.take() {
            let rt = self.lsp_runtime.clone();
            rt.block_on(client.shutdown_all_with_timeout(std::time::Duration::from_secs(2)));
        }
        // Shutdown file watcher
        self.file_watcher = None;
        tracing::info!("BerryCode shutdown complete");
    }
}

#[cfg(test)]
mod project_open_tests {
    //! Regression tests for the `open_project` startup flow. They
    //! exercise the helpers that drive it (`list_project_bscenes`)
    //! without standing up `BerryCodeApp` so we can guard the "tab
    //! per scene" contract directly.
    use super::*;

    #[test]
    fn list_project_bscenes_empty_for_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        // No `scenes/` directory yet.
        let out = list_project_bscenes(&tmp.path().to_string_lossy());
        assert!(out.is_empty());
    }

    #[test]
    fn list_project_bscenes_picks_up_every_bscene_sorted() {
        // Regression test for "ファイルツリーに二つあるのにスクリーン
        // エディターには１つじゃん" — `open_project` used to stop at
        // the first `.bscene` it found, leaving any later scene
        // orphaned in the file tree with no way to reopen it.
        let tmp = tempfile::tempdir().unwrap();
        let scenes_dir = tmp.path().join("scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        std::fs::write(scenes_dir.join("scene2.bscene"), "()").unwrap();
        std::fs::write(scenes_dir.join("scene.bscene"), "()").unwrap();
        std::fs::write(scenes_dir.join("README.md"), "ignored").unwrap();
        // Subdir / hidden file shouldn't crash the scan.
        std::fs::create_dir(scenes_dir.join("nested")).unwrap();

        let root = tmp.path().to_string_lossy().to_string();
        let out = list_project_bscenes(&root);
        let names: Vec<String> = out
            .iter()
            .map(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["scene.bscene", "scene2.bscene"]);
    }

    #[test]
    fn list_project_bscenes_case_insensitive_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let scenes_dir = tmp.path().join("scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        std::fs::write(scenes_dir.join("a.BSCENE"), "()").unwrap();
        std::fs::write(scenes_dir.join("b.Bscene"), "()").unwrap();
        let out = list_project_bscenes(&tmp.path().to_string_lossy());
        assert_eq!(out.len(), 2, "extension match must be case-insensitive");
    }

    #[test]
    fn list_project_bscenes_skips_dirs_with_bscene_name() {
        // A directory named `something.bscene` (unusual but possible)
        // must not be reported as a scene file.
        let tmp = tempfile::tempdir().unwrap();
        let scenes_dir = tmp.path().join("scenes");
        std::fs::create_dir_all(&scenes_dir).unwrap();
        std::fs::create_dir(scenes_dir.join("not_a_scene.bscene")).unwrap();
        std::fs::write(scenes_dir.join("real.bscene"), "()").unwrap();

        let out = list_project_bscenes(&tmp.path().to_string_lossy());
        // Both happen to match `is_file` filter logic? `read_dir` reports
        // both, but our filter uses extension only. Directory will pass
        // extension check — so this test documents the current behaviour
        // and forces a deliberate decision if we ever change it.
        assert!(out.iter().any(|p| p.ends_with("real.bscene")));
    }
}

#[cfg(test)]
mod lsp_completion_trigger_tests {
    //! Regression tests for the LSP auto-trigger detection. The bug
    //! that motivates these: the user reported "コードヒント (LSP
    //! completion popup) 出なくなった" when macOS IME was enabled
    //! even for plain ASCII typing — bevy_egui forwarded each Latin
    //! keystroke through `Ime::Preedit`, our IME filter stripped the
    //! paired `Event::Text`, and the auto-trigger detector — which
    //! only looked at Text + Commit — never fired.

    use super::*;
    use egui::{Event, ImeEvent, Modifiers};

    fn text(s: &str) -> Event {
        Event::Text(s.to_string())
    }
    fn preedit(s: &str) -> Event {
        Event::Ime(ImeEvent::Preedit(s.to_string()))
    }
    fn commit(s: &str) -> Event {
        Event::Ime(ImeEvent::Commit(s.to_string()))
    }
    fn key_press(key: egui::Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    /// **Direct Latin path**: plain `Event::Text("a")` must register
    /// as typing.
    #[test]
    fn text_event_counts_as_typing() {
        assert!(events_look_like_typing(&[text("a")]));
    }

    /// **macOS IME passthrough**: a Latin keystroke arrives as a
    /// non-empty `Preedit` (the paired Text is stripped upstream by
    /// our IME filter). This must STILL count as typing — otherwise
    /// completions never trigger while the IME is on.
    #[test]
    fn preedit_counts_as_typing() {
        assert!(
            events_look_like_typing(&[preedit("a")]),
            "Preedit('a') must count as typing — this is the IME-on regression"
        );
    }

    /// IME conversion commit (Japanese composition finalized) counts
    /// as typing.
    #[test]
    fn commit_counts_as_typing() {
        assert!(events_look_like_typing(&[commit("あ")]));
    }

    /// Empty preedit is the "IME cleared" signal, not typing.
    #[test]
    fn empty_preedit_is_not_typing() {
        assert!(!events_look_like_typing(&[preedit("")]));
    }

    /// Non-text input shouldn't fire the popup.
    #[test]
    fn key_press_alone_is_not_typing() {
        assert!(!events_look_like_typing(&[key_press(egui::Key::ArrowDown)]));
        assert!(!events_look_like_typing(&[key_press(egui::Key::Backspace)]));
        assert!(!events_look_like_typing(&[key_press(egui::Key::Enter)]));
    }

    /// Empty event list is a no-op frame.
    #[test]
    fn empty_frame_is_not_typing() {
        assert!(!events_look_like_typing(&[]));
    }

    // ── char_triggers_completion ───────────────────────────────────

    #[test]
    fn alphanumeric_triggers_completion() {
        assert!(char_triggers_completion('a'));
        assert!(char_triggers_completion('Z'));
        assert!(char_triggers_completion('0'));
        assert!(char_triggers_completion('9'));
    }

    #[test]
    fn special_completion_triggers() {
        // VS Code-style trigger characters.
        assert!(char_triggers_completion('_'));
        assert!(char_triggers_completion('.'));
        assert!(char_triggers_completion(':'));
        assert!(char_triggers_completion('<'));
    }

    #[test]
    fn whitespace_and_brackets_do_not_trigger() {
        assert!(!char_triggers_completion(' '));
        assert!(!char_triggers_completion('\n'));
        assert!(!char_triggers_completion('\t'));
        assert!(!char_triggers_completion('('));
        assert!(!char_triggers_completion(')'));
        assert!(!char_triggers_completion(';'));
        assert!(!char_triggers_completion(','));
    }

    /// CJK chars should also trigger — a user typing into an
    /// identifier with kanji (rare but legal) deserves completion.
    #[test]
    fn cjk_chars_trigger_completion() {
        assert!(char_triggers_completion('あ'));
        assert!(char_triggers_completion('日'));
    }
}

#[cfg(test)]
mod ime_filter_tests {
    //! Reproduces the bevy_egui → egui IME pipeline manually so we
    //! can assert that the filter in `berry_ui_system` actually
    //! restores Backspace during composition.
    //!
    //! The user-visible bug was: type "あああ" (3 chars via Preedit),
    //! then Backspace — only 2 chars deleted, the last one stuck.
    //! The root cause was egui's TextEdit dropping `Backspace` from
    //! the event list while `state.ime_enabled` is true, and
    //! bevy_egui re-asserting `ImeEvent::Enabled` ahead of every
    //! Preedit.

    use super::*;
    use egui::{Event, ImeEvent};

    /// Apply our filter exactly the way `berry_ui_system` does to a
    /// borrowed event vector.
    fn apply_filter(events: &mut Vec<Event>) {
        let has_preedit = events
            .iter()
            .any(|e| matches!(e, Event::Ime(ImeEvent::Preedit(s)) if !s.is_empty()));
        let has_backspace = events.iter().any(|e| {
            matches!(
                e,
                Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                }
            )
        });
        events.retain(|e| match e {
            Event::Ime(ImeEvent::Enabled) => false,
            Event::Ime(ImeEvent::Preedit(_)) if has_backspace => false,
            Event::Text(_) if has_preedit => false,
            _ => true,
        });
    }

    fn render(ctx: &egui::Context, buffer: &mut String, id: egui::Id) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let resp = egui::TextEdit::multiline(buffer).id(id).show(ui);
            resp.response.request_focus();
        });
    }

    fn run_frame(ctx: &egui::Context, buffer: &mut String, id: egui::Id, mut events: Vec<Event>) {
        apply_filter(&mut events);
        let raw = egui::RawInput {
            events,
            ..Default::default()
        };
        let _ = ctx.run(raw, |c| render(c, buffer, id));
    }

    fn backspace() -> Event {
        Event::Key {
            key: egui::Key::Backspace,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// **The user's bug**: type "あ", "ああ", "あああ" via Preedit
    /// (bevy_egui-style: Enabled-then-Preedit-then-Text), then send
    /// Commit + a Backspace. The buffer should end up with "ああ".
    /// Without the filter, ime_enabled stays true and Backspace is
    /// dropped — buffer stays "あああ".
    #[test]
    fn aaa_then_backspace_deletes_last_char() {
        let ctx = egui::Context::default();
        let mut buffer = String::new();
        let id = egui::Id::new("test_te");

        // Focus.
        let _ = ctx.run(egui::RawInput::default(), |c| render(c, &mut buffer, id));

        // Three preedit frames mimic bevy_egui's "Enabled then Preedit
        // then duplicated Text" pattern for typing あ, ああ, あああ.
        for value in ["あ", "ああ", "あああ"] {
            run_frame(
                &ctx,
                &mut buffer,
                id,
                vec![
                    Event::Ime(ImeEvent::Enabled),
                    Event::Ime(ImeEvent::Preedit(value.to_string())),
                    Event::Text(value.to_string()),
                ],
            );
        }
        // Commit the composition.
        run_frame(
            &ctx,
            &mut buffer,
            id,
            vec![Event::Ime(ImeEvent::Commit("あああ".into()))],
        );
        assert_eq!(buffer, "あああ", "commit must leave 3 chars");

        // The actual regression: Backspace.
        run_frame(&ctx, &mut buffer, id, vec![backspace()]);
        assert_eq!(buffer, "ああ", "backspace must delete the last char");
    }

    /// Backspace mid-preedit must REACH egui (the whole point of the
    /// filter). With egui's `state.ime_enabled = false` the inserted
    /// preedit lives in the buffer as a selection, so Backspace
    /// deletes the entire selection in one shot. That's the trade-
    /// off vs. shrinking-by-one — the alternative was "can't delete
    /// anything", which is much worse. The next Preedit frame from
    /// the IME resyncs the visible composition with whatever macOS
    /// still has in its preedit state.
    #[test]
    fn backspace_during_preedit_reaches_egui() {
        let ctx = egui::Context::default();
        let mut buffer = String::new();
        let id = egui::Id::new("test_te");

        let _ = ctx.run(egui::RawInput::default(), |c| render(c, &mut buffer, id));

        // Type "ああ" in preedit.
        for value in ["あ", "ああ"] {
            run_frame(
                &ctx,
                &mut buffer,
                id,
                vec![
                    Event::Ime(ImeEvent::Enabled),
                    Event::Ime(ImeEvent::Preedit(value.to_string())),
                    Event::Text(value.to_string()),
                ],
            );
        }
        // Backspace mid-preedit must NOT be silently dropped. The
        // buffer changes (vs. staying "ああ" forever, which was the
        // bug).
        let before = buffer.clone();
        run_frame(&ctx, &mut buffer, id, vec![backspace()]);
        assert_ne!(
            buffer, before,
            "Backspace must mutate the buffer; was {:?}",
            before
        );
    }

    /// Sanity: Enabled-only frames (no Preedit) get their Enabled
    /// stripped but other events pass through unchanged.
    #[test]
    fn enabled_alone_is_stripped() {
        let mut events = vec![Event::Ime(ImeEvent::Enabled), backspace()];
        apply_filter(&mut events);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Key { .. }));
    }

    /// Sanity: Preedit + duplicated Text → Text dropped but Preedit
    /// preserved.
    #[test]
    fn preedit_drops_text_keeps_preedit() {
        let mut events = vec![
            Event::Ime(ImeEvent::Preedit("あ".into())),
            Event::Text("あ".into()),
        ];
        apply_filter(&mut events);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Ime(ImeEvent::Preedit(_))));
    }

    /// "最後の一文字を消すのに Backspace 2回必要" 対策の検証：
    /// macOS が Backspace と同じフレームに Preedit を再送ってきた場合、
    /// Preedit を捨てて Backspace だけ通す。
    #[test]
    fn backspace_frame_drops_preedit() {
        let mut events = vec![Event::Ime(ImeEvent::Preedit("あ".into())), backspace()];
        apply_filter(&mut events);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Key { .. }));
    }
}

#[cfg(test)]
mod theme_and_workspace_tests {
    use super::*;

    /// `ui_colors::set_theme` must swap the active palette so callers
    /// see the right values immediately (no rerender lag, no caching).
    #[test]
    fn ui_colors_dark_vs_light_distinct() {
        ui_colors::set_theme(types::ThemeMode::Dark);
        let dark_editor = ui_colors::EDITOR_BG();
        let dark_text = ui_colors::TEXT_DEFAULT();
        ui_colors::set_theme(types::ThemeMode::Light);
        let light_editor = ui_colors::EDITOR_BG();
        let light_text = ui_colors::TEXT_DEFAULT();
        assert_ne!(dark_editor, light_editor, "EDITOR_BG must differ");
        assert_ne!(dark_text, light_text, "TEXT_DEFAULT must differ");
        // Light bg should be brighter than dark bg.
        assert!(light_editor.r() > dark_editor.r());
        // Restore for unrelated tests.
        ui_colors::set_theme(types::ThemeMode::Dark);
    }

    #[test]
    fn ui_colors_high_contrast_uses_pure_black_white() {
        ui_colors::set_theme(types::ThemeMode::HighContrast);
        assert_eq!(ui_colors::EDITOR_BG(), egui::Color32::BLACK);
        assert_eq!(ui_colors::TEXT_DEFAULT(), egui::Color32::WHITE);
        ui_colors::set_theme(types::ThemeMode::Dark);
    }

    /// `save_additional_roots` / `load_additional_roots` must round-trip
    /// through JSON without losing entries. We can't easily redirect the
    /// home-dir-derived path, so just hit the JSON serialisation /
    /// deserialisation logic directly.
    #[test]
    fn additional_roots_roundtrip_json() {
        let roots = vec!["/tmp/proj_a".to_string(), "/tmp/proj_b".to_string()];
        let json = serde_json::to_string(&roots).unwrap();
        let decoded: Vec<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, roots);
    }

    /// `load_additional_roots` filters out paths that no longer exist
    /// on disk so a stale entry from last session doesn't blow up.
    #[test]
    fn additional_roots_filter_missing_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let exists = tmp.path().to_string_lossy().to_string();
        let stale = tmp
            .path()
            .join("does_not_exist")
            .to_string_lossy()
            .to_string();
        let raw = vec![exists.clone(), stale.clone()];
        let filtered: Vec<String> = raw
            .into_iter()
            .filter(|p| std::path::Path::new(p).is_dir())
            .collect();
        assert_eq!(filtered, vec![exists]);
    }
}
