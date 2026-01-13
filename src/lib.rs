//! BerryEditor - 100% Rust Code Editor
//!
//! A fully-featured code editor built entirely in Rust using Dioxus and WASM.
//! No JavaScript required!

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub mod buffer;
pub mod components_tauri;
mod cursor;
pub mod css_classes;  // CSS class name constants
pub mod editor;
pub mod file_tree_tauri;
mod git;
mod search;
pub mod search_provider;  // Search provider abstraction
mod syntax;
pub mod theme;

// Common utilities (zero duplication)
pub mod common;

// Tauri bindings for native file access
pub mod tauri_bindings;
pub mod tauri_bindings_search;
pub mod tauri_bindings_database;
pub mod tauri_bindings_workflow;
pub mod tauri_bindings_terminal;
pub mod tauri_bindings_berrycode;

// ✅ Web Workers for background processing
pub mod syntax_worker; // ✅ Strategy 1: Non-blocking syntax analysis
pub mod tree_sitter_engine;
pub mod web_worker; // ✅ Strategy 2: Deep contextual analysis
                    // pub mod webgpu_renderer;  // ✅ Strategy 4: GPU-accelerated DOM-free rendering (requires web-sys WebGPU support)

// Core modules (Editor Engine)
pub mod core;

// Phase 2: Search functionality
pub mod search_panel;
pub mod search_dialog;

// Database Tools
pub mod database_panel;

// Workflow Automation
pub mod workflow_panel;

// Terminal
pub mod terminal_panel;

// BerryCode AI Assistant
pub mod berrycode_panel;

// Settings management
pub mod settings;

// Common types
pub mod types;

// Phase 1: High-performance rendering
pub mod highlight_job;
pub mod virtual_scroll; // ✅ IntelliJ Pro: Async syntax highlighting

// Phase 1: LSP UI Integration
pub mod completion_widget;
pub mod diagnostics_panel;
pub mod hover_tooltip;
pub mod lsp_ui;

// Phase 5: UX Polishing
pub mod command_palette;
pub mod focus_stack; // Global focus management
pub mod accessibility; // Screen reader support

// Phase 2: Debugger Integration
pub mod debugger;

// Phase 3: Refactoring Integration
pub mod refactoring;

// Phase 4: Git UI Integration (disabled in WASM - requires std::time)
#[cfg(not(target_arch = "wasm32"))]
pub mod git_ui;

use components_tauri::EditorAppTauri;

/// Test helper: Get mock file tree data for testing
#[wasm_bindgen]
pub fn get_test_file_tree() -> JsValue {
    // Return empty array - Tauri version uses native file system
    serde_wasm_bindgen::to_value(&Vec::<()>::new()).unwrap()
}

/// Initialize the BerryEditor WASM application
/// This is called automatically when WASM loads (via #[wasm_bindgen(start)])
#[wasm_bindgen(start)]
pub fn init_berry_editor() {
    // Set up better panic messages in development
    console_error_panic_hook::set_once();

    // ✅ Wait for JetBrains Mono and Codicons to load before rendering
    // This prevents the "flash of unstyled text" (FOUT) on Canvas
    wasm_bindgen_futures::spawn_local(async {
        // Get window.fontReady promise
        let window = web_sys::window().expect("no window");
        if let Ok(font_ready_val) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("fontReady")) {
            // fontReadyはPromiseとして扱う（async即時実行関数の結果）
            if !font_ready_val.is_undefined() {
                let promise = js_sys::Promise::from(font_ready_val);
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            }
        }

        // ✅ Launch Dioxus app
        dioxus_web::launch(EditorAppTauri);
    });
}
