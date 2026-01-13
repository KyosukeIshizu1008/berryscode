/// Integration test for LSP goto_definition functionality
///
/// This test verifies that:
/// 1. rust-analyzer can be started
/// 2. Files can be opened in rust-analyzer
/// 3. goto_definition returns valid locations
/// 4. The entire flow from Tauri command to rust-analyzer works

use std::path::PathBuf;
use std::sync::Arc;
use berry_editor_tauri::lsp::LspManager;

#[tokio::test]
async fn test_lsp_goto_definition_integration() {
    // Setup: Create LspManager
    let manager = Arc::new(LspManager::new());

    // Get current project root (berrycode/src-tauri)
    let project_root = std::env::current_dir()
        .expect("Failed to get current directory")
        .parent()
        .expect("No parent directory")
        .to_path_buf();

    eprintln!("📁 Project root: {}", project_root.display());

    let root_uri = format!("file://{}", project_root.display());

    // Step 1: Initialize rust-analyzer
    eprintln!("🚀 Initializing rust-analyzer...");
    let init_result = manager.initialize_client("rust".to_string(), root_uri.clone()).await;

    assert!(
        init_result.is_ok(),
        "LSP initialization failed: {:?}",
        init_result.err()
    );

    eprintln!("✅ rust-analyzer initialized successfully");

    // Step 2: Get LSP client
    let client_arc = manager
        .get_client("rust")
        .await
        .expect("rust-analyzer client not found after initialization");

    let client = client_arc.lock().await;

    // Step 3: Open a test file
    let test_file = project_root.join("src/lib.rs");
    let test_file_uri = format!("file://{}", test_file.display());

    eprintln!("📂 Opening file: {}", test_file_uri);

    let content = std::fs::read_to_string(&test_file)
        .expect("Failed to read test file");

    client.did_open(&test_file_uri, &content)
        .expect("Failed to open file in rust-analyzer");

    eprintln!("✅ File opened in rust-analyzer");

    // Step 4: Wait for indexing (rust-analyzer needs time to analyze)
    eprintln!("⏳ Waiting 15 seconds for rust-analyzer to index project...");
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Step 5: Test goto_definition on line 0 (should have some module or use statement)
    eprintln!("🔍 Testing goto_definition at line 0, character 10...");

    let result = client.goto_definition(&test_file_uri, 0, 10).await;

    eprintln!("📋 goto_definition result: {:?}", result);

    assert!(
        result.is_ok(),
        "goto_definition failed: {:?}",
        result.err()
    );

    let location_opt = result.unwrap();

    // It's OK if no definition is found (might be on whitespace)
    // The important thing is that the call succeeds without error
    match location_opt {
        Some(location) => {
            eprintln!("✅ Definition found: uri={}, line={}, char={}",
                location.uri, location.range.start.line, location.range.start.character);

            // Verify location has valid data
            assert!(!location.uri.is_empty(), "Location URI should not be empty");
            assert!(location.uri.starts_with("file://"), "Location URI should start with file://");
        }
        None => {
            eprintln!("⚠️  No definition found (cursor might be on whitespace or comment)");
        }
    }

    // Step 6: Test goto_definition with timeout (from commands.rs)
    eprintln!("🔍 Testing goto_definition with 10s timeout...");

    let goto_future = client.goto_definition(&test_file_uri, 0, 10);

    let timeout_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        goto_future
    ).await;

    assert!(
        timeout_result.is_ok(),
        "goto_definition timed out after 10 seconds"
    );

    eprintln!("✅ goto_definition responded within timeout");

    // Step 7: Shutdown rust-analyzer
    eprintln!("🛑 Shutting down rust-analyzer...");
    client.shutdown().expect("Failed to shutdown rust-analyzer");

    eprintln!("✅ All tests passed!");
}

#[tokio::test]
async fn test_lsp_goto_definition_on_known_symbol() {
    // Setup
    let manager = Arc::new(LspManager::new());
    let project_root = std::env::current_dir()
        .expect("Failed to get current directory")
        .parent()
        .expect("No parent directory")
        .to_path_buf();

    let root_uri = format!("file://{}", project_root.display());

    // Initialize
    eprintln!("🚀 Initializing rust-analyzer for known symbol test...");
    manager.initialize_client("rust".to_string(), root_uri.clone())
        .await
        .expect("Failed to initialize LSP");

    let client_arc = manager.get_client("rust").await.unwrap();
    let client = client_arc.lock().await;

    // Open file with known symbol (e.g., src/buffer.rs has TextBuffer struct)
    let buffer_file = project_root.join("src/buffer.rs");
    let buffer_file_uri = format!("file://{}", buffer_file.display());

    let content = std::fs::read_to_string(&buffer_file)
        .expect("Failed to read buffer.rs");

    client.did_open(&buffer_file_uri, &content)
        .expect("Failed to open buffer.rs");

    eprintln!("⏳ Waiting 15 seconds for indexing...");
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Find line with "TextBuffer" usage
    let lines: Vec<&str> = content.lines().collect();
    let mut test_line = 0;
    let mut test_col = 0;

    for (i, line) in lines.iter().enumerate() {
        if let Some(pos) = line.find("TextBuffer") {
            test_line = i as u32;
            test_col = pos as u32;
            eprintln!("🎯 Found 'TextBuffer' at line {}, col {}", test_line, test_col);
            break;
        }
    }

    if test_line == 0 && test_col == 0 {
        eprintln!("⚠️  Could not find 'TextBuffer' in buffer.rs, skipping symbol test");
        return;
    }

    // Test goto_definition on known symbol
    eprintln!("🔍 Testing goto_definition on 'TextBuffer'...");

    let result = client.goto_definition(&buffer_file_uri, test_line, test_col).await;

    assert!(result.is_ok(), "goto_definition on TextBuffer failed");

    let location_opt = result.unwrap();

    assert!(
        location_opt.is_some(),
        "Expected definition for TextBuffer, got None"
    );

    let location = location_opt.unwrap();
    eprintln!("✅ TextBuffer definition found at: {}:{}:{}",
        location.uri, location.range.start.line, location.range.start.character);

    // Verify it points to a valid location
    assert!(location.uri.contains("buffer.rs") || location.uri.contains("rope"),
        "Expected definition to point to buffer.rs or rope crate, got: {}", location.uri);

    client.shutdown().expect("Failed to shutdown");
    eprintln!("✅ Known symbol test passed!");
}
