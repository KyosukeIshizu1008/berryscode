//! BerryFlow Configuration
//!
//! プロジェクトごとの設定を .berryflow.json で管理

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// BerryFlow設定ファイル (.berryflow.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BerryFlowConfig {
    /// プロジェクト名
    #[serde(default)]
    pub project_name: String,

    /// カスタムテストコマンド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_command: Option<String>,

    /// カスタムビルドコマンド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,

    /// 環境変数
    #[serde(default)]
    pub environment_variables: HashMap<String, String>,

    /// 除外ファイルパターン
    #[serde(default)]
    pub excluded_files: Vec<String>,

    /// カスタムノード定義
    #[serde(default)]
    pub custom_nodes: Vec<CustomNodeDef>,

    /// 通知設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationConfig>,
}

/// カスタムノード定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomNodeDef {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<String>,
}

/// 通知設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slack_webhook: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailConfig>,
}

/// メール設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub from: String,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl Default for BerryFlowConfig {
    fn default() -> Self {
        Self {
            project_name: String::new(),
            test_command: None,
            build_command: None,
            environment_variables: HashMap::new(),
            excluded_files: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                ".git/**".to_string(),
            ],
            custom_nodes: Vec::new(),
            notifications: None,
        }
    }
}

impl BerryFlowConfig {
    /// 設定ファイルを読み込む
    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(".berryflow.json");

        if !config_path.exists() {
            // デフォルト設定を返す
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: BerryFlowConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// 設定ファイルを保存
    pub fn save(&self, project_root: &Path) -> Result<()> {
        let config_path = project_root.join(".berryflow.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// テストコマンドを取得（設定またはデフォルト）
    pub fn get_test_command(&self, project_root: &Path) -> String {
        if let Some(cmd) = &self.test_command {
            return cmd.clone();
        }

        // プロジェクトタイプに応じてデフォルトコマンドを返す
        if project_root.join("Cargo.toml").exists() {
            "cargo test --no-fail-fast".to_string()
        } else if project_root.join("package.json").exists() {
            "npm test".to_string()
        } else if project_root.join("pytest.ini").exists() || project_root.join("setup.py").exists() {
            "pytest".to_string()
        } else {
            "echo 'No test command configured'".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = BerryFlowConfig::default();
        assert!(config.test_command.is_none());
        assert!(config.excluded_files.contains(&"node_modules/**".to_string()));
    }

    #[test]
    fn test_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let project_root = temp_dir.path();

        let mut config = BerryFlowConfig::default();
        config.project_name = "test-project".to_string();
        config.test_command = Some("cargo test".to_string());

        config.save(project_root)?;

        let loaded = BerryFlowConfig::load(project_root)?;
        assert_eq!(loaded.project_name, "test-project");
        assert_eq!(loaded.test_command, Some("cargo test".to_string()));

        Ok(())
    }
}
