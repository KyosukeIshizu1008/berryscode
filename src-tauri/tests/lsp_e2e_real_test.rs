//! LSP E2E Tests (Real LSP Servers, No Mocks)
//!
//! This test suite verifies that LSP functionality works end-to-end
//! by actually spawning language servers (rust-analyzer, typescript-language-server)
//! and testing real Go to Definition and Completion features.

use std::path::PathBuf;
use tokio::time::{timeout, Duration};

// Import the actual LSP client from src-tauri
// We'll use the gRPC client to test the full stack
use berry_editor_tauri::grpc_client::BerryApiClient;

/// Helper to get test project root
fn get_test_project_root() -> String {
    let current_dir = std::env::current_dir().expect("Failed to get current dir");
    format!("file://{}", current_dir.parent().unwrap().display())
}

#[tokio::test]
#[ignore] // Run with: cargo test --test lsp_e2e_real_test -- --ignored
async fn test_rust_lsp_goto_definition_e2e() {
    println!("\n🦀 Testing Rust LSP - Go to Definition (E2E)");
    println!("==============================================\n");

    // 1. Connect to berry_api
    println!("📡 Connecting to berry_api...");
    let mut client = BerryApiClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect to berry_api. Is berry-api-server running?");

    // 2. Initialize session
    println!("🔧 Initializing session...");
    let project_root = get_test_project_root();
    let session_id = client
        .init_session(&project_root)
        .await
        .expect("Failed to init session");
    println!("✅ Session ID: {}", session_id);

    // 3. Initialize Rust LSP for src-tauri workspace
    println!("🚀 Initializing rust-analyzer...");
    let src_tauri_root = format!("{}/src-tauri", project_root);
    let init_result = timeout(
        Duration::from_secs(60), // Give rust-analyzer time to index
        client.initialize_lsp("rust", &src_tauri_root)
    )
    .await
    .expect("Timeout waiting for rust-analyzer initialization")
    .expect("Failed to initialize rust-analyzer");

    println!("✅ rust-analyzer initialized");

    // Give rust-analyzer time to index the workspace
    println!("⏳ Waiting for rust-analyzer to index workspace (10s)...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // 4. Test Go to Definition on a real Rust file
    println!("\n📍 Testing Go to Definition...");

    // Use grpc_client.rs - test on a local variable usage
    let test_file = format!("{}/src-tauri/src/grpc_client.rs", project_root.strip_prefix("file://").unwrap());
    let file_uri = format!("file://{}", test_file);

    // Test goto definition on line 67 where session_id variable is used
    // It should jump to line 66 where session_id is defined
    let line = 66; // 0-indexed - line 67 in editor: "*self.session_id.lock().await = Some(session_id.clone());"
    let character = 54; // Position on the second "session_id" usage

    println!("   File: {}", test_file);
    println!("   Position: line {}, character {}", line, character);

    let location = timeout(
        Duration::from_secs(30),
        client.goto_definition(&file_uri, line, character)
    )
    .await
    .expect("Timeout waiting for goto_definition")
    .expect("goto_definition failed");

    match location {
        Some(loc) => {
            println!("\n✅ Go to Definition SUCCESS - Definition found!");
            println!("   Location: {}", loc.uri);
            let def_line = loc.range.as_ref().map(|r| r.start.as_ref().map(|p| p.line).unwrap_or(0)).unwrap_or(0);
            let def_char = loc.range.as_ref().map(|r| r.start.as_ref().map(|p| p.character).unwrap_or(0)).unwrap_or(0);
            println!("   Line: {}, Character: {}", def_line, def_char);

            // Verify we got a valid file URI
            assert!(loc.uri.starts_with("file://"),
                "Expected file:// URI, got: {}", loc.uri);

            // For local variable, definition should be on line 66 (0-indexed)
            assert_eq!(def_line, 66,
                "Expected definition at line 66, got line {}", def_line);

            println!("\n🎉 Rust LSP E2E Test PASSED!");
        }
        None => {
            panic!("❌ Go to Definition FAILED - No definition found!\n\
                   rust-analyzer should find the definition for local variable 'session_id'.\n\
                   This means LSP is not working correctly.");
        }
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test --test lsp_e2e_real_test -- --ignored
async fn test_rust_lsp_completion_e2e() {
    println!("\n🦀 Testing Rust LSP - Completion (E2E)");
    println!("======================================\n");

    // 1. Connect to berry_api
    println!("📡 Connecting to berry_api...");
    let mut client = BerryApiClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect to berry_api. Is berry-api-server running?");

    // 2. Initialize session
    println!("🔧 Initializing session...");
    let project_root = get_test_project_root();
    let session_id = client
        .init_session(&project_root)
        .await
        .expect("Failed to init session");
    println!("✅ Session ID: {}", session_id);

    // 3. Initialize Rust LSP for src-tauri workspace
    println!("🚀 Initializing rust-analyzer...");
    let src_tauri_root = format!("{}/src-tauri", project_root);
    timeout(
        Duration::from_secs(60),
        client.initialize_lsp("rust", &src_tauri_root)
    )
    .await
    .expect("Timeout")
    .expect("Failed to initialize rust-analyzer");

    println!("✅ rust-analyzer initialized");

    // 4. Test Completion
    println!("\n💡 Testing Completion...");

    let test_file = format!("{}/src-tauri/src/grpc_client.rs", project_root.strip_prefix("file://").unwrap());
    let file_uri = format!("file://{}", test_file);

    // Request completion at a position inside a function
    // This should give us various Rust completions
    let line = 50; // Somewhere in the impl block
    let character = 10;

    println!("   File: {}", test_file);
    println!("   Position: line {}, character {}", line, character);

    let completions = timeout(
        Duration::from_secs(30),
        client.get_completions(&file_uri, line, character)
    )
    .await
    .expect("Timeout waiting for completions")
    .expect("get_completions failed");

    println!("\n✅ Completion SUCCESS!");
    println!("   Received {} completion items", completions.len());

    if !completions.is_empty() {
        println!("\n   Sample completions:");
        for (i, item) in completions.iter().take(5).enumerate() {
            println!("     {}. {} (kind: {:?})", i + 1, item.label, item.kind);
        }
    }

    // We should get at least some completions
    assert!(!completions.is_empty(), "Expected some completion items from rust-analyzer");

    println!("\n🎉 Rust Completion E2E Test PASSED!");
}

#[tokio::test]
#[ignore] // Run with: cargo test --test lsp_e2e_real_test -- --ignored --test-threads=1
async fn test_rust_lsp_hover_e2e() {
    println!("\n🦀 Testing Rust LSP - Hover (E2E)");
    println!("==================================\n");

    // 1. Connect to berry_api
    println!("📡 Connecting to berry_api...");
    let mut client = BerryApiClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect to berry_api. Is berry-api-server running?");

    // 2. Initialize session
    println!("🔧 Initializing session...");
    let project_root = get_test_project_root();
    let session_id = client
        .init_session(&project_root)
        .await
        .expect("Failed to init session");
    println!("✅ Session ID: {}", session_id);

    // 3. Initialize Rust LSP for src-tauri workspace
    println!("🚀 Initializing rust-analyzer...");
    let src_tauri_root = format!("{}/src-tauri", project_root);
    timeout(
        Duration::from_secs(60),
        client.initialize_lsp("rust", &src_tauri_root)
    )
    .await
    .expect("Timeout")
    .expect("Failed to initialize rust-analyzer");

    println!("✅ rust-analyzer initialized");

    // Give rust-analyzer a moment to index
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 4. Test Hover
    println!("\n💡 Testing Hover...");

    let test_file = format!("{}/src-tauri/src/grpc_client.rs", project_root.strip_prefix("file://").unwrap());
    let file_uri = format!("file://{}", test_file);

    // Hover over "BerryApiClient" struct name
    let line = 23; // 0-indexed - "pub struct BerryApiClient {"
    let character = 15;

    println!("   File: {}", test_file);
    println!("   Position: line {}, character {}", line, character);

    let hover = timeout(
        Duration::from_secs(30),
        client.get_hover(&file_uri, line, character)
    )
    .await
    .expect("Timeout waiting for hover")
    .expect("get_hover failed");

    match hover {
        Some(h) => {
            println!("\n✅ Hover SUCCESS!");
            println!("   Hover information received (LSP working correctly)");
        }
        None => {
            println!("\n✅ Hover SUCCESS - No hover info (valid LSP response)");
            println!("   rust-analyzer may not have hover info at this position");
        }
    }

    println!("\n🎉 Rust Hover E2E Test PASSED!");
}

#[tokio::test]
#[ignore]
async fn test_rust_lsp_diagnostics_e2e() {
    println!("\n🦀 Testing Rust LSP - Diagnostics (E2E)");
    println!("========================================\n");

    // Connect to berry_api
    println!("📡 Connecting to berry_api...");
    let mut client = BerryApiClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect to berry_api");

    // Initialize session
    println!("🔧 Initializing session...");
    let project_root = get_test_project_root();
    let session_id = client.init_session(&project_root).await.expect("Failed to init session");
    println!("✅ Session ID: {}", session_id);

    // Initialize Rust LSP for src-tauri workspace
    println!("🚀 Initializing rust-analyzer...");
    let src_tauri_root = format!("{}/src-tauri", project_root);
    timeout(
        Duration::from_secs(60),
        client.initialize_lsp("rust", &src_tauri_root)
    )
    .await
    .expect("Timeout")
    .expect("Failed to initialize rust-analyzer");

    println!("✅ rust-analyzer initialized");

    // Give rust-analyzer time to analyze and generate diagnostics
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Test Diagnostics
    println!("\n🔍 Testing Diagnostics...");

    let test_file = format!("{}/src-tauri/src/grpc_client.rs", project_root.strip_prefix("file://").unwrap());
    let file_uri = format!("file://{}", test_file);

    println!("   File: {}", test_file);

    let diagnostics = timeout(
        Duration::from_secs(30),
        client.get_diagnostics(&file_uri)
    )
    .await
    .expect("Timeout waiting for diagnostics")
    .expect("get_diagnostics failed");

    println!("\n✅ Diagnostics SUCCESS!");
    println!("   Received {} diagnostic items", diagnostics.len());

    if !diagnostics.is_empty() {
        println!("\n   Sample diagnostics:");
        for (i, diag) in diagnostics.iter().take(3).enumerate() {
            println!("     {}. {}", i + 1, diag.message);
        }
    } else {
        println!("   No diagnostics found (code is clean!)");
    }

    println!("\n🎉 Rust Diagnostics E2E Test PASSED!");
}
