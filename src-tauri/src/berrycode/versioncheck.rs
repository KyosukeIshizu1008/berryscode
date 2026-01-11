//! Version checking and upgrade utilities

use crate::berrycode::Result;
use anyhow::anyhow;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::process::Command;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_API_URL: &str = "https://api.github.com/repos/paul-gauthier/aider/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

pub fn check_version() -> Result<Option<String>> {
    // Check if a newer version is available
    let client = Client::builder()
        .user_agent("aider-rust")
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let response = client.get(GITHUB_API_URL).send();

    let release: GithubRelease = match response {
        Ok(resp) => {
            if !resp.status().is_success() {
                // Silent fail - version check is not critical
                return Ok(None);
            }
            resp.json()?
        }
        Err(_) => {
            // Silent fail on network errors
            return Ok(None);
        }
    };

    let latest_version = release.tag_name.trim_start_matches('v');

    if is_newer_version(latest_version, CURRENT_VERSION) {
        Ok(Some(format!(
            "New version available: {} (current: {})\nVisit: {}\n\nChangelog:\n{}",
            latest_version,
            CURRENT_VERSION,
            release.html_url,
            release.body.unwrap_or_else(|| "No changelog available".to_string())
        )))
    } else {
        Ok(None)
    }
}

pub fn install_upgrade() -> Result<()> {
    // Upgrade to the latest release using cargo
    println!("Upgrading aider-rust to the latest version...");

    let status = Command::new("cargo")
        .args(&["install", "--git", "https://github.com/paul-gauthier/aider", "--force"])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to upgrade. Please try manually with: cargo install --git https://github.com/paul-gauthier/aider --force"));
    }

    println!("✓ Successfully upgraded to the latest version!");
    println!("Please restart aider to use the new version.");

    Ok(())
}

pub fn install_from_main_branch() -> Result<()> {
    // Install from the main branch (development version)
    println!("Installing aider-rust from the main branch...");
    println!("⚠ Warning: This is the development version and may be unstable.");

    let status = Command::new("cargo")
        .args(&[
            "install",
            "--git",
            "https://github.com/paul-gauthier/aider",
            "--branch",
            "main",
            "--force",
        ])
        .status()?;

    if !status.success() {
        return Err(anyhow!("Failed to install from main branch"));
    }

    println!("✓ Successfully installed from main branch!");
    println!("Please restart aider to use the new version.");

    Ok(())
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    // Simple version comparison (assumes semantic versioning)
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    for i in 0..latest_parts.len().min(current_parts.len()) {
        if latest_parts[i] > current_parts[i] {
            return true;
        } else if latest_parts[i] < current_parts[i] {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(is_newer_version("1.1.0", "1.0.9"));
        assert!(is_newer_version("2.0.0", "1.9.9"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
    }
}
