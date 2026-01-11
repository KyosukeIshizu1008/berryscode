//! Artifacts - Local preview server for generated web content (Standalone, No Web Feature Required)
//!
//! This is a completely standalone implementation that works with just `cargo run`.
//! No separate web server process needed!

use std::path::PathBuf;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::{Result, Context};
use tiny_http::{Server, Response, Header};

/// Artifacts server state
#[derive(Clone)]
pub struct ArtifactsState {
    artifacts_dir: PathBuf,
    last_modified: Arc<Mutex<std::time::SystemTime>>,
}

impl ArtifactsState {
    /// Create new artifacts state
    pub fn new(artifacts_dir: PathBuf) -> Self {
        Self {
            artifacts_dir,
            last_modified: Arc::new(Mutex::new(std::time::SystemTime::now())),
        }
    }

    /// Update artifact content
    pub fn update_content(&self, content: &str) -> Result<()> {
        let index_path = self.artifacts_dir.join("index.html");

        // Wrap content with auto-reload meta tag
        let wrapped_content = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta http-equiv="refresh" content="2">
    <title>BerryCode Artifact</title>
</head>
<body style="margin: 0; padding: 0;">
<div id="artifact-content">
{}
</div>
</body>
</html>"#,
            content
        );

        fs::write(&index_path, &wrapped_content)
            .with_context(|| format!(
                "アーティファクトファイルの書き込みに失敗しました: {:?}\n\
                解決方法:\n\
                1. ディレクトリの書き込み権限を確認してください: {:?}\n\
                2. 十分なディスク容量があるか確認してください\n\
                3. ファイルがロックされていないか確認してください\n\
                4. 別の場所に保存するには --artifacts-dir オプションを使用してください",
                index_path, index_path.parent()
            ))?;

        // Update modification time
        *self.last_modified.lock().unwrap() = std::time::SystemTime::now();

        tracing::info!("Updated artifact at {:?}", index_path);

        Ok(())
    }

    /// Get current artifact content
    pub fn get_content(&self) -> Result<String> {
        let index_path = self.artifacts_dir.join("index.html");

        if !index_path.exists() {
            return Ok(include_str!("../static/artifacts_template.html").to_string());
        }

        fs::read_to_string(&index_path)
            .with_context(|| format!(
                "アーティファクトファイルの読み込みに失敗しました: {:?}\n\
                解決方法:\n\
                1. ファイルが存在するか確認してください: ls {:?}\n\
                2. ファイルの読み取り権限を確認してください\n\
                3. ファイルが破損していないか確認してください\n\
                4. アーティファクトを再生成するには 'berryscode --clear-artifacts' を実行してください",
                index_path, index_path
            ))
    }
}

/// Standalone artifacts server (no axum/tokio required!)
pub struct ArtifactsServer {
    state: ArtifactsState,
    port: u16,
    artifacts_dir: PathBuf,
}

impl ArtifactsServer {
    /// Create new artifacts server
    pub fn new(project_root: &PathBuf, port: u16) -> Result<Self> {
        let artifacts_dir = project_root.join(".berrycode").join("artifacts");

        // Create artifacts directory
        fs::create_dir_all(&artifacts_dir)
            .with_context(|| format!(
                "アーティファクトディレクトリの作成に失敗しました: {:?}\n\
                解決方法:\n\
                1. 親ディレクトリの書き込み権限を確認してください: {:?}\n\
                2. 十分なディスク容量があるか確認してください\n\
                3. ディレクトリが既に存在する場合は削除して再試行してください\n\
                4. 別の場所にアーティファクトを保存するには --artifacts-dir オプションを使用してください",
                artifacts_dir, artifacts_dir.parent()
            ))?;

        // Create initial template
        let template_path = artifacts_dir.join("index.html");
        if !template_path.exists() {
            fs::write(&template_path, include_str!("../static/artifacts_template.html"))?;
        }

        Ok(Self {
            state: ArtifactsState::new(artifacts_dir.clone()),
            port,
            artifacts_dir,
        })
    }

    /// Start the server in background thread
    pub fn start_background(&self) -> Result<()> {
        let state = self.state.clone();
        let port = self.port;

        thread::spawn(move || {
            if let Err(e) = Self::run_server(state, port) {
                tracing::error!("Artifacts server error: {}", e);
            }
        });

        tracing::info!("🚀 Artifacts server started on http://localhost:{}", self.port);

        Ok(())
    }

    /// Run the HTTP server (blocking)
    fn run_server(state: ArtifactsState, port: u16) -> Result<()> {
        let server = Server::http(format!("127.0.0.1:{}", port))
            .map_err(|e| anyhow::anyhow!(
                "HTTPサーバーの起動に失敗しました: {}\n\
                解決方法:\n\
                1. ポート {} が既に使用されていないか確認してください: lsof -i:{}\n\
                2. 別のポートを指定するには --artifacts-port オプションを使用してください\n\
                3. ファイアウォール設定を確認してください\n\
                4. 管理者権限が必要な場合は sudo で実行してみてください",
                e, port, port
            ))?;

        tracing::info!("Artifacts server listening on port {}", port);

        for request in server.incoming_requests() {
            let path = request.url().to_string();

            let response = match path.as_str() {
                "/" | "/index.html" => {
                    match state.get_content() {
                        Ok(content) => {
                            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
                            Response::from_string(content).with_header(header)
                        }
                        Err(e) => {
                            let error_html = format!(
                                "<html><body><h1>Error</h1><p>{}</p></body></html>",
                                e
                            );
                            Response::from_string(error_html).with_status_code(500)
                        }
                    }
                }
                _ => {
                    let not_found = "<html><body><h1>404 Not Found</h1></body></html>";
                    Response::from_string(not_found).with_status_code(404)
                }
            };

            if let Err(e) = request.respond(response) {
                tracing::error!("Failed to send response: {}", e);
            }
        }

        Ok(())
    }

    /// Get server URL
    pub fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// Get artifacts directory
    pub fn artifacts_dir(&self) -> &PathBuf {
        &self.artifacts_dir
    }

    /// Update artifact content
    pub fn update(&self, content: &str) -> Result<()> {
        self.state.update_content(content)
    }

    /// Open browser to view artifact
    pub fn open_browser(&self) -> Result<()> {
        let url = self.url();
        webbrowser::open(&url)
            .with_context(|| format!("Failed to open browser to {}", url))?;

        tracing::info!("Opened browser to {}", url);

        Ok(())
    }
}

/// Global artifacts server instance
static ARTIFACTS_SERVER: once_cell::sync::OnceCell<Arc<Mutex<ArtifactsServer>>> = once_cell::sync::OnceCell::new();

/// Get or create global artifacts server
pub fn get_artifacts_server(project_root: &PathBuf) -> Result<Arc<Mutex<ArtifactsServer>>> {
    ARTIFACTS_SERVER.get_or_try_init(|| {
        let server = ArtifactsServer::new(project_root, 3456)?;
        server.start_background()?;
        Ok(Arc::new(Mutex::new(server)))
    }).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_artifacts_server_new() {
        let temp_dir = TempDir::new().unwrap();
        let server = ArtifactsServer::new(&temp_dir.path().to_path_buf(), 3457).unwrap();

        assert!(server.artifacts_dir().exists());
        assert!(server.artifacts_dir().join("index.html").exists());
        assert_eq!(server.url(), "http://localhost:3457");
    }

    #[test]
    fn test_artifacts_state_update() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();

        let state = ArtifactsState::new(temp_dir.path().to_path_buf());

        let content = "<h1>Test Artifact</h1>";
        state.update_content(content).unwrap();

        let written = fs::read_to_string(temp_dir.path().join("index.html")).unwrap();
        assert!(written.contains("Test Artifact"));
        assert!(written.contains("meta http-equiv=\"refresh\"")); // Auto-reload
    }

    #[test]
    fn test_artifacts_state_get_content() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path()).unwrap();

        let state = ArtifactsState::new(temp_dir.path().to_path_buf());

        // Initial content (template)
        let initial = state.get_content().unwrap();
        assert!(initial.contains("BerryCode Artifacts"));

        // After update
        state.update_content("<h1>Updated</h1>").unwrap();
        let updated = state.get_content().unwrap();
        assert!(updated.contains("Updated"));
    }
}
