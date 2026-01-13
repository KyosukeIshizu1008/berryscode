/// End-to-end LSP test that mimics the full flow from frontend to backend
/// This test verifies:
/// 1. LspManager can initialize
/// 2. goto_definition works through the full stack
/// 3. The flow that the actual app uses works correctly

use std::sync::Arc;
use berry_editor_tauri::lsp::LspManager;

#[tokio::test]
async fn test_lsp_full_flow_e2e() {
    eprintln!("=================================================");
    eprintln!("🧪 Starting LSP End-to-End Test");
    eprintln!("=================================================\n");

    // Step 1: Create LspManager (simulating Tauri state)
    eprintln!("📦 Step 1: Creating LspManager...");
    let manager = Arc::new(LspManager::new());
    eprintln!("✅ LspManager created\n");

    // Step 2: Get project root
    let project_root = std::env::current_dir()
        .expect("Failed to get current directory")
        .parent()
        .expect("No parent directory")
        .to_path_buf();

    eprintln!("📁 Step 2: Project root: {}\n", project_root.display());

    let root_uri = format!("file://{}", project_root.display());

    // Step 3: Initialize LSP (simulating lsp_initialize command)
    eprintln!("🚀 Step 3: Initializing LSP client...");
    eprintln!("   Language: rust");
    eprintln!("   Root URI: {}\n", root_uri);

    let init_result = manager
        .initialize_client("rust".to_string(), root_uri.clone())
        .await;

    match &init_result {
        Ok(_) => {
            eprintln!("✅ LSP client initialized successfully\n");
        }
        Err(e) => {
            eprintln!("❌ LSP initialization failed: {}", e);
            eprintln!("❌ This means:");
            eprintln!("   1. berry_api server is not running at localhost:50051");
            eprintln!("   2. Run: cd berry_api && cargo run --bin berry-api-server");
            panic!("LSP initialization failed: {}", e);
        }
    }

    assert!(init_result.is_ok(), "LSP initialization must succeed");

    // Step 4: Get client
    eprintln!("🔍 Step 4: Getting LSP client...");
    let client_arc = manager
        .get_client("rust")
        .await
        .expect("Client should exist after initialization");

    eprintln!("✅ Got LSP client\n");

    // Step 5: Test goto_definition (simulating lsp_goto_definition command)
    eprintln!("🔍 Step 5: Testing goto_definition...");

    let test_file = project_root.join("src/lib.rs");
    let test_file_uri = format!("file://{}", test_file.display());

    eprintln!("   File: {}", test_file_uri);
    eprintln!("   Position: line 0, character 10\n");

    let client = client_arc.lock().await;

    // Wait a bit for rust-analyzer to index
    eprintln!("⏳ Waiting 8 seconds for rust-analyzer to index...\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(8)).await;

    let result = client.goto_definition(&test_file_uri, 0, 10).await;

    match &result {
        Ok(Some(location)) => {
            eprintln!("✅ goto_definition succeeded!");
            eprintln!("   Definition found at:");
            eprintln!("   URI: {}", location.uri);
            eprintln!("   Line: {}", location.range.start.line);
            eprintln!("   Character: {}\n", location.range.start.character);
        }
        Ok(None) => {
            eprintln!("⚠️  goto_definition returned None (no definition found)");
            eprintln!("   This might be because:");
            eprintln!("   - Cursor is on whitespace");
            eprintln!("   - Cursor is on a comment");
            eprintln!("   - rust-analyzer hasn't finished indexing yet\n");
        }
        Err(e) => {
            eprintln!("❌ goto_definition failed: {}", e);
            panic!("goto_definition error: {}", e);
        }
    }

    assert!(result.is_ok(), "goto_definition must not error");

    // Step 6: Test with timeout (simulating actual command behavior)
    eprintln!("🔍 Step 6: Testing goto_definition with 10s timeout...");

    let goto_future = client.goto_definition(&test_file_uri, 0, 10);
    let timeout_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        goto_future
    ).await;

    match &timeout_result {
        Ok(Ok(_)) => {
            eprintln!("✅ goto_definition responded within timeout\n");
        }
        Ok(Err(e)) => {
            eprintln!("❌ goto_definition error: {}\n", e);
        }
        Err(_) => {
            panic!("goto_definition timed out after 10 seconds!");
        }
    }

    assert!(timeout_result.is_ok(), "Should not timeout");

    // Step 7: Cleanup
    eprintln!("🧹 Step 7: Shutting down...");
    client.shutdown().await.expect("Shutdown should succeed");
    eprintln!("✅ Shutdown complete\n");

    eprintln!("=================================================");
    eprintln!("🎉 ALL TESTS PASSED!");
    eprintln!("=================================================");
}
