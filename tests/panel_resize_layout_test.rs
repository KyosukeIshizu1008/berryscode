#![cfg(not(target_arch = "wasm32"))]

use fantoccini::{ClientBuilder, Locator};
use serde_json::json;

async fn setup_client() -> Result<fantoccini::Client, Box<dyn std::error::Error>> {
    let mut caps = serde_json::map::Map::new();
    let chrome_opts = json!({
        "args": ["--headless", "--no-sandbox", "--disable-dev-shm-usage", "--disable-gpu"],
    });
    caps.insert("goog:chromeOptions".to_string(), chrome_opts);

    let client = ClientBuilder::native()
        .capabilities(caps)
        .connect("http://localhost:4444")
        .await?;

    client.goto("http://localhost:8080").await?;
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    Ok(client)
}

#[tokio::test]
#[ignore]
async fn test_panel_resize_layout_structure() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Wait for app to load
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Check that main container exists
    let main_area = client
        .find(Locator::Css(".berry-editor-main-area"))
        .await?;

    let display = main_area.css_value("display").await?;
    assert_eq!(display, "flex", "Main area should be flex");

    // Verify layout structure: Activity Bar + Sidebar + Resize Handle + Editor Area
    let result: serde_json::Value = client.execute(r#"
        const mainArea = document.querySelector('.berry-editor-main-area');
        if (!mainArea) return { error: 'Main area not found' };

        const children = Array.from(mainArea.children);
        const activityBar = children.find(el => el.classList.contains('activity-bar'));
        const sidebarContainer = children.find(el => el.style.width && el.style.width.includes('px'));
        const editorArea = children.find(el => el.style.flex === '1');

        // Find resize handle (5px width element)
        const resizeHandle = children.find(el => {
            const width = el.style.width;
            return width === '5px' || width.includes('5px');
        });

        return {
            totalChildren: children.length,
            hasActivityBar: !!activityBar,
            hasSidebarContainer: !!sidebarContainer,
            hasResizeHandle: !!resizeHandle,
            hasEditorArea: !!editorArea,
            sidebarWidth: sidebarContainer ? sidebarContainer.style.width : null,
            resizeHandleWidth: resizeHandle ? resizeHandle.style.width : null,
            childrenInfo: children.map(el => ({
                tagName: el.tagName,
                className: el.className,
                width: el.style.width,
                flex: el.style.flex
            }))
        };
    "#, vec![]).await?;

    println!("Layout structure: {:#?}", result);

    // Verify all components exist
    assert_eq!(result["hasActivityBar"], true, "Activity bar should exist");
    assert_eq!(result["hasSidebarContainer"], true, "Sidebar container should exist");
    assert_eq!(result["hasResizeHandle"], true, "Resize handle should exist");
    assert_eq!(result["hasEditorArea"], true, "Editor area should exist");

    // Verify editor area is visible
    let editor_visible: serde_json::Value = client.execute(r#"
        const mainArea = document.querySelector('.berry-editor-main-area');
        const children = Array.from(mainArea.children);
        const editorArea = children.find(el => el.style.flex === '1');

        if (!editorArea) return false;

        const rect = editorArea.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
    "#, vec![]).await?;

    assert_eq!(editor_visible, true, "Editor area should be visible with width and height > 0");

    client.close().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_canvas_editor_is_visible() -> Result<(), Box<dyn std::error::Error>> {
    let client = setup_client().await?;

    // Wait for app to load
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Check if canvas exists and is visible (canvas is inside .berry-editor-pane)
    let result: serde_json::Value = client.execute(r#"
        const pane = document.querySelector('.berry-editor-pane');
        if (!pane) return { error: 'Editor pane not found' };

        const canvas = pane.querySelector('canvas');
        if (!canvas) return { error: 'Canvas not found' };

        const rect = canvas.getBoundingClientRect();
        const computedStyle = window.getComputedStyle(canvas);
        const paneRect = pane.getBoundingClientRect();

        return {
            exists: true,
            width: rect.width,
            height: rect.height,
            paneWidth: paneRect.width,
            paneHeight: paneRect.height,
            display: computedStyle.display,
            visibility: computedStyle.visibility,
            opacity: computedStyle.opacity,
            isVisible: rect.width > 0 && rect.height > 0,
            hasGoodHeight: rect.height > 300  // Should be at least 300px tall
        };
    "#, vec![]).await?;

    println!("Canvas state: {:#?}", result);

    assert_eq!(result["exists"], true, "Canvas should exist");
    assert!(result["width"].as_f64().unwrap() > 0.0, "Canvas width should be > 0");
    assert!(result["height"].as_f64().unwrap() > 0.0, "Canvas height should be > 0");
    assert_eq!(result["isVisible"], true, "Canvas should be visible");

    // CRITICAL TEST: Canvas should have reasonable height (at least 300px)
    assert_eq!(
        result["hasGoodHeight"],
        true,
        "Canvas height should be at least 300px, got: {}",
        result["height"]
    );

    client.close().await?;
    Ok(())
}
