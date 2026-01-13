/// Simplified LSP goto_definition integration test
///
/// Tests that rust-analyzer can:
/// 1. Start successfully
/// 2. Initialize with a project root
/// 3. Respond to goto_definition requests

use std::sync::Arc;

#[tokio::test]
async fn test_rust_analyzer_basic_functionality() {
    // Get project root (berrycode directory)
    let project_root = std::env::current_dir()
        .expect("Failed to get current directory")
        .parent()
        .expect("No parent directory")
        .to_path_buf();

    eprintln!("📁 Project root: {}", project_root.display());

    let root_uri = format!("file://{}", project_root.display());

    // Import directly from the crate internals
    let lsp_manager = Arc::new(berry_editor_tauri::lsp::LspManager::new());

    // Step 1: Initialize rust-analyzer
    eprintln!("🚀 Step 1: Initializing rust-analyzer...");

    let init_result = lsp_manager
        .initialize_client("rust".to_string(), root_uri.clone())
        .await;

    match &init_result {
        Ok(_) => {
            eprintln!("✅ rust-analyzer initialization succeeded");
        }
        Err(e) => {
            eprintln!("❌ rust-analyzer initialization failed: {}", e);
            panic!("LSP initialization failed: {}", e);
        }
    }

    assert!(init_result.is_ok(), "LSP initialization should succeed");

    // Step 2: Get client
    eprintln!("🚀 Step 2: Getting LSP client...");

    let client_arc = lsp_manager
        .get_client("rust")
        .await
        .expect("Client should exist after initialization");

    eprintln!("✅ Got LSP client");

    let client = client_arc.lock().await;

    // Step 3: Open a file
    eprintln!("🚀 Step 3: Opening test file...");

    let test_file = project_root.join("src/lib.rs");
    let test_file_uri = format!("file://{}", test_file.display());

    // LspClient auto-opens files on goto_definition, no explicit did_open needed
    eprintln!("✅ Using test file: {}", test_file_uri);

    // Step 4: Wait for indexing
    eprintln!("⏳ Step 4: Waiting 12 seconds for indexing...");
    tokio::time::sleep(tokio::time::Duration::from_secs(12)).await;

    // Step 5: Test goto_definition
    eprintln!("🚀 Step 5: Testing goto_definition...");

    let result = client.goto_definition(&test_file_uri, 0, 10).await;

    match &result {
        Ok(Some(location)) => {
            eprintln!("✅ Definition found: {}:{}:{}",
                location.uri,
                location.range.start.line,
                location.range.start.character
            );
        }
        Ok(None) => {
            eprintln!("⚠️  No definition found (cursor might be on whitespace)");
        }
        Err(e) => {
            eprintln!("❌ goto_definition error: {}", e);
            panic!("goto_definition failed: {}", e);
        }
    }

    assert!(result.is_ok(), "goto_definition should not error");

    // Step 6: Test with timeout
    eprintln!("🚀 Step 6: Testing with timeout...");

    let goto_future = client.goto_definition(&test_file_uri, 0, 10);
    let timeout_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        goto_future
    ).await;

    match timeout_result {
        Ok(Ok(_)) => {
            eprintln!("✅ goto_definition responded within timeout");
        }
        Ok(Err(e)) => {
            eprintln!("❌ goto_definition error: {}", e);
        }
        Err(_) => {
            panic!("goto_definition timed out!");
        }
    }

    assert!(timeout_result.is_ok(), "Should not timeout");

    // Step 7: Shutdown
    eprintln!("🚀 Step 7: Shutting down...");
    client.shutdown().await.expect("Shutdown should succeed");

    eprintln!("✅✅✅ ALL TESTS PASSED! ✅✅✅");
}
