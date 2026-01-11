//! Project type definitions and prompt templates

use serde::{Deserialize, Serialize};

/// Project type for scaffolding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    WebApp,
    CliTool,
    Library,
    ApiService,
    MobileApp,
}

impl ProjectType {
    /// Get the system prompt for this project type
    pub fn system_prompt(&self) -> &'static str {
        match self {
            ProjectType::WebApp => include_str!("prompts/web_app.txt"),
            ProjectType::CliTool => include_str!("prompts/cli_tool.txt"),
            ProjectType::Library => include_str!("prompts/library.txt"),
            ProjectType::ApiService => include_str!("prompts/api_service.txt"),
            ProjectType::MobileApp => include_str!("prompts/mobile_app.txt"),
        }
    }

    /// Convert string to ProjectType
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "web" | "webapp" => Some(ProjectType::WebApp),
            "cli" | "clitool" => Some(ProjectType::CliTool),
            "lib" | "library" => Some(ProjectType::Library),
            "api" | "apiservice" => Some(ProjectType::ApiService),
            "mobile" | "mobileapp" => Some(ProjectType::MobileApp),
            _ => None,
        }
    }
}

/// Scaffolding request
#[derive(Debug, Serialize, Deserialize)]
pub struct ScaffoldingRequest {
    pub description: String,
    pub project_name: String,
    pub project_type: ProjectType,
    pub language: Option<String>,
}

/// Scaffolding response from AI
#[derive(Debug, Serialize, Deserialize)]
pub struct ScaffoldingResponse {
    pub files: Vec<FileToCreate>,
    pub instructions: String,
}

/// File to create
#[derive(Debug, Serialize, Deserialize)]
pub struct FileToCreate {
    pub path: String,
    pub content: String,
}
