use berry_editor::native::lsp_native::NativeLspClient;

#[tokio::test]
async fn test_native_lsp_rust_analyzer() {
    // Skip if rust-analyzer is not installed
    if std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("⚠️  Skipping test: rust-analyzer not installed");
        return;
    }

    let client = NativeLspClient::new();

    println!("\n=== Testing Native LSP with rust-analyzer ===");

    // Start rust-analyzer
    let project_root = std::env::current_dir()
        .expect("Failed to get current dir")
        .to_string_lossy()
        .to_string();

    println!("Project root: {}", project_root);

    match client.start_server("rust", &project_root).await {
        Ok(_) => {
            println!("✅ LSP server started successfully");

            // Wait a bit for initialization
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // Test getting completions (this might fail if no file is open, but server should respond)
            println!("\n=== Testing Completions ===");
            let file_path = format!("{}/src/lib.rs", project_root);
            match client.get_completions("rust", file_path, 0, 0).await {
                Ok(completions) => {
                    println!("✅ Got {} completions", completions.len());
                    for (i, item) in completions.iter().take(5).enumerate() {
                        println!("  {}. {:?}", i + 1, item.label);
                    }
                }
                Err(e) => {
                    println!("⚠️  Completions failed (expected if file doesn't exist): {}", e);
                }
            }

            // Shutdown
            println!("\n=== Shutting down ===");
            client.shutdown("rust").await.expect("Failed to shutdown");
            println!("✅ LSP server shutdown successfully");
        }
        Err(e) => {
            eprintln!("❌ Failed to start LSP server: {}", e);
            panic!("LSP server failed to start");
        }
    }
}

#[tokio::test]
async fn test_native_lsp_shutdown_all() {
    if std::process::Command::new("rust-analyzer")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("⚠️  Skipping test: rust-analyzer not installed");
        return;
    }

    let client = NativeLspClient::new();

    let project_root = std::env::current_dir()
        .expect("Failed to get current dir")
        .to_string_lossy()
        .to_string();

    // Start server
    client
        .start_server("rust", &project_root)
        .await
        .expect("Failed to start server");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Shutdown all
    client
        .shutdown_all()
        .await
        .expect("Failed to shutdown all");

    println!("✅ All servers shut down successfully");
}
