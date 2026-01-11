//! Database Panel E2E Tests
//!
//! Tests Database Tools panel features in the actual Tauri desktop app:
//! - Panel mounting and display
//! - Empty state rendering
//! - Add connection dialog
//! - Connection list display
//! - Action buttons (Test, Edit, Delete)
//!
//! Run with: ./run_e2e_tests.sh

#![cfg(not(target_arch = "wasm32"))]

use fantoccini::{Client, ClientBuilder};
use serde_json::json;
use std::time::Duration;

async fn setup_client() -> Result<Client, Box<dyn std::error::Error>> {
    let mut caps = serde_json::map::Map::new();
    let opts = json!({
        "args": ["--headless"]
    });
    caps.insert("moz:firefoxOptions".to_string(), opts);

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;

    client.goto("http://localhost:8080").await?;
    tokio::time::sleep(Duration::from_millis(3000)).await;

    Ok(client)
}

#[tokio::test]
#[ignore]
async fn test_database_panel_mounts() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that Database panel mounts and displays header...");

    // Wait for app to load
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Click on Database icon in left sidebar to activate panel
    let result: serde_json::Value = client
        .execute(
            r#"
        // Find Database panel button (look for database icon or "Database" text)
        const buttons = Array.from(document.querySelectorAll('.bottom-panel-tabs button, .left-sidebar button'));
        const dbButton = buttons.find(btn =>
            btn.textContent.includes('Database') ||
            btn.querySelector('.codicon-database')
        );

        if (!dbButton) {
            return { success: false, error: "Database panel button not found" };
        }

        // Click to activate
        dbButton.click();

        // Wait a bit for panel to render
        await new Promise(resolve => setTimeout(resolve, 500));

        // Check if panel exists
        const sidebar = document.querySelector('.berry-editor-sidebar');
        if (!sidebar) {
            return { success: false, error: "Database sidebar not found after clicking" };
        }

        // Check header
        const header = sidebar.querySelector('.berry-editor-sidebar-header');
        if (!header) {
            return { success: false, error: "Database sidebar header not found" };
        }

        const headerText = header.textContent || '';
        console.log('📋 Database panel header:', headerText);

        if (!headerText.includes('DATABASE')) {
            return {
                success: false,
                error: `Header text incorrect. Got: ${headerText}`
            };
        }

        return { success: true, headerText: headerText };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            panic!("❌ {}", error);
        }
    }

    let header_text = result.get("headerText").and_then(|v| v.as_str()).unwrap_or("unknown");
    println!("✅ Database panel mounted with header: {}", header_text);

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_empty_state_displays() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that Database panel shows empty state...");

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        // Activate Database panel
        const buttons = Array.from(document.querySelectorAll('.bottom-panel-tabs button, .left-sidebar button'));
        const dbButton = buttons.find(btn =>
            btn.textContent.includes('Database') ||
            btn.querySelector('.codicon-database')
        );

        if (dbButton) {
            dbButton.click();
            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        // Check for empty state
        const emptyState = document.querySelector('.db-empty-state');
        if (!emptyState) {
            return { success: false, error: "Empty state element not found" };
        }

        const emptyText = emptyState.textContent || '';
        console.log('📋 Empty state text:', emptyText);

        // Check for "No database connections" text
        if (!emptyText.includes('No database connections')) {
            return {
                success: false,
                error: `Empty state text incorrect. Got: ${emptyText}`
            };
        }

        // Check for database icon
        const icon = emptyState.querySelector('.codicon-database');
        if (!icon) {
            return { success: false, error: "Database icon not found in empty state" };
        }

        // Check for Add Connection button
        const addButton = emptyState.querySelector('button');
        if (!addButton) {
            return { success: false, error: "Add Connection button not found in empty state" };
        }

        const buttonText = addButton.textContent || '';
        if (!buttonText.includes('Add Connection')) {
            return {
                success: false,
                error: `Button text incorrect. Got: ${buttonText}`
            };
        }

        return {
            success: true,
            emptyText: emptyText,
            hasIcon: true,
            buttonText: buttonText
        };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            panic!("❌ {}", error);
        }
    }

    println!("✅ Empty state displays correctly with icon and Add Connection button");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_add_button_triggers_dialog() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that Add button opens connection dialog...");

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        // Activate Database panel
        const buttons = Array.from(document.querySelectorAll('.bottom-panel-tabs button, .left-sidebar button'));
        const dbButton = buttons.find(btn =>
            btn.textContent.includes('Database') ||
            btn.querySelector('.codicon-database')
        );

        if (dbButton) {
            dbButton.click();
            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        // Find Add button in header
        const header = document.querySelector('.berry-editor-sidebar-header');
        if (!header) {
            return { success: false, error: "Sidebar header not found" };
        }

        const addButton = header.querySelector('button');
        if (!addButton) {
            return { success: false, error: "Add button not found in header" };
        }

        // Click Add button
        addButton.click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Check for dialog
        const dialog = document.querySelector('div[style*="position: fixed"]') ||
                      document.querySelector('.modal-dialog') ||
                      document.querySelector('[role="dialog"]');

        if (!dialog) {
            return { success: false, error: "Modal dialog not found after clicking Add button" };
        }

        const dialogText = dialog.textContent || '';
        console.log('📋 Dialog content:', dialogText.substring(0, 100));

        if (!dialogText.includes('Add Database Connection') && !dialogText.includes('Database')) {
            return {
                success: false,
                error: `Dialog content incorrect. Got: ${dialogText.substring(0, 100)}`
            };
        }

        return { success: true, dialogFound: true };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            panic!("❌ {}", error);
        }
    }

    println!("✅ Add button successfully opens connection dialog");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_cancel_button_closes_dialog() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that Cancel button closes connection dialog...");

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        // Activate Database panel
        const buttons = Array.from(document.querySelectorAll('.bottom-panel-tabs button, .left-sidebar button'));
        const dbButton = buttons.find(btn =>
            btn.textContent.includes('Database') ||
            btn.querySelector('.codicon-database')
        );

        if (dbButton) {
            dbButton.click();
            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        // Open dialog
        const header = document.querySelector('.berry-editor-sidebar-header');
        if (!header) {
            return { success: false, error: "Sidebar header not found" };
        }

        const addButton = header.querySelector('button');
        if (!addButton) {
            return { success: false, error: "Add button not found" };
        }

        addButton.click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Verify dialog is open
        let dialog = document.querySelector('div[style*="position: fixed"]') ||
                    document.querySelector('.modal-dialog') ||
                    document.querySelector('[role="dialog"]');

        if (!dialog) {
            return { success: false, error: "Dialog not opened" };
        }

        console.log('✅ Dialog opened');

        // Find Cancel button (first button is usually Cancel)
        const cancelButton = dialog.querySelector('button');
        if (!cancelButton) {
            return { success: false, error: "Cancel button not found in dialog" };
        }

        // Click Cancel
        cancelButton.click();
        await new Promise(resolve => setTimeout(resolve, 500));

        // Verify dialog is closed
        dialog = document.querySelector('div[style*="position: fixed"]') ||
                document.querySelector('.modal-dialog') ||
                document.querySelector('[role="dialog"]');

        if (dialog) {
            return { success: false, error: "Dialog still visible after clicking Cancel" };
        }

        console.log('✅ Dialog closed');

        return { success: true };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            panic!("❌ {}", error);
        }
    }

    println!("✅ Cancel button successfully closes dialog");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_connection_list_structure() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that connection list container exists...");

    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        // Activate Database panel
        const buttons = Array.from(document.querySelectorAll('.bottom-panel-tabs button, .left-sidebar button'));
        const dbButton = buttons.find(btn =>
            btn.textContent.includes('Database') ||
            btn.querySelector('.codicon-database')
        );

        if (dbButton) {
            dbButton.click();
            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        // Check for connection list container
        const connectionList = document.querySelector('.db-connection-list');
        if (!connectionList) {
            return { success: false, error: "Connection list container not found" };
        }

        console.log('✅ Connection list container found');

        return { success: true };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            panic!("❌ {}", error);
        }
    }

    println!("✅ Connection list structure exists");

    client.close().await?;
    Ok(())
}
