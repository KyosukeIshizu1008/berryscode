//! LLM model definitions and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::berrycode::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub max_context_tokens: Option<usize>,
    pub max_output_tokens: Option<usize>,
    pub input_cost_per_token: Option<f64>,
    pub output_cost_per_token: Option<f64>,
    pub supports_functions: bool,
    pub supports_vision: bool,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub info: ModelInfo,
    pub edit_format: Option<String>,
    pub weak_model: Option<String>,
    pub editor_model: Option<String>,
    pub editor_edit_format: Option<String>,
    pub verbose: bool,
    pub accepts_settings: Option<Vec<String>>,
    pub reasoning_effort: Option<String>,
    pub thinking_tokens: Option<String>,
}

impl Model {
    pub fn new(
        name: String,
        weak_model: Option<String>,
        editor_model: Option<String>,
        editor_edit_format: Option<String>,
        verbose: bool,
    ) -> Result<Self> {
        let info = Self::get_model_info(&name)?;
        let edit_format = Self::default_edit_format(&name);

        Ok(Self {
            name: name.clone(),
            info,
            edit_format,
            weak_model,
            editor_model,
            editor_edit_format,
            verbose,
            accepts_settings: Some(vec![]),
            reasoning_effort: None,
            thinking_tokens: None,
        })
    }

    fn get_model_info(name: &str) -> Result<ModelInfo> {
        // Built-in model database
        let model_info = match name {
            // OpenAI GPT-4 models
            "gpt-4" | "gpt-4-0613" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(8192),
                max_output_tokens: Some(8192),
                input_cost_per_token: Some(0.00003),
                output_cost_per_token: Some(0.00006),
                supports_functions: true,
                supports_vision: false,
            },
            "gpt-4-turbo" | "gpt-4-turbo-preview" | "gpt-4-0125-preview" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(128000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.00001),
                output_cost_per_token: Some(0.00003),
                supports_functions: true,
                supports_vision: false,
            },
            "gpt-4-vision-preview" | "gpt-4-turbo-2024-04-09" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(128000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.00001),
                output_cost_per_token: Some(0.00003),
                supports_functions: true,
                supports_vision: true,
            },
            "gpt-4o" | "gpt-4o-2024-05-13" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(128000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.000005),
                output_cost_per_token: Some(0.000015),
                supports_functions: true,
                supports_vision: true,
            },
            "gpt-4o-mini" | "gpt-4o-mini-2024-07-18" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(128000),
                max_output_tokens: Some(16384),
                input_cost_per_token: Some(0.00000015),
                output_cost_per_token: Some(0.0000006),
                supports_functions: true,
                supports_vision: true,
            },

            // OpenAI GPT-3.5 models
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(16384),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.0000005),
                output_cost_per_token: Some(0.0000015),
                supports_functions: true,
                supports_vision: false,
            },

            // Anthropic Claude models
            "claude-3-opus-20240229" | "claude-3-opus" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(200000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.000015),
                output_cost_per_token: Some(0.000075),
                supports_functions: true,
                supports_vision: true,
            },
            "claude-3-sonnet-20240229" | "claude-3-sonnet" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(200000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.000003),
                output_cost_per_token: Some(0.000015),
                supports_functions: true,
                supports_vision: true,
            },
            "claude-3-haiku-20240307" | "claude-3-haiku" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(200000),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.00000025),
                output_cost_per_token: Some(0.00000125),
                supports_functions: true,
                supports_vision: true,
            },
            "claude-3-5-sonnet-20240620" | "claude-3.5-sonnet" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(200000),
                max_output_tokens: Some(8192),
                input_cost_per_token: Some(0.000003),
                output_cost_per_token: Some(0.000015),
                supports_functions: true,
                supports_vision: true,
            },

            // Llama 4 Scout (Ollama)
            "Llama 4 Scout" | "llama-4-scout" => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(32768),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.0), // Local model, no cost
                output_cost_per_token: Some(0.0), // Local model, no cost
                supports_functions: true,
                supports_vision: false,
            },

            // Default for unknown models
            _ => ModelInfo {
                name: name.to_string(),
                max_context_tokens: Some(8192),
                max_output_tokens: Some(4096),
                input_cost_per_token: Some(0.00001),
                output_cost_per_token: Some(0.00003),
                supports_functions: true,
                supports_vision: false,
            },
        };

        Ok(model_info)
    }

    fn default_edit_format(name: &str) -> Option<String> {
        // Default edit formats for known models
        if name.contains("claude") || name.contains("sonnet") || name.contains("opus") {
            Some("diff".to_string())
        } else if name.contains("gpt-4") {
            Some("diff".to_string())  // Changed from "udiff" to "diff" for better stability
        } else if name.contains("gpt-3.5") {
            Some("whole".to_string())
        } else {
            Some("diff".to_string())
        }
    }

    pub fn set_reasoning_effort(&mut self, effort: String) {
        self.reasoning_effort = Some(effort);
    }

    pub fn set_thinking_tokens(&mut self, tokens: String) {
        self.thinking_tokens = Some(tokens);
    }

    pub fn get_weak_model(&self) -> Option<&str> {
        self.weak_model.as_deref()
    }

    pub fn get_editor_model(&self) -> Option<&str> {
        self.editor_model.as_deref()
    }
}

pub struct ModelSettings {
    models: HashMap<String, ModelInfo>,
}

impl ModelSettings {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn load_from_file(&mut self, _path: &std::path::Path) -> Result<()> {
        // TODO: Load model settings from YAML file
        Ok(())
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }

    pub fn add_model(&mut self, info: ModelInfo) {
        self.models.insert(info.name.clone(), info);
    }
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if models are compatible
pub fn sanity_check_models(_io: &crate::berrycode::io::InputOutput, _model: &Model) -> Option<String> {
    // TODO: Implement model compatibility checks
    None
}

/// List available models
pub fn list_models(filter: Option<&str>) -> Vec<String> {
    let all_models = vec![
        "gpt-4".to_string(),
        "gpt-4-turbo".to_string(),
        "gpt-3.5-turbo".to_string(),
        "claude-3-opus".to_string(),
        "claude-3-sonnet".to_string(),
        "claude-3-haiku".to_string(),
    ];

    if let Some(filter) = filter {
        all_models
            .into_iter()
            .filter(|m| m.contains(filter))
            .collect()
    } else {
        all_models
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_creation() {
        let model = Model::new(
            "gpt-4".to_string(),
            None,
            None,
            None,
            false,
        );
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.name, "gpt-4");
        assert_eq!(model.info.max_context_tokens, Some(8192));
        assert!(model.info.supports_functions);
    }

    #[test]
    fn test_model_creation_claude() {
        let model = Model::new(
            "claude-3-opus".to_string(),
            None,
            None,
            None,
            false,
        );
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.name, "claude-3-opus");
        assert_eq!(model.info.max_context_tokens, Some(200000));
        assert!(model.info.supports_vision);
    }

    #[test]
    fn test_model_creation_unknown() {
        let model = Model::new(
            "unknown-model".to_string(),
            None,
            None,
            None,
            false,
        );
        // Should create a default model
        assert!(model.is_ok());
    }

    #[test]
    fn test_default_edit_format() {
        let claude_format = Model::default_edit_format("claude-3-opus");
        assert_eq!(claude_format, Some("diff".to_string()));

        let gpt4_format = Model::default_edit_format("gpt-4");
        assert_eq!(gpt4_format, Some("diff".to_string()));

        let gpt35_format = Model::default_edit_format("gpt-3.5-turbo");
        assert_eq!(gpt35_format, Some("whole".to_string()));

        let unknown_format = Model::default_edit_format("unknown");
        assert_eq!(unknown_format, Some("diff".to_string()));
    }

    /// Test that all Claude models route to diff format
    #[test]
    fn test_claude_models_use_diff() {
        assert_eq!(Model::default_edit_format("claude-3-opus"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format("claude-3-sonnet"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format("claude-3.5-sonnet"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format("sonnet-4"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format("opus-4"), Some("diff".to_string()));
    }

    /// Test that all GPT-4 models route to diff format (not udiff)
    #[test]
    fn test_gpt4_models_use_diff_not_udiff() {
        assert_eq!(Model::default_edit_format("gpt-4"), Some("diff".to_string()),
            "GPT-4 should use diff, not udiff");
        assert_eq!(Model::default_edit_format("gpt-4-turbo"), Some("diff".to_string()),
            "GPT-4 Turbo should use diff, not udiff");
        assert_eq!(Model::default_edit_format("gpt-4o"), Some("diff".to_string()),
            "GPT-4o should use diff, not udiff");
        assert_eq!(Model::default_edit_format("gpt-4-32k"), Some("diff".to_string()),
            "GPT-4-32k should use diff, not udiff");
    }

    /// Test that GPT-3.5 still uses whole format
    #[test]
    fn test_gpt35_uses_whole() {
        assert_eq!(Model::default_edit_format("gpt-3.5-turbo"), Some("whole".to_string()));
        assert_eq!(Model::default_edit_format("gpt-3.5-turbo-16k"), Some("whole".to_string()));
    }

    /// Test that unknown models default to diff
    #[test]
    fn test_unknown_models_default_to_diff() {
        assert_eq!(Model::default_edit_format("unknown-model"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format("random-llm"), Some("diff".to_string()));
        assert_eq!(Model::default_edit_format(""), Some("diff".to_string()));
    }

    /// Test that no model routes to udiff or editblock
    #[test]
    fn test_no_models_route_to_deprecated_formats() {
        // Test common model names
        let models = vec![
            "claude-3-opus", "claude-3-sonnet", "sonnet-4",
            "gpt-4", "gpt-4-turbo", "gpt-4o",
            "gpt-3.5-turbo",
            "deepseek-chat", "deepseek-coder",
            "unknown-model"
        ];

        for model in models {
            let format = Model::default_edit_format(model);
            assert_ne!(format, Some("udiff".to_string()),
                "Model {} should not route to udiff", model);
            assert_ne!(format, Some("editblock".to_string()),
                "Model {} should not route to editblock", model);
            assert_ne!(format, Some("editblock-fenced".to_string()),
                "Model {} should not route to editblock-fenced", model);
            assert_ne!(format, Some("editblock-func".to_string()),
                "Model {} should not route to editblock-func", model);
        }
    }

    #[test]
    fn test_model_settings() {
        let mut settings = ModelSettings::new();

        // Test adding a model
        let info = ModelInfo {
            name: "test-model".to_string(),
            max_context_tokens: Some(1000),
            max_output_tokens: Some(500),
            input_cost_per_token: Some(0.001),
            output_cost_per_token: Some(0.002),
            supports_functions: true,
            supports_vision: false,
        };

        settings.add_model(info);

        // Test retrieving the model
        let retrieved = settings.get_model("test-model");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.max_context_tokens, Some(1000));

        // Test non-existent model
        assert!(settings.get_model("nonexistent").is_none());
    }

    #[test]
    fn test_list_models() {
        let all_models = list_models(None);
        assert!(all_models.len() > 0);
        assert!(all_models.contains(&"gpt-4".to_string()));
        assert!(all_models.contains(&"claude-3-opus".to_string()));
    }

    #[test]
    fn test_list_models_filtered() {
        let gpt_models = list_models(Some("gpt"));
        assert!(gpt_models.iter().all(|m| m.contains("gpt")));

        let claude_models = list_models(Some("claude"));
        assert!(claude_models.iter().all(|m| m.contains("claude")));
    }
}
