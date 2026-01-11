//! Custom exception types for aider

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiderError {
    #[error("Git error: {0}")]
    GitError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Model error: {0}")]
    ModelError(String),

    #[error("Unknown edit format: {0}")]
    UnknownEditFormat(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Interrupted by user")]
    UserInterrupt,
}

pub type Result<T> = std::result::Result<T, AiderError>;
