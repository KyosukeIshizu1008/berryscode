//! Web Worker Integration for Background Processing
//!
//! Runs heavy computations (indexing, parsing) in background threads
//! without blocking the UI

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use crate::core::bridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    IndexWorkspace { path: String, api_endpoint: Option<String> },
    SearchSymbols { query: String, api_endpoint: Option<String> },
    GetStatus,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerResponse {
    Ready,
    Progress { data: ProgressData },
    Status { data: StatusData },
    SearchResult { symbols: Vec<crate::tauri_bindings::Symbol> },
    CallTauri { command: String, args: serde_json::Value },
    Error { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub status: String,
    pub message: String,
    pub is_indexing: bool,
    pub total_symbols: usize,
    pub processed_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusData {
    pub is_indexing: bool,
    pub total_symbols: usize,
    pub processed_files: usize,
    pub errors: Vec<ErrorData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub message: String,
    pub timestamp: u64,
}

/// Web Worker wrapper for background indexing (100% web_sys free)
pub struct IndexerWorker {
    // ✅ No web_sys::Worker - just a handle for posting messages
    worker: bridge::WorkerHandle<WorkerMessage>,
}

impl IndexerWorker {
    /// Create a new worker instance
    ///
    /// This function sets up the worker through the bridge abstraction,
    /// keeping web_sys usage isolated to the bridge module.
    pub fn new(
        on_progress: RwSignal<Option<ProgressData>>,
        on_error: RwSignal<Option<String>>,
    ) -> Result<Self, JsValue> {
        // ✅ Use bridge to spawn worker - web_sys is hidden
        let worker = bridge::spawn_worker::<WorkerMessage>(
            "/indexer-worker.js",
            move |data| {
                if let Ok(response) = serde_wasm_bindgen::from_value::<WorkerResponse>(data) {
                    match response {
                        WorkerResponse::Ready => {
                            leptos::logging::log!("Worker ready");
                        }
                        WorkerResponse::Progress { data } => {
                            on_progress.set(Some(data));
                        }
                        WorkerResponse::Status { .. } => {
                            // Handle status if needed
                        }
                        WorkerResponse::SearchResult { .. } => {
                            // Handle search results if needed
                        }
                        WorkerResponse::CallTauri { command: _, args: _ } => {
                            // Note: Tauri calls handled separately via events
                            leptos::logging::log!("Tauri call requested");
                        }
                        WorkerResponse::Error { error } => {
                            on_error.set(Some(error));
                        }
                    }
                }
            },
        )?;

        Ok(Self { worker })
    }

    /// Index workspace
    pub fn index_workspace(&self, path: String) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(WorkerMessage::IndexWorkspace {
            path,
            api_endpoint: None,
        });
    }

    /// Search symbols
    pub fn search_symbols(&self, query: String) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(WorkerMessage::SearchSymbols {
            query,
            api_endpoint: None,
        });
    }

    /// Get status
    pub fn get_status(&self) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(WorkerMessage::GetStatus);
    }

    /// Cancel indexing
    pub fn cancel(&self) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(WorkerMessage::Cancel);
    }
}
