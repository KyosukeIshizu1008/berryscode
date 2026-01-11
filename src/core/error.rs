//! Shared Error Types for Frontend and Backend
//!
//! This module defines error types that can be serialized/deserialized
//! across the Tauri bridge, providing detailed error information to the UI.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Application error type that can be serialized across Tauri bridge
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppError {
    /// File system errors (reading, writing, permissions)
    FileSystem {
        operation: String,
        path: String,
        message: String,
    },

    /// LSP server errors (initialization, requests, responses)
    LspServer {
        operation: String,
        message: String,
    },

    /// Git operation errors
    Git {
        command: String,
        message: String,
    },

    /// Parsing errors (JSON, TOML, source code)
    Parse {
        format: String,
        message: String,
    },

    /// Network/IPC errors
    Network {
        operation: String,
        message: String,
    },

    /// Configuration errors
    Config {
        key: String,
        message: String,
    },

    /// Generic errors with context
    Generic {
        context: String,
        message: String,
    },
}

impl AppError {
    /// Create a file system error
    pub fn file_system(operation: impl Into<String>, path: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::FileSystem {
            operation: operation.into(),
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create an LSP error
    pub fn lsp(operation: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::LspServer {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Create a Git error
    pub fn git(command: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::Git {
            command: command.into(),
            message: message.into(),
        }
    }

    /// Create a parse error
    pub fn parse(format: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::Parse {
            format: format.into(),
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network(operation: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::Network {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Create a config error
    pub fn config(key: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::Config {
            key: key.into(),
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic(context: impl Into<String>, message: impl Into<String>) -> Self {
        AppError::Generic {
            context: context.into(),
            message: message.into(),
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            AppError::FileSystem { operation, path, message } => {
                format!("Failed to {} file '{}': {}", operation, path, message)
            }
            AppError::LspServer { operation, message } => {
                format!("LSP error during {}: {}", operation, message)
            }
            AppError::Git { command, message } => {
                format!("Git command '{}' failed: {}", command, message)
            }
            AppError::Parse { format, message } => {
                format!("Failed to parse {} format: {}", format, message)
            }
            AppError::Network { operation, message } => {
                format!("Network error during {}: {}", operation, message)
            }
            AppError::Config { key, message } => {
                format!("Configuration error for '{}': {}", key, message)
            }
            AppError::Generic { context, message } => {
                format!("{}: {}", context, message)
            }
        }
    }

    /// Get error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            AppError::FileSystem { .. } => "file_system",
            AppError::LspServer { .. } => "lsp_server",
            AppError::Git { .. } => "git",
            AppError::Parse { .. } => "parse",
            AppError::Network { .. } => "network",
            AppError::Config { .. } => "config",
            AppError::Generic { .. } => "generic",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AppError {}

/// Convert from anyhow::Error to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::generic("Unexpected error", err.to_string())
    }
}

/// Convert from std::io::Error to AppError
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::file_system("I/O operation", "unknown", err.to_string())
    }
}

/// Result type using AppError
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = AppError::file_system("read", "/path/to/file.txt", "Permission denied");
        assert_eq!(err.category(), "file_system");
        assert!(err.user_message().contains("Permission denied"));
    }

    #[test]
    fn test_lsp_error() {
        let err = AppError::lsp("initialize", "Server crashed");
        assert_eq!(err.category(), "lsp_server");
        assert!(err.user_message().contains("Server crashed"));
    }

    #[test]
    fn test_serialization() {
        let err = AppError::git("commit", "Nothing to commit");
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: AppError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }
}
