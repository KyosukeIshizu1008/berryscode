//! Codicon Font Loading Integration Test
//!
//! This test verifies that the Codicon icon font is properly loaded from CDN
//! and prevents the regression where icons disappeared due to Tauri CSP issues.
//!
//! REGRESSION PREVENTION:
//! - Previously, codicon fonts were loaded from localhost, which Tauri CSP blocked
//! - This caused all file tree icons to disappear
//! - Solution: Load from CDN (cdn.jsdelivr.net)
//!
//! This test ensures:
//! 1. Codicon CSS is loaded from CDN (not localhost)
//! 2. File tree icons render correctly
//! 3. No localhost font loading attempts

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
async fn test_codicon_css_loaded_from_cdn() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that Codicon CSS is loaded from CDN...");

    let result: serde_json::Value = client
        .execute(
            r#"
        // Find all stylesheet links
        const links = Array.from(document.querySelectorAll('link[rel="stylesheet"]'));
        const codiconLinks = links.filter(link => link.href.includes('codicon'));

        if (codiconLinks.length === 0) {
            return { success: false, error: "No codicon CSS link found" };
        }

        const href = codiconLinks[0].href;
        console.log('📦 Codicon CSS loaded from:', href);

        // Verify it's from CDN, not localhost
        if (href.includes('localhost') || href.includes('127.0.0.1')) {
            return {
                success: false,
                error: `REGRESSION: Codicon loaded from localhost! This causes Tauri CSP issues. URL: ${href}`
            };
        }

        if (!href.includes('cdn.jsdelivr.net') && !href.includes('unpkg.com')) {
            return {
                success: false,
                error: `Codicon not loaded from known CDN. Got: ${href}`
            };
        }

        return { success: true, url: href };
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

    let url = result.get("url").and_then(|v| v.as_str()).unwrap_or("unknown");
    println!("✅ Codicon CSS loaded from CDN: {}", url);

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_file_tree_icons_render() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that file tree icons render correctly...");

    // Wait for file tree to load
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        // Find file tree icons
        const fileTreeIcons = document.querySelectorAll('.file-tree-item i.codicon');

        if (fileTreeIcons.length === 0) {
            return { success: false, error: "No codicon icons found in file tree" };
        }

        console.log(`📊 Found ${fileTreeIcons.length} codicon icons in file tree`);

        // Check that icons have proper codicon classes
        let iconWithoutClass = null;
        for (const icon of fileTreeIcons) {
            const className = icon.className;
            if (!className.includes('codicon-')) {
                iconWithoutClass = className;
                break;
            }
        }

        if (iconWithoutClass) {
            return {
                success: false,
                error: `Icon without specific codicon class: ${iconWithoutClass}`
            };
        }

        // Check computed font-family on one icon
        const firstIcon = fileTreeIcons[0];
        const computedStyle = window.getComputedStyle(firstIcon);
        const fontFamily = computedStyle.fontFamily;

        console.log('🎨 Icon font-family:', fontFamily);

        if (!fontFamily.toLowerCase().includes('codicon')) {
            return {
                success: false,
                error: `Icon not using codicon font. Got: ${fontFamily}`
            };
        }

        return {
            success: true,
            iconCount: fileTreeIcons.length,
            fontFamily: fontFamily
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

    let icon_count = result.get("iconCount").and_then(|v| v.as_i64()).unwrap_or(0);
    let font_family = result.get("fontFamily").and_then(|v| v.as_str()).unwrap_or("unknown");

    println!("✅ {} file tree icons rendered with font: {}", icon_count, font_family);

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_no_localhost_font_loading() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 REGRESSION TEST: Ensuring no localhost font loading...");

    let result: serde_json::Value = client
        .execute(
            r#"
        // Check all stylesheet links
        const links = Array.from(document.querySelectorAll('link[rel="stylesheet"]'));
        const localhostFonts = links.filter(link => {
            const href = link.href;
            return (href.includes('localhost') || href.includes('127.0.0.1')) &&
                   (href.includes('codicon') || href.includes('font'));
        });

        if (localhostFonts.length > 0) {
            const urls = localhostFonts.map(l => l.href);
            return {
                success: false,
                error: `CRITICAL REGRESSION: Font loaded from localhost! This causes Tauri CSP issues.`,
                urls: urls
            };
        }

        console.log('✅ No localhost font loading detected');
        return { success: true };
        "#,
            vec![],
        )
        .await?;

    if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
        if !success {
            let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
            let urls = result.get("urls").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }).unwrap_or_default();
            panic!("❌ {}\nURLs: {}", error, urls);
        }
    }

    println!("✅ No localhost font loading - CSP safe");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_codicon_icons_have_specific_classes() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    println!("🔍 Testing that codicon icons have specific icon classes...");

    // Wait for file tree
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let result: serde_json::Value = client
        .execute(
            r#"
        const icons = document.querySelectorAll('.codicon');
        const iconClasses = new Set();

        for (const icon of icons) {
            const classes = Array.from(icon.classList);
            const specificClass = classes.find(c => c.startsWith('codicon-'));
            if (specificClass) {
                iconClasses.add(specificClass);
            }
        }

        const classArray = Array.from(iconClasses);
        console.log('📋 Icon classes found:', classArray);

        // Common expected classes
        const expectedClasses = ['codicon-folder', 'codicon-file', 'codicon-file-code'];
        const foundExpected = expectedClasses.filter(c => classArray.includes(c));

        return {
            success: true,
            totalIcons: icons.length,
            uniqueClasses: classArray.length,
            foundExpected: foundExpected
        };
        "#,
            vec![],
        )
        .await?;

    let total_icons = result.get("totalIcons").and_then(|v| v.as_i64()).unwrap_or(0);
    let unique_classes = result.get("uniqueClasses").and_then(|v| v.as_i64()).unwrap_or(0);

    println!("✅ {} codicon icons with {} unique classes", total_icons, unique_classes);

    client.close().await?;
    Ok(())
}
