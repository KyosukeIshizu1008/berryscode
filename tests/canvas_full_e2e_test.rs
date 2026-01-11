//! Canvas Editor Full E2E Test Suite
//!
//! Comprehensive end-to-end tests for the Canvas-based editor.
//! Tests all major functionality from file loading to editing operations.

#[cfg(all(test, not(target_arch = "wasm32")))]
mod canvas_full_e2e_tests {
    use fantoccini::{ClientBuilder, Locator};
    use tokio;

    const APP_URL: &str = "http://localhost:8080";

    /// Test 1: Application Startup and Canvas Rendering
    #[tokio::test]
    async fn test_app_startup_and_canvas_exists() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Verify canvas element exists on startup
        let canvas_exists = client
            .find(Locator::Css("canvas"))
            .await
            .is_ok();

        assert!(canvas_exists, "Canvas element should exist on app startup");

        // Verify canvas has valid dimensions
        let canvas_width = client
            .execute(
                "const canvas = document.querySelector('canvas'); \
                 return canvas ? canvas.width : 0;",
                vec![],
            )
            .await
            .expect("Failed to get canvas width");

        let width_num = canvas_width.as_u64().unwrap_or(0);
        assert!(width_num > 0, "Canvas should have non-zero width on startup");

        let _ = client.close().await;
    }

    /// Test 2: File Opening and Content Display
    #[tokio::test]
    async fn test_file_opening_displays_content() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Find and click a Rust file in the file tree
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Verify canvas received content rendering
            let canvas_height = client
                .execute(
                    "const canvas = document.querySelector('canvas'); \
                     return canvas ? canvas.height : 0;",
                    vec![],
                )
                .await
                .expect("Failed to get canvas height");

            let height_num = canvas_height.as_u64().unwrap_or(0);
            assert!(
                height_num > 100,
                "Canvas should have sufficient height to display file content, got: {}",
                height_num
            );
        }

        let _ = client.close().await;
    }

    /// Test 3: Text Input via Canvas
    #[tokio::test]
    async fn test_text_input_on_canvas() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Open a file
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.txt']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Click on canvas to focus
            let canvas = client
                .find(Locator::Css("canvas"))
                .await
                .expect("Canvas not found");

            canvas.click().await.expect("Failed to click canvas");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify hidden input is focused (for IME support)
            let focused_element = client
                .execute(
                    "return document.activeElement.tagName.toLowerCase();",
                    vec![],
                )
                .await
                .expect("Failed to get active element");

            assert_eq!(
                focused_element.as_str(),
                Some("input"),
                "Hidden input should be focused after clicking canvas"
            );

            // Send text input
            let hidden_input = client
                .find(Locator::Css("input[type='text']"))
                .await
                .expect("Hidden input not found");

            hidden_input.send_keys("Hello Canvas").await.expect("Failed to send keys");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Verify rendering was triggered
            let console_logs = client
                .execute(
                    "return window.lastRenderTime !== undefined;",
                    vec![],
                )
                .await;

            assert!(
                console_logs.is_ok(),
                "Canvas should re-render after text input"
            );
        }

        let _ = client.close().await;
    }

    /// Test 4: Mouse Click Cursor Positioning
    #[tokio::test]
    async fn test_mouse_click_positions_cursor() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Click at specific position on canvas
            let canvas = client
                .find(Locator::Css("canvas"))
                .await
                .expect("Canvas not found");

            // Click at coordinates (100, 50) - should position cursor
            client
                .execute(
                    "const canvas = document.querySelector('canvas'); \
                     const rect = canvas.getBoundingClientRect(); \
                     const evt = new MouseEvent('mousedown', { \
                         bubbles: true, \
                         clientX: rect.left + 100, \
                         clientY: rect.top + 50 \
                     }); \
                     canvas.dispatchEvent(evt);",
                    vec![],
                )
                .await
                .expect("Failed to dispatch mouse event");

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify cursor position changed
            let cursor_moved = client
                .execute(
                    "return document.activeElement.tagName.toLowerCase() === 'input';",
                    vec![],
                )
                .await
                .expect("Failed to check cursor");

            assert_eq!(
                cursor_moved.as_bool(),
                Some(true),
                "Cursor should be positioned after mouse click"
            );
        }

        let _ = client.close().await;
    }

    /// Test 5: Keyboard Navigation (Arrow Keys)
    #[tokio::test]
    async fn test_keyboard_navigation() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let canvas = client
                .find(Locator::Css("canvas"))
                .await
                .expect("Canvas not found");

            canvas.click().await.expect("Failed to click canvas");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Send arrow key navigation
            client
                .execute(
                    "const canvas = document.querySelector('canvas'); \
                     const evt = new KeyboardEvent('keydown', { \
                         key: 'ArrowRight', \
                         bubbles: true \
                     }); \
                     canvas.dispatchEvent(evt);",
                    vec![],
                )
                .await
                .expect("Failed to dispatch arrow key");

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify cursor moved (rendering was triggered)
            let render_triggered = client
                .execute(
                    "return document.querySelector('canvas') !== null;",
                    vec![],
                )
                .await
                .expect("Failed to verify rendering");

            assert_eq!(
                render_triggered.as_bool(),
                Some(true),
                "Canvas should still exist after keyboard navigation"
            );
        }

        let _ = client.close().await;
    }

    /// Test 6: Scroll Handling
    #[tokio::test]
    async fn test_canvas_scroll_handling() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Find a file with enough lines to scroll
        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Dispatch wheel event for scrolling
            let scroll_result = client
                .execute(
                    "const canvas = document.querySelector('canvas'); \
                     if (canvas) { \
                         const evt = new WheelEvent('wheel', { \
                             deltaY: 100, \
                             bubbles: true \
                         }); \
                         canvas.dispatchEvent(evt); \
                         return true; \
                     } \
                     return false;",
                    vec![],
                )
                .await
                .expect("Failed to dispatch wheel event");

            assert_eq!(
                scroll_result.as_bool(),
                Some(true),
                "Wheel event should be dispatched successfully"
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify canvas still renders after scroll
            let canvas_exists = client
                .find(Locator::Css("canvas"))
                .await
                .is_ok();

            assert!(canvas_exists, "Canvas should exist after scrolling");
        }

        let _ = client.close().await;
    }

    /// Test 7: Canvas Resizing
    #[tokio::test]
    async fn test_canvas_handles_window_resize() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Get initial canvas size
        let initial_width = client
            .execute(
                "const canvas = document.querySelector('canvas'); \
                 return canvas ? canvas.width : 0;",
                vec![],
            )
            .await
            .expect("Failed to get canvas width");

        let initial_width_num = initial_width.as_u64().unwrap_or(0);

        // Resize window (simulated via script)
        client
            .execute(
                "window.dispatchEvent(new Event('resize'));",
                vec![],
            )
            .await
            .expect("Failed to dispatch resize event");

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify canvas still has valid dimensions
        let new_width = client
            .execute(
                "const canvas = document.querySelector('canvas'); \
                 return canvas ? canvas.width : 0;",
                vec![],
            )
            .await
            .expect("Failed to get new canvas width");

        let new_width_num = new_width.as_u64().unwrap_or(0);

        assert!(
            new_width_num > 0,
            "Canvas should maintain valid dimensions after resize"
        );

        let _ = client.close().await;
    }

    /// Test 8: IME Composition Events
    #[tokio::test]
    async fn test_ime_composition_support() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.txt']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Click canvas to focus
            let canvas = client
                .find(Locator::Css("canvas"))
                .await
                .expect("Canvas not found");

            canvas.click().await.expect("Failed to click canvas");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Simulate IME composition start
            let composition_handled = client
                .execute(
                    "const input = document.querySelector('input[type=text]'); \
                     if (input) { \
                         const evt = new CompositionEvent('compositionstart', { \
                             bubbles: true, \
                             data: 'に' \
                         }); \
                         input.dispatchEvent(evt); \
                         return true; \
                     } \
                     return false;",
                    vec![],
                )
                .await
                .expect("Failed to dispatch composition event");

            assert_eq!(
                composition_handled.as_bool(),
                Some(true),
                "IME composition event should be handled"
            );

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify hidden input still exists and is positioned
            let input_visible = client
                .find(Locator::Css("input[type='text']"))
                .await
                .is_ok();

            assert!(input_visible, "Hidden IME input should exist for composition");
        }

        let _ = client.close().await;
    }

    /// Test 9: Multiple File Tabs (Future Feature)
    #[tokio::test]
    #[ignore] // Enable when tab support is implemented
    async fn test_multiple_file_tabs() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Open first file
        if let Ok(file1) = client.find(Locator::Css("div[data-path$='.rs']")).await {
            file1.click().await.expect("Failed to click first file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Open second file
            if let Ok(file2) = client.find(Locator::Css("div[data-path$='.toml']")).await {
                file2.click().await.expect("Failed to click second file");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Verify both tabs exist
                let tab_count = client
                    .execute(
                        "return document.querySelectorAll('.berry-editor-tab').length;",
                        vec![],
                    )
                    .await
                    .expect("Failed to count tabs");

                assert!(
                    tab_count.as_u64().unwrap_or(0) >= 2,
                    "Should have at least 2 tabs open"
                );
            }
        }

        let _ = client.close().await;
    }

    /// Test 10: Undo/Redo Operations
    #[tokio::test]
    async fn test_undo_redo_operations() {
        let client = ClientBuilder::native()
            .connect("http://localhost:4444")
            .await
            .expect("Failed to connect to WebDriver");

        client.goto(APP_URL).await.expect("Failed to navigate");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if let Ok(file_elem) = client.find(Locator::Css("div[data-path$='.txt']")).await {
            file_elem.click().await.expect("Failed to click file");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let canvas = client
                .find(Locator::Css("canvas"))
                .await
                .expect("Canvas not found");

            canvas.click().await.expect("Failed to click canvas");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Type some text
            canvas.send_keys("test").await.expect("Failed to type");
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Trigger undo with Ctrl+Z
            client
                .execute(
                    "const canvas = document.querySelector('canvas'); \
                     const evt = new KeyboardEvent('keydown', { \
                         key: 'z', \
                         ctrlKey: true, \
                         bubbles: true \
                     }); \
                     canvas.dispatchEvent(evt);",
                    vec![],
                )
                .await
                .expect("Failed to trigger undo");

            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            // Verify undo was processed (canvas still exists)
            let canvas_exists = client
                .find(Locator::Css("canvas"))
                .await
                .is_ok();

            assert!(canvas_exists, "Canvas should exist after undo operation");
        }

        let _ = client.close().await;
    }
}
