//! Bevy version detection and management.

/// Detect the Bevy version from the project's Cargo.toml
pub fn detect_bevy_version(root: &str) -> Option<String> {
    let cargo_path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&cargo_path).ok()?;

    for line in content.lines() {
        let trimmed = line.trim();
        // Match: bevy = "0.15" or bevy = { version = "0.15", ... }
        if trimmed.starts_with("bevy") && trimmed.contains('=') {
            if let Some(ver_start) = trimmed.find('"') {
                let after = &trimmed[ver_start + 1..];
                if let Some(ver_end) = after.find('"') {
                    return Some(after[..ver_end].to_string());
                }
            }
        }
    }
    None
}

/// Check for newer Bevy versions on crates.io
#[allow(dead_code)]
pub fn check_latest_bevy_version() -> Option<String> {
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-H",
            "User-Agent: BerryCode-Editor",
            "https://crates.io/api/v1/crates/bevy",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json.get("crate")
        .and_then(|c| c.get("newest_version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Update Bevy version in Cargo.toml
#[allow(dead_code)]
pub fn update_bevy_version(root: &str, new_version: &str) -> Result<(), String> {
    let cargo_path = format!("{}/Cargo.toml", root);
    let content = std::fs::read_to_string(&cargo_path).map_err(|e| e.to_string())?;

    let mut new_content = String::new();
    for line in content.lines() {
        if line.trim().starts_with("bevy") && line.contains("version") {
            // Replace version in: bevy = { version = "0.15", ... }
            let new_line = regex::Regex::new(r#"version\s*=\s*"[^"]*""#)
                .map_err(|e| e.to_string())?
                .replace(line, &format!("version = \"{}\"", new_version));
            new_content.push_str(&new_line);
        } else if line.trim().starts_with("bevy")
            && line.contains('"')
            && !line.contains("version")
        {
            // Replace version in: bevy = "0.15"
            let new_line = regex::Regex::new(r#""[^"]*""#)
                .map_err(|e| e.to_string())?
                .replace(line, &format!("\"{}\"", new_version));
            new_content.push_str(&new_line);
        } else {
            new_content.push_str(line);
        }
        new_content.push('\n');
    }

    std::fs::write(&cargo_path, new_content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_version_simple() {
        let tmp = tempfile::tempdir().unwrap();
        let cargo = tmp.path().join("Cargo.toml");
        std::fs::write(&cargo, "[dependencies]\nbevy = \"0.15\"\n").unwrap();
        let ver = detect_bevy_version(tmp.path().to_str().unwrap());
        assert_eq!(ver, Some("0.15".to_string()));
    }

    #[test]
    fn detect_version_table() {
        let tmp = tempfile::tempdir().unwrap();
        let cargo = tmp.path().join("Cargo.toml");
        std::fs::write(
            &cargo,
            "[dependencies]\nbevy = { version = \"0.15\", features = [\"x\"] }\n",
        )
        .unwrap();
        let ver = detect_bevy_version(tmp.path().to_str().unwrap());
        assert_eq!(ver, Some("0.15".to_string()));
    }
}
