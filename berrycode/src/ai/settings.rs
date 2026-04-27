//! Persisted BYOK (bring-your-own-key) settings for the AI providers.
//!
//! Stored at `~/.berrycode/ai.json`. Keys are kept on disk in plaintext
//! for now — a future revision can switch to the OS keyring (Keychain
//! on macOS, libsecret on Linux, Credential Manager on Windows) without
//! changing the call sites.

use super::ProviderKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSettings {
    pub anthropic_api_key: String,
    pub openai_api_key: String,
    pub ollama_endpoint: String,

    /// Model used for the chat sidebar.
    pub chat_provider: ProviderKind,
    pub chat_model: String,

    /// Model used for inline / Tab completion (Phase 2). Often a
    /// smaller / faster model than the chat one.
    pub completion_provider: ProviderKind,
    pub completion_model: String,

    /// Whether AI features are enabled at all. Lets users keep keys on
    /// disk but disable the assistant temporarily.
    pub enabled: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            anthropic_api_key: String::new(),
            openai_api_key: String::new(),
            ollama_endpoint: "http://localhost:11434".to_string(),
            chat_provider: ProviderKind::Anthropic,
            chat_model: "claude-sonnet-4-6".to_string(),
            completion_provider: ProviderKind::Anthropic,
            completion_model: "claude-haiku-4-5".to_string(),
            enabled: true,
        }
    }
}

impl AiSettings {
    fn path() -> std::path::PathBuf {
        if let Some(home) = dirs::home_dir() {
            let dir = home.join(".berrycode");
            std::fs::create_dir_all(&dir).ok();
            dir.join("ai.json")
        } else {
            std::path::PathBuf::from("ai.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Available chat-tier models per provider, exposed in the Settings
    /// dropdown. Names match the provider's API exactly.
    pub fn chat_models_for(provider: ProviderKind) -> &'static [&'static str] {
        match provider {
            ProviderKind::Anthropic => &[
                "claude-opus-4-7",
                "claude-sonnet-4-6",
                "claude-haiku-4-5-20251001",
            ],
            ProviderKind::OpenAi => &["gpt-5", "gpt-5-codex", "gpt-4.1"],
            ProviderKind::Ollama => &["llama3.1", "qwen2.5-coder", "deepseek-coder-v2"],
        }
    }

    /// Models recommended for inline / Tab completion — smaller / faster.
    pub fn completion_models_for(provider: ProviderKind) -> &'static [&'static str] {
        match provider {
            ProviderKind::Anthropic => &["claude-haiku-4-5-20251001", "claude-sonnet-4-6"],
            ProviderKind::OpenAi => &["gpt-5-codex", "gpt-4.1-mini"],
            ProviderKind::Ollama => &["qwen2.5-coder", "starcoder2"],
        }
    }
}
