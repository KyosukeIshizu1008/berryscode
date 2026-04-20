//! Editor Settings Management
//!
//! This module manages all editor settings with file-based persistence.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    // Editor
    pub font_size: u32,
    pub font_family: String,
    pub line_height: u32,
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub word_wrap: bool,

    // Theme
    pub color_theme: String,
    pub icon_theme: String,

    // BerryCode AI
    pub ai_model: String,
    pub ai_mode: String,
    pub ai_enabled: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            // Editor defaults
            font_size: 13,
            font_family: "JetBrains Mono".to_string(),
            line_height: 20,
            tab_size: 4,
            insert_spaces: true,
            word_wrap: false,

            // Theme defaults
            color_theme: "darcula".to_string(),
            icon_theme: "Codicons".to_string(),

            // AI defaults
            ai_model: "Llama 4 Scout".to_string(),
            ai_mode: "code".to_string(),
            ai_enabled: true,
        }
    }
}

impl EditorSettings {
    #[allow(dead_code)]
    const STORAGE_KEY: &'static str = "berry-editor-settings";

    /// Get settings file path
    fn settings_path() -> std::path::PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let berry_config = config_dir.join("berrycode");
            std::fs::create_dir_all(&berry_config).ok();
            berry_config.join("settings.json")
        } else {
            std::path::PathBuf::from("settings.json")
        }
    }

    /// Load settings from file
    pub fn load() -> Self {
        let path = Self::settings_path();
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<EditorSettings>(&contents) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let path = Self::settings_path();
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Get available font families
    pub fn available_fonts() -> Vec<&'static str> {
        vec![
            "JetBrains Mono",
            "Fira Code",
            "Source Code Pro",
            "Monaco",
            "Consolas",
            "Courier New",
        ]
    }

    /// Get available color themes
    pub fn available_themes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("darcula", "IntelliJ Darcula (Default)"),
            ("light", "IntelliJ Light"),
            ("high-contrast", "High Contrast (WCAG AAA)"),
        ]
    }

    /// Get available AI models
    pub fn available_models() -> Vec<&'static str> {
        vec![
            "Llama 4 Scout",
            "gpt-4o",
            "gpt-4-turbo",
            "gpt-3.5-turbo",
            "claude-3-opus",
            "claude-3-sonnet",
        ]
    }

    /// Get available AI modes
    pub fn available_modes() -> Vec<&'static str> {
        vec!["code", "architect", "help", "ask"]
    }

    /// Apply theme to DOM by setting data-theme attribute on body (WASM only)
    pub fn apply_theme(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(body) = document.body() {
                        let _ = body.set_attribute("data-theme", &self.color_theme);
                    }
                }
            }
        }
    }
}
