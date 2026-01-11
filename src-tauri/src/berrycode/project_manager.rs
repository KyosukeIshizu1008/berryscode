//! Project management with persistent history and Git integration
//!
//! This module manages BerryCode projects, tracking recently opened projects,
//! Git state, and project-specific settings in `.berrycode/` directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use crate::berrycode::Result;

/// Project manager with persistent history
pub struct ProjectManager {
    projects_file: PathBuf,
    projects: Vec<Project>,
}

/// Individual project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub last_opened: u64,
    pub git_status: GitStatus,
    pub tags: Vec<String>,
    #[serde(default)]
    pub workflow_count: usize,
}

/// Git repository status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    pub is_git_repo: bool,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub ahead_behind: Option<(usize, usize)>, // (ahead, behind)
    pub last_commit_msg: Option<String>,
    pub last_commit_hash: Option<String>,
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

        let berrycode_dir = home_dir.join(".berrycode");
        std::fs::create_dir_all(&berrycode_dir)?;

        let projects_file = berrycode_dir.join("projects.json");

        let mut manager = Self {
            projects_file,
            projects: Vec::new(),
        };

        manager.load()?;
        Ok(manager)
    }

    /// Load projects from disk
    fn load(&mut self) -> Result<()> {
        if !self.projects_file.exists() {
            return Ok(());
        }

        let data = std::fs::read_to_string(&self.projects_file)?;

        // Try to load projects - if it fails, backup old file and start fresh
        match serde_json::from_str::<Vec<Project>>(&data) {
            Ok(projects) => {
                self.projects = projects;
                tracing::info!("Loaded {} projects from {:?}", self.projects.len(), self.projects_file);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load projects file ({}), backing up and starting fresh",
                    e
                );
                // Backup old file
                let backup_path = self.projects_file.with_extension("json.backup");
                std::fs::rename(&self.projects_file, &backup_path)?;
                tracing::info!("Old projects file backed up to {:?}", backup_path);
                self.projects = Vec::new();
            }
        }

        Ok(())
    }

    /// Save projects to disk
    fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.projects)?;
        std::fs::write(&self.projects_file, data)?;

        tracing::debug!("Saved {} projects to {:?}", self.projects.len(), self.projects_file);
        Ok(())
    }

    /// Get all projects sorted by last opened (most recent first)
    pub fn list_projects(&self) -> Vec<Project> {
        let mut projects = self.projects.clone();
        projects.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
        projects
    }

    /// Add or update a project
    pub fn add_or_update_project(&mut self, path: PathBuf) -> Result<()> {
        let canonical_path = std::fs::canonicalize(&path)?;

        // Check if project already exists
        if let Some(existing) = self.projects.iter_mut().find(|p| p.path == canonical_path) {
            // Update last opened time
            existing.last_opened = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();

            // Update Git status
            existing.git_status = Self::get_git_status(&canonical_path)?;
        } else {
            // Add new project
            let name = canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let project = Project {
                name,
                path: canonical_path.clone(),
                last_opened: SystemTime::now()
                    .duration_since(UNIX_EPOCH)?
                    .as_secs(),
                git_status: Self::get_git_status(&canonical_path)?,
                tags: Vec::new(),
                workflow_count: 0,
            };

            self.projects.push(project);
        }

        self.save()?;
        Ok(())
    }

    /// Remove a project from history
    pub fn remove_project(&mut self, path: &Path) -> Result<()> {
        let canonical_path = std::fs::canonicalize(path)?;
        self.projects.retain(|p| p.path != canonical_path);
        self.save()?;
        Ok(())
    }

    /// Get Git status for a project
    pub fn get_git_status(path: &Path) -> Result<GitStatus> {
        use std::process::Command;

        // Check if it's a Git repository
        let is_git = path.join(".git").exists();

        if !is_git {
            return Ok(GitStatus {
                is_git_repo: false,
                ..Default::default()
            });
        }

        let mut status = GitStatus {
            is_git_repo: true,
            ..Default::default()
        };

        // Get current branch
        if let Ok(output) = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                status.branch = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Check if working directory is dirty
        if let Ok(output) = Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                status.is_dirty = !output.stdout.is_empty();
            }
        }

        // Get ahead/behind count
        if let Some(branch) = &status.branch {
            if let Ok(output) = Command::new("git")
                .args(&["rev-list", "--left-right", "--count", &format!("{}...@{{u}}", branch)])
                .current_dir(path)
                .output()
            {
                if output.status.success() {
                    let counts = String::from_utf8_lossy(&output.stdout);
                    let parts: Vec<&str> = counts.trim().split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(ahead), Ok(behind)) = (parts[0].parse(), parts[1].parse()) {
                            status.ahead_behind = Some((ahead, behind));
                        }
                    }
                }
            }
        }

        // Get last commit message
        if let Ok(output) = Command::new("git")
            .args(&["log", "-1", "--pretty=%s"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                status.last_commit_msg = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Get last commit hash
        if let Ok(output) = Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                status.last_commit_hash = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        Ok(status)
    }

    /// Initialize .berrycode directory for a project
    pub fn init_project_dir(path: &Path) -> Result<()> {
        let berrycode_dir = path.join(".berrycode");
        std::fs::create_dir_all(&berrycode_dir)?;

        // Create subdirectories
        std::fs::create_dir_all(berrycode_dir.join("workflows"))?;
        std::fs::create_dir_all(berrycode_dir.join("assets"))?;
        std::fs::create_dir_all(berrycode_dir.join("cache"))?;

        // Create config file
        let config_path = berrycode_dir.join("config.json");
        if !config_path.exists() {
            let config = ProjectConfig::default();
            let data = serde_json::to_string_pretty(&config)?;
            std::fs::write(config_path, data)?;
        }

        // Add .gitignore
        let gitignore_path = berrycode_dir.join(".gitignore");
        if !gitignore_path.exists() {
            std::fs::write(gitignore_path, "cache/\nrepomap.bin\n*.log\n")?;
        }

        tracing::info!("✅ Initialized .berrycode directory at {:?}", berrycode_dir);
        Ok(())
    }

    /// Get project configuration
    pub fn get_project_config(path: &Path) -> Result<ProjectConfig> {
        let config_path = path.join(".berrycode/config.json");

        if !config_path.exists() {
            return Ok(ProjectConfig::default());
        }

        let data = std::fs::read_to_string(config_path)?;
        let config = serde_json::from_str(&data)?;
        Ok(config)
    }

    /// Update project configuration
    pub fn update_project_config(path: &Path, config: &ProjectConfig) -> Result<()> {
        let config_path = path.join(".berrycode/config.json");
        let data = serde_json::to_string_pretty(config)?;
        std::fs::write(config_path, data)?;
        Ok(())
    }
}

/// Project-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub enable_prompt_caching: bool,
    pub asset_registry: HashMap<String, String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5@20250929".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            enable_prompt_caching: true,
            asset_registry: HashMap::new(),
        }
    }
}
