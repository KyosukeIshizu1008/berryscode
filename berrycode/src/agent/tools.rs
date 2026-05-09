//! Filesystem + shell tools shared by the in-process agent backends
//! ([`super::native`] for OpenAI Responses API, [`super::ollama`] for
//! local Ollama). The CLI-backed agents ([`super::claude`],
//! [`super::codex`]) bring their own toolchains so they don't use these.
//!
//! Path resolution applies a cheap traversal guard: anything that
//! resolves outside the working directory is rejected. The model can
//! still escape via `run_bash`, but the file tools shouldn't be a free
//! pass.

use std::path::{Path, PathBuf};

use tokio::sync::mpsc;

use super::AgentEvent;

const MAX_FILE_BYTES: usize = 200 * 1024;
const MAX_BASH_OUTPUT: usize = 16_384;

fn resolve(cwd: &Path, rel: &str) -> anyhow::Result<PathBuf> {
    let joined = cwd.join(rel);
    let canonical = joined
        .canonicalize()
        .or_else(|_| Ok::<_, std::io::Error>(joined.clone()))
        .ok();
    let cwd_canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    if let Some(c) = canonical {
        if !c.starts_with(&cwd_canonical) {
            anyhow::bail!("path escapes working directory: {}", rel);
        }
    }
    Ok(joined)
}

pub fn read_file(cwd: &Path, rel: &str) -> anyhow::Result<String> {
    let path = resolve(cwd, rel)?;
    let bytes = std::fs::read(&path)?;
    if bytes.len() > MAX_FILE_BYTES {
        anyhow::bail!(
            "file too large ({} bytes > {} limit) — use run_bash to inspect",
            bytes.len(),
            MAX_FILE_BYTES
        );
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn write_file(
    cwd: &Path,
    rel: &str,
    contents: &str,
    tx: &mpsc::UnboundedSender<AgentEvent>,
) -> anyhow::Result<String> {
    let path = resolve(cwd, rel)?;
    let before = std::fs::read_to_string(&path).ok();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, contents)?;
    let _ = tx.send(AgentEvent::Edit {
        path: path.clone(),
        before,
        after: contents.to_string(),
    });
    Ok(format!("wrote {} bytes to {}", contents.len(), rel))
}

pub fn list_files(cwd: &Path, rel: &str) -> anyhow::Result<String> {
    let dir = resolve(cwd, rel)?;
    let mut out = String::new();
    let entries = std::fs::read_dir(&dir)?;
    let mut names: Vec<(String, bool)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if matches!(name.as_str(), "target" | ".git" | "node_modules") {
            continue;
        }
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        names.push((name, is_dir));
    }
    names.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, is_dir) in names {
        if is_dir {
            out.push_str(&format!("{}/\n", name));
        } else {
            out.push_str(&format!("{}\n", name));
        }
    }
    Ok(out)
}

pub async fn run_bash(cwd: &Path, command: &str) -> anyhow::Result<String> {
    let output = tokio::process::Command::new("/bin/bash")
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .output()
        .await?;
    let cap = |s: &[u8]| {
        if s.len() > MAX_BASH_OUTPUT {
            let truncated = &s[..MAX_BASH_OUTPUT];
            format!(
                "{}\n... [truncated {} bytes]",
                String::from_utf8_lossy(truncated),
                s.len() - MAX_BASH_OUTPUT
            )
        } else {
            String::from_utf8_lossy(s).into_owned()
        }
    };
    let stdout = cap(&output.stdout);
    let stderr = cap(&output.stderr);
    Ok(format!(
        "exit: {}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status.code().unwrap_or(-1),
        stdout,
        stderr
    ))
}
