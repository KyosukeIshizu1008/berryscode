//! Syntax Worker Integration
//!
//! Web Worker wrapper for non-blocking syntax analysis
//! Guarantees 144fps UI by isolating parsing to separate thread

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::core::bridge;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyntaxWorkerMessage {
    HighlightLines {
        lines: Vec<LineToHighlight>,
        language: Option<String>,
    },
    HighlightSingleLine {
        line_number: usize,
        text: String,
        language: Option<String>,
    },
    ClearCache,
    SetLanguage {
        language: String,
    },
    GetCacheStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineToHighlight {
    #[serde(rename = "lineNumber")]
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyntaxWorkerResponse {
    Ready,
    HighlightResult {
        results: Vec<HighlightedLine>,
    },
    SingleLineResult {
        #[serde(rename = "lineNumber")]
        line_number: usize,
        html: String,
    },
    CacheCleared,
    LanguageSet {
        language: String,
    },
    CacheStats {
        size: usize,
        #[serde(rename = "isAnalyzing")]
        is_analyzing: bool,
    },
    Error {
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighlightedLine {
    #[serde(rename = "lineNumber")]
    pub line_number: usize,
    pub html: String,
}

/// Syntax Worker wrapper (100% web_sys free)
pub struct SyntaxWorker {
    // ✅ No web_sys::Worker - just a handle for posting messages
    worker: bridge::WorkerHandle<SyntaxWorkerMessage>,
}

impl SyntaxWorker {
    /// Create a new syntax worker
    ///
    /// This function sets up the worker through the bridge abstraction,
    /// keeping web_sys usage isolated to the bridge module.
    pub fn new(
        on_result: RwSignal<Option<HashMap<usize, String>>>,
        on_error: RwSignal<Option<String>>,
    ) -> Result<Self, wasm_bindgen::JsValue> {
        // ✅ Use bridge to spawn worker - web_sys is hidden
        let worker = bridge::spawn_worker::<SyntaxWorkerMessage>(
            "/syntax-worker.js",
            move |data| {
                if let Ok(response) = serde_wasm_bindgen::from_value::<SyntaxWorkerResponse>(data) {
                    match response {
                        SyntaxWorkerResponse::Ready => {
                            leptos::logging::log!("Syntax worker ready");
                        }
                        SyntaxWorkerResponse::HighlightResult { results } => {
                            let mut map = HashMap::new();
                            for line in results {
                                map.insert(line.line_number, line.html);
                            }
                            on_result.set(Some(map));
                        }
                        SyntaxWorkerResponse::SingleLineResult { line_number, html } => {
                            let mut map = HashMap::new();
                            map.insert(line_number, html);
                            on_result.set(Some(map));
                        }
                        SyntaxWorkerResponse::CacheCleared => {
                            leptos::logging::log!("Cache cleared");
                        }
                        SyntaxWorkerResponse::LanguageSet { language } => {
                            leptos::logging::log!("Language set to: {}", language);
                        }
                        SyntaxWorkerResponse::CacheStats { size, is_analyzing } => {
                            leptos::logging::log!("Cache: {} items, analyzing: {}", size, is_analyzing);
                        }
                        SyntaxWorkerResponse::Error { error } => {
                            on_error.set(Some(error));
                        }
                    }
                }
            },
        )?;

        Ok(Self { worker })
    }

    /// Highlight multiple lines (batch)
    pub fn highlight_lines(&self, lines: Vec<(usize, String)>, language: Option<String>) {
        let lines_to_highlight: Vec<LineToHighlight> = lines
            .into_iter()
            .map(|(line_number, text)| LineToHighlight { line_number, text })
            .collect();

        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(SyntaxWorkerMessage::HighlightLines {
            lines: lines_to_highlight,
            language,
        });
    }

    /// Highlight single line (for real-time typing)
    pub fn highlight_single_line(&self, line_number: usize, text: String, language: Option<String>) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(SyntaxWorkerMessage::HighlightSingleLine {
            line_number,
            text,
            language,
        });
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(SyntaxWorkerMessage::ClearCache);
    }

    /// Set language
    pub fn set_language(&self, language: String) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(SyntaxWorkerMessage::SetLanguage { language });
    }

    /// Get cache stats
    pub fn get_cache_stats(&self) {
        // ✅ Use WorkerHandle - no web_sys::Worker
        self.worker.post(SyntaxWorkerMessage::GetCacheStats);
    }
}
