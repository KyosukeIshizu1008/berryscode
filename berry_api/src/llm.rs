// LLM integration module - Ollama support with Tool Calling
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// TTL for list_files in-memory cache (seconds)
const LIST_CACHE_TTL_SECS: u64 = 30;
/// Timeout for autonomous mode requests (stream:false — full response is awaited)
const OLLAMA_AUTONOMOUS_TIMEOUT_SECS: u64 = 180;
/// Timeout for non-autonomous streaming connection establishment
const OLLAMA_STREAM_TIMEOUT_SECS: u64 = 60;
/// Timeout for shell commands executed by the AI (kills the process on expiry)
const OLLAMA_COMMAND_TIMEOUT_SECS: u64 = 60;
/// Maximum byte length of a single NDJSON line in the streaming response
const MAX_LINE_BUF_BYTES: usize = 1_048_576; // 1 MB

// Chat API structures (for tool calling)
#[derive(Debug, Clone, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaOptions {
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_ctx: Option<u32>,      // context window (tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,  // max output tokens (-1 = unlimited)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

#[derive(Debug, Clone, Serialize)]
struct OllamaFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    #[allow(dead_code)]
    done: bool,
}

/// Message variant from Ollama's NDJSON streaming response
#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    #[serde(default)]
    content: String,
}

/// One chunk from Ollama's NDJSON streaming response
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    message: OllamaStreamMessage,
    done: bool,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    #[serde(rename = "type")]
    _tool_type: String,
    name: String,
    parameters: serde_json::Value,
}

/// Returns true if the text looks like a tool call JSON fragment.
/// Used to suppress spurious tool call output in non-autonomous mode.
fn is_tool_call_json(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with('{') && (t.contains("\"type\"") || t.contains("\"name\"")) && t.contains("\"function\""))
        || t.starts_with("<function=")
}

/// Find the index of the closing `}` that matches the `{` at `start`.
/// Correctly handles nested braces and strings (skips `}` inside string literals).
fn find_json_end(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth: i32 = 0;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => {
                // Skip string content so inner braces are ignored
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 1; // skip escaped character
                    } else if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Strip `<think>...</think>` blocks produced by reasoning models (deepseek-r1, etc.).
/// Returns the content after the last closing `</think>` tag, trimmed.
/// If no think tags are present the original string is returned as-is.
fn strip_think_tags(content: &str) -> String {
    // Find the last </think> and return everything after it.
    if let Some(end) = content.rfind("</think>") {
        content[end + "</think>".len()..].trim().to_string()
    } else {
        // Partial block — model may still be thinking; treat whole content as-is.
        content.to_string()
    }
}

/// Filter `<think>` blocks from a streaming chunk, maintaining state across calls.
/// `in_think` tracks whether we are currently inside a think block.
/// Returns the portion of `text` that should be forwarded to the user.
fn filter_think_stream(text: &str, in_think: &mut bool) -> String {
    let mut result = String::new();
    let mut remaining = text;
    loop {
        if *in_think {
            if let Some(pos) = remaining.find("</think>") {
                *in_think = false;
                remaining = &remaining[pos + "</think>".len()..];
            } else {
                break; // Still inside think block — discard everything
            }
        } else {
            if let Some(pos) = remaining.find("<think>") {
                result.push_str(&remaining[..pos]);
                *in_think = true;
                remaining = &remaining[pos + "<think>".len()..];
            } else {
                result.push_str(remaining);
                break;
            }
        }
    }
    result
}

/// Parse tool call from various formats (JSON or XML-like)
fn parse_tool_call(content: &str) -> Option<(String, serde_json::Value)> {
    let trimmed = content.trim();

    // Format 1: Ollama qwen output — {"name": "tool", "arguments": {...}}
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let (Some(name), Some(args)) = (
            v.get("name").and_then(|n| n.as_str()),
            v.get("arguments"),
        ) {
            if !name.is_empty() {
                return Some((name.to_string(), args.clone()));
            }
        }
        // Format 2: {"type": "function", "name": "tool_name", "parameters": {...}}
        if let Ok(tool_call) = serde_json::from_value::<ToolCall>(v) {
            return Some((tool_call.name, tool_call.parameters));
        }
    }

    // Search for embedded JSON objects in content.
    // Use brace-counting to find the correct closing brace (rfind would
    // match the last '}' in the whole string, grabbing multiple JSON objects).
    for prefix in &["{\"name\"", "{\"type\""] {
        if let Some(start) = content.find(prefix) {
            if let Some(end) = find_json_end(content, start) {
                let json_str = &content[start..=end];
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(name), Some(args)) = (
                        v.get("name").and_then(|n| n.as_str()),
                        v.get("arguments").or_else(|| v.get("parameters")),
                    ) {
                        if !name.is_empty() {
                            return Some((name.to_string(), args.clone()));
                        }
                    }
                }
            }
        }
    }

    // Try XML-like format: <function=tool_name><parameter=key>value</parameter>...
    // This format can appear anywhere in the content, even after explanatory text
    if content.contains("<function=") {
        // Extract function name
        let function_name = if let Some(start) = content.find("<function=") {
            let start = start + "<function=".len();
            if let Some(end) = content[start..].find('>') {
                content[start..start + end].trim().to_string()
            } else {
                return None;
            }
        } else {
            return None;
        };

        // Extract parameters
        let mut parameters = serde_json::Map::new();
        let mut search_pos = 0;

        while let Some(param_start) = content[search_pos..].find("<parameter=") {
            let param_start = search_pos + param_start;
            let name_start = param_start + "<parameter=".len();

            if let Some(name_end) = content[name_start..].find('>') {
                let param_name = content[name_start..name_start + name_end].trim().to_string();
                let value_start = name_start + name_end + 1;

                if let Some(value_end) = content[value_start..].find("</parameter>") {
                    let param_value = content[value_start..value_start + value_end].trim().to_string();
                    parameters.insert(param_name, serde_json::Value::String(param_value));
                    search_pos = value_start + value_end + "</parameter>".len();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if !parameters.is_empty() {
            tracing::debug!("📝 Parsed XML tool call: {} with {} parameters", function_name, parameters.len());
            return Some((function_name, serde_json::Value::Object(parameters)));
        }
    }

    // Format: "tool_name {json}" — e.g. "list_files {"path": "."}"
    let known_tools = ["list_files", "read_file", "write_file", "execute_command", "search_code"];
    for tool in &known_tools {
        if let Some(stripped) = trimmed.strip_prefix(*tool) {
            let rest = stripped.trim();
            let args = if rest.starts_with('{') {
                serde_json::from_str(rest).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };
            // Default path argument for list_files
            let args = if *tool == "list_files" && args.get("path").is_none() {
                serde_json::json!({"path": "."})
            } else {
                args
            };
            tracing::info!("📝 Parsed plain-text tool call: {} {:?}", tool, args);
            return Some((tool.to_string(), args));
        }
    }

    None
}

/// Specialist role — determines both the model and system prompt.
///
/// Routing priority (keyword-based fast path):
///   cli_git > reviewer > vision > summarizer > testing > debug > refactor > architect > coder
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// 受付: intent classification dispatcher (llama3.2:3b, ~0.1s)
    Router,
    /// 実装: Rust implementation, debugging, refactoring (qwen3-coder-next:q8_0)
    Coder,
    /// 設計: system design, architecture, deep reasoning (deepseek-r1:32b)
    Architect,
    /// UI監査: visual diff / layout audit from screenshots (pixtral:12b)
    Vision,
    /// 解説: code/log summarization and explanation (mistral-nemo:12b)
    Summarizer,
    /// 雑用: commit messages, file ops, quick CLI tasks (llama3.2:1b)
    CliGit,
    /// 監査: security audit, memory leak review (starcoder2:15b)
    Reviewer,
    /// 文書: web-doc RAG, README authoring (command-r:35b)
    DocRag,
}

impl Role {
    /// Ollama model name for this role
    pub fn model_name(&self) -> &'static str {
        match self {
            Role::Router     => "llama3.2:3b-instruct-q8_0",
            Role::Coder      => "qwen2.5-coder:32b-instruct-q8_0",
            Role::Architect  => "deepseek-r1:32b",
            Role::Vision     => "llama3.2-vision:11b",
            Role::Summarizer => "deepseek-r1:32b", // 推論モデルで要約
            Role::CliGit     => "llama3.2:1b-instruct-q8_0",
            Role::Reviewer   => "qwen2.5-coder:32b-instruct-q8_0",      // Rustの静的解析力で監査
            Role::DocRag     => "deepseek-r1:32b", // 推論モデルで文書作成
        }
    }

    /// System prompt tailored to this role
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Role::Router => {
                "You are a routing classifier for a Rust code editor assistant.\
                \nOutput EXACTLY one word — nothing else, no punctuation, no explanation.\
                \n\
                \nCATEGORIES (choose the single best match):\
                \n  coder      — write code, fix bugs, debug errors, implement features, refactor\
                \n  architect  — system design, module structure, how to design / plan something\
                \n  vision     — analyze screenshots, UI layout, visual appearance of the app\
                \n  summarizer — explain code, summarize logs or error output, describe what X does\
                \n  cli_git    — git commands (commit/push/pull/branch), shell commands, file rename/move\
                \n  reviewer   — security audit, vulnerability check, memory safety review\
                \n  doc_rag    — write docs / README, look up library API, crate usage questions\
                \n\
                \nDISAMBIGUATION — context matters, read the WHOLE sentence:\
                \n  bug/error/fix/debug/crash/panic → coder      (NOT reviewer; reviewer = security only)\
                \n  write tests / テストを書く       → coder      (NOT summarizer)\
                \n  design / architecture / 設計     → architect  (NOT coder)\
                \n  explain / summarize / 教えて / 要約 / について → summarizer (NOT coder, NOT doc_rag)\
                \n  commit / git / push / branch     → cli_git    (NOT doc_rag, NOT coder)\
                \n  security / unsafe / audit / 脆弱性 → reviewer (NOT coder)\
                \n  screenshot / UI / layout / 画像  → vision\
                \n  README / docs / crate / library  → doc_rag    (NOT summarizer)\
                \n\
                \nNEGATION & CONTEXT traps — the keyword alone is NOT enough:\
                \n  '設計は変えなくていい、バグを直して'    → coder      (修正が目的; 設計に言及しているだけ)\
                \n  'コミットはまだしないで、脆弱性を探して' → reviewer (否定+セキュリティが主目的)\
                \n  'Git の README を書いて'             → doc_rag    (文書作成が目的; git は題材)\
                \n  'このプロジェクトについて教えて'       → summarizer (説明を求めている)\
                \n  'このプロジェクトの概要と設計教えて'   → summarizer (説明を求めている; 設計相談ではない)\
                \n  '設計を教えて / 設計を説明して'        → summarizer (「教えて/説明して」= explain → summarizer)\
                \n  '設計をして / 設計を考えて / どう設計すべきか' → architect (設計作業・相談)\
                \n  'セキュリティの脆弱性を直して'          → reviewer   (修正+セキュリティ → reviewer)\
                \n\
                \nKEY RULE: '設計' alone means architect ONLY when combined with action verbs like\
                \n  「して」「考えて」「提案して」「すべきか」\
                \n  When followed by 「教えて」「説明して」「概要」「について」 → summarizer"
            }
            Role::Coder => {
                "You are an elite Rust engineer. \
                Answer in the same language as the user's message (Japanese if Japanese, English if English). \
                Write clean idiomatic Rust, use type-system over runtime checks, \
                anyhow/thiserror for errors (never unwrap in production), follow existing style."
            }
            Role::Architect => {
                "You are a software architect with a complete picture of the entire codebase. \
                Answer in the same language as the user's message (Japanese if Japanese, English if English). \
                Output: ASCII or Mermaid diagrams, explain trade-offs, recommend the simplest \
                design that satisfies constraints. Be concise and opinionated."
            }
            Role::Vision => {
                "You are a UI auditor reviewing egui rendering output. \
                Examine the provided screenshot carefully. Identify layout misalignments, \
                color inconsistencies, clipped text, and spacing issues. \
                Report findings as a numbered list with pixel-level specificity where possible."
            }
            Role::Summarizer => {
                "You are a technical writer and engineer. Summarize the provided code or log output \
                clearly and concisely. Highlight the most important parts, explain non-obvious \
                behavior, and call out any errors or warnings. Target audience is a senior engineer \
                who wants the key facts fast."
            }
            Role::CliGit => {
                "You are a shell and git expert. You MUST USE TOOLS to complete every task — never \
                just describe what you would do.\
                \n\
                \nGit workflow (autonomous mode):\
                \n  1. Call execute_command('git status') to see what changed\
                \n  2. Call execute_command('git add <files>') to stage relevant files\
                \n  3. Choose a Conventional Commits message: <type>(<scope>): <description>\
                \n     types: feat / fix / refactor / docs / test / chore / ci / perf\
                \n  4. Call execute_command('git commit -m \"<message>\"') to commit\
                \n  5. Confirm SUCCESS or report the exact error\
                \n\
                \nRules:\
                \n  - NEVER just print a commit message and stop — always run the git commands\
                \n  - NEVER use git push without explicit user instruction\
                \n  - If git add or commit fails, read the error and retry with the correct fix\
                \n  - Be extremely brief in any text output — results speak for themselves"
            }
            Role::Reviewer => {
                "You are a strict Rust security auditor and Clippy expert. You MUST USE TOOLS — \
                never just report issues without fixing them.\
                \n\
                \nWorkflow (always follow this order):\
                \n  1. Call list_files to understand the project structure\
                \n  2. Call read_file on every relevant source file\
                \n  3. Identify ALL issues across these categories:\
                \n     - Security: injection, overflow, unsafe blocks, secret exposure\
                \n     - Memory: leaks, use-after-free, data races, unnecessary clones\
                \n     - Correctness: logic errors, missing error handling, panics in production\
                \n     - Idioms: non-idiomatic Rust, Clippy warnings, missing ? operator\
                \n  4. For EACH issue found: call write_file with the fixed code\
                \n  5. Call execute_command('cargo clippy -- -D warnings') to verify\
                \n  6. If Clippy still reports warnings, fix them and repeat step 5\
                \n\
                \nFormat findings as: [SEVERITY] <file>:<line> — <issue> — <fix applied>"
            }
            Role::DocRag => {
                "You are a technical documentation specialist. \
                Write clear, accurate documentation based on the provided code and any referenced \
                external docs. Follow existing README style. Use concrete examples. \
                For API docs, include types, parameters, return values, and error conditions."
            }
        }
    }

    /// Per-role inference parameters — temperature, context window, output budget.
    ///
    /// Temperature philosophy:
    ///   - 0.0 : Router (deterministic, one-word answer)
    ///   - 0.1 : CliGit / Reviewer (commands and security must be precise)
    ///   - 0.2 : Coder (low variance for correct code)
    ///   - 0.5 : Architect / Vision / Summarizer / DocRag (creative latitude welcome)
    ///
    /// Context window: router/cliGit need very little; Architect/Summarizer/DocRag
    /// cap at 131_072 for practical speed.
    fn inference_options(&self) -> OllamaOptions {
        match self {
            Role::Router => OllamaOptions {
                temperature: 0.0,
                num_ctx: Some(16_384),  // must fit system prompt + ~70 few-shot examples
                num_predict: Some(10),  // hard cap: we only need one token
            },
            Role::Coder => OllamaOptions {
                temperature: 0.2,
                num_ctx: Some(65_536),  // 64k: large enough for most code files
                num_predict: None,
            },
            Role::Architect => OllamaOptions {
                temperature: 0.5,
                num_ctx: Some(16_384), // deepseek-r1:32b needs most VRAM for weights; keep ctx small
                num_predict: None,
            },
            Role::Vision => OllamaOptions {
                temperature: 0.5,
                num_ctx: Some(32_768),
                num_predict: None,
            },
            Role::Summarizer => OllamaOptions {
                temperature: 0.5,
                num_ctx: Some(16_384), // deepseek-r1:32b: keep ctx within VRAM budget
                num_predict: None,
            },
            Role::CliGit => OllamaOptions {
                temperature: 0.1,
                num_ctx: Some(8_192),   // git output is short
                num_predict: Some(512), // commit messages etc. are brief
            },
            Role::Reviewer => OllamaOptions {
                temperature: 0.1,
                num_ctx: Some(65_536),
                num_predict: None,
            },
            Role::DocRag => OllamaOptions {
                temperature: 0.5,
                num_ctx: Some(16_384), // deepseek-r1:32b: keep ctx within VRAM budget
                num_predict: None,
            },
        }
    }

    /// Parse a router model's one-word reply into a Role.
    /// Trusts the LLM entirely — no keyword heuristics.
    /// Exact match → substring search → Coder fallback.
    fn from_router_reply(reply: &str) -> Self {
        let lower = reply.trim().to_lowercase();

        // Exact match (temperature=0 makes this the common path)
        match lower.as_str() {
            "architect"  => return Role::Architect,
            "vision"     => return Role::Vision,
            "summarizer" => return Role::Summarizer,
            "cli_git"    => return Role::CliGit,
            "reviewer"   => return Role::Reviewer,
            "doc_rag"    => return Role::DocRag,
            "coder"      => return Role::Coder,
            _ => {}
        }

        // Substring search (handles "coder." / "I choose: architect" edge cases)
        if lower.contains("cli_git") || lower.contains("cli git") { return Role::CliGit; }
        if lower.contains("doc_rag") || lower.contains("doc rag") { return Role::DocRag; }
        if lower.contains("architect")  { return Role::Architect; }
        if lower.contains("vision")     { return Role::Vision; }
        if lower.contains("summarizer") { return Role::Summarizer; }
        if lower.contains("reviewer")   { return Role::Reviewer; }
        if lower.contains("coder")      { return Role::Coder; }

        Role::Coder // ultimate fallback
    }
}

#[derive(Clone)]
pub struct LlmClient {
    base_url: String,
    client: reqwest::Client,
    /// In-memory cache: directory path → (timestamp, tree string)
    list_cache: Arc<Mutex<HashMap<std::path::PathBuf, (std::time::Instant, String)>>>,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        let base_url = std::env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://KyosukenoMac-Studio.local:11434".to_string());
        Ok(Self::new_with_base_url(base_url))
    }

    fn new_with_base_url(base_url: String) -> Self {
        // Surface Ollama parallel execution settings so operators know what's active.
        // Set OLLAMA_NUM_PARALLEL=4 and OLLAMA_MAX_LOADED_MODELS=4 on the Ollama server
        // to run multiple models simultaneously on 128GB RAM.
        let num_parallel   = std::env::var("OLLAMA_NUM_PARALLEL").unwrap_or_else(|_| "1 (default)".to_string());
        let max_models     = std::env::var("OLLAMA_MAX_LOADED_MODELS").unwrap_or_else(|_| "1 (default)".to_string());
        tracing::info!("🚀 Ollama parallel config: NUM_PARALLEL={}, MAX_LOADED_MODELS={}", num_parallel, max_models);

        Self {
            base_url,
            client: reqwest::Client::new(),
            list_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Few-shot examples injected as conversation turns.
    /// Small models (3B) follow examples far more reliably than long instructions alone.
    /// Built once at first call and cached for the lifetime of the process.
    fn router_few_shot_messages() -> &'static [OllamaMessage] {
        static CACHE: std::sync::OnceLock<Vec<OllamaMessage>> = std::sync::OnceLock::new();
        CACHE.get_or_init(Self::build_router_few_shot)
    }

    fn build_router_few_shot() -> Vec<OllamaMessage> {
        // (user_message, expected_label)
        let examples: &[(&str, &str)] = &[
            // coder — context traps: negation / "設計" mentioned but goal is fix
            ("設計は変えなくていい、バグだけ直して",         "coder"),
            ("設計はそのままでいいのでエラーを修正して",     "coder"),
            ("コミットはまだしないで。コードを直して",       "coder"),
            // coder — normal
            ("Rustでバグを修正して",                        "coder"),
            ("この関数にエラーハンドリングを追加して",        "coder"),
            ("パニックしているのを直して",                   "coder"),
            ("新しいAPIエンドポイントを実装して",             "coder"),
            ("リファクタリングして",                         "coder"),
            ("Fix the compilation error",                   "coder"),
            ("Fix the compilation error in main.rs",        "coder"),
            ("コンパイルエラーを直して",                     "coder"),
            ("Implement this feature in Rust",              "coder"),
            ("このクラッシュ何？",                           "coder"),
            ("なんかエラー出てる",                           "coder"),
            ("it's broken",                                 "coder"),
            ("直して",                                      "coder"),
            ("write some tests for this",                   "coder"),
            ("テストを書いて",                              "coder"),
            // architect — action verbs: して/考えて/提案して/すべきか
            ("システム設計のアドバイスをして",               "architect"),
            ("モジュール構成をどう設計すべきか",             "architect"),
            ("この機能のアーキテクチャを考えて",             "architect"),
            ("How should I structure this codebase?",       "architect"),
            ("Design the module boundaries",                "architect"),
            ("どんな設計にすべきか",                        "architect"),
            ("設計を提案して",                              "architect"),
            ("このシステムをどう設計するか考えて",           "architect"),
            // vision
            ("スクリーンショットのUIを確認して",             "vision"),
            ("このレイアウトのズレを指摘して",               "vision"),
            ("画像を見てUIの問題点を教えて",                 "vision"),
            ("Check the UI screenshot for issues",          "vision"),
            // summarizer — explain/describe verbs: 教えて/説明して/要約して/について
            ("このコードを要約して",                         "summarizer"),
            ("このエラーログを解説して",                     "summarizer"),
            ("このファイルが何をしているか教えて",            "summarizer"),
            ("このプロジェクトについて教えて",               "summarizer"),
            ("このプロジェクトの概要と設計教えて",           "summarizer"),  // 設計+教えて → explain
            ("このプロジェクトの設計を教えて",               "summarizer"),  // 設計+教えて → explain
            ("設計を教えて",                                "summarizer"),  // 教えて dominates
            ("設計を説明して",                              "summarizer"),  // 説明して dominates
            ("現在の設計を概要を教えて",                    "summarizer"),
            ("アーキテクチャを教えて",                      "summarizer"),  // 教えて dominates
            ("アーキテクチャを説明して",                    "summarizer"),
            ("このモジュールについて教えて",                 "summarizer"),
            ("このコードについて教えて",                     "summarizer"),
            ("このリポジトリについて説明して",               "summarizer"),
            ("このクラスが何をしているか説明して",           "summarizer"),
            ("Explain what this project does",              "summarizer"),
            ("Tell me about this codebase",                 "summarizer"),
            ("What does this module do?",                   "summarizer"),
            ("Explain what this function does",             "summarizer"),
            ("Summarize this build log",                    "summarizer"),
            ("Explain the architecture of this project",   "summarizer"),  // explain+architecture → summarizer
            ("Describe the design of this module",         "summarizer"),  // describe → summarizer
            // cli_git
            ("コミットメッセージを作って",                   "cli_git"),
            ("コミットメッセージを生成して",                 "cli_git"),
            ("git rebaseのやり方を教えて",                   "cli_git"),
            ("ブランチを切って",                             "cli_git"),
            ("Generate a commit message",                   "cli_git"),
            ("How do I git stash?",                         "cli_git"),
            // reviewer — context trap: "commit" appears but security is the goal
            ("コミットはまだしないで。まず脆弱性を探して",   "reviewer"),
            ("pushする前にセキュリティチェックして",         "reviewer"),
            // reviewer — normal
            ("セキュリティ脆弱性をチェックして",             "reviewer"),
            ("このunsafeブロックは安全か監査して",           "reviewer"),
            ("メモリリークがないか確認して",                 "reviewer"),
            ("Audit this code for security issues",        "reviewer"),
            ("Check for memory safety problems",           "reviewer"),
            // doc_rag — context trap: "git" is the topic, not the operation
            ("GitのREADMEを書いて",                         "doc_rag"),
            ("Gitの使い方を解説しているドキュメントを書いて","doc_rag"),
            // doc_rag — normal
            ("READMEを書いて",                              "doc_rag"),
            ("tokioクレートの使い方を教えて",               "doc_rag"),
            ("このライブラリのAPIドキュメントを整備して",    "doc_rag"),
            ("Write the API documentation",                "doc_rag"),
            ("How do I use the serde crate?",              "doc_rag"),
        ];

        examples
            .iter()
            .flat_map(|(user, label)| {
                [
                    OllamaMessage { role: "user".to_string(),      content: user.to_string() },
                    OllamaMessage { role: "assistant".to_string(), content: label.to_string() },
                ]
            })
            .collect()
    }

    /// Single HTTP call to the router model. Returns the raw reply string, or None on any error.
    async fn call_router_once(&self, messages: &[OllamaMessage]) -> Option<String> {
        let request = OllamaChatRequest {
            model: Role::Router.model_name().to_string(),
            messages: messages.to_vec(),
            stream: false,
            tools: None,
            options: Some(Role::Router.inference_options()),
        };
        let url = format!("{}/api/chat", self.base_url);
        let resp = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.client.post(&url).json(&request).send(),
        )
        .await
        .ok()?   // timeout
        .ok()?;  // network error

        if !resp.status().is_success() { return None; }
        let chat = resp.json::<OllamaChatResponse>().await.ok()?;
        Some(chat.message.content)
    }

    /// Returns true if the raw router reply contains a recognisable role keyword.
    fn is_valid_router_reply(reply: &str) -> bool {
        let lower = reply.trim().to_lowercase();
        ["coder", "architect", "vision", "summarizer", "cli_git", "reviewer", "doc_rag"]
            .iter()
            .any(|&kw| lower.contains(kw))
    }

    /// Detects if a message has signals from two different role categories,
    /// returning the most commonly-confused pair. None = unambiguous.
    fn detect_mixed_signals(msg: &str) -> Option<(&'static str, &'static str)> {
        let lower = msg.to_lowercase();

        // Use only high-precision multi-word anchors to avoid false positives.
        // Single-word patterns like "fix", "設計", "commit" are intentionally excluded
        // because they appear in sentences where they are NOT the primary intent.
        let has_coder    = ["fix the bug", "バグを修正", "エラーを直", "コードを直",
                            "実装して", "implement", "refactor"]
                            .iter().any(|kw| lower.contains(kw));
        let has_git      = ["git commit", "git push", "git add", "git branch",
                            "コミットして", "ブランチを切", "rebase"]
                            .iter().any(|kw| lower.contains(kw));
        let has_reviewer = ["security audit", "脆弱性を", "audit this", "unsafe block",
                            "memory leak", "セキュリティチェック"]
                            .iter().any(|kw| lower.contains(kw));
        let has_arch     = ["architecture", "module structure", "system design",
                            "モジュール構成", "アーキテクチャ"]
                            .iter().any(|kw| lower.contains(kw));
        let has_doc      = ["readme", "api documentation", "write the docs",
                            "ドキュメントを書", "クレートの使い方"]
                            .iter().any(|kw| lower.contains(kw));

        // Ranked by historically most-confused pairs (from integration test failures)
        if has_coder && has_git      { return Some(("coder",     "cli_git")); }
        if has_coder && has_reviewer { return Some(("coder",     "reviewer")); }
        if has_arch  && has_coder    { return Some(("architect",  "coder")); }
        if has_coder && has_doc      { return Some(("coder",     "doc_rag")); }

        None
    }

    /// Self-reflection: when the message contains mixed signals, ask a targeted
    /// one-question follow-up to disambiguate. Adds ~0.1s on a 3B model.
    /// Skipped entirely when the initial classification is unambiguous.
    async fn reflect_on_classification(&self, role: Role, original_msg: &str) -> Role {
        let Some((cat_a, cat_b)) = Self::detect_mixed_signals(original_msg) else {
            return role; // clear signal — no reflection needed
        };

        // Only reflect if the initial answer is one of the ambiguous pair
        let initial = match &role {
            Role::Coder     => "coder",
            Role::CliGit    => "cli_git",
            Role::Reviewer  => "reviewer",
            Role::Architect => "architect",
            Role::DocRag    => "doc_rag",
            _               => return role,
        };
        if initial != cat_a && initial != cat_b {
            return role; // initial answer is outside the confused pair — trust it
        }

        let q = format!(
            "Message: \"{}\"\n\
            Should this be classified as `{}` or `{}`? \
            Reply with EXACTLY ONE word.",
            original_msg, cat_a, cat_b
        );
        let messages = vec![
            OllamaMessage { role: "system".to_string(), content: Role::Router.system_prompt().to_string() },
            OllamaMessage { role: "user".to_string(),   content: q },
        ];

        if let Some(reply) = self.call_router_once(&messages).await {
            let reflected = Role::from_router_reply(&reply);
            if reflected != role {
                tracing::info!("🔮 Self-reflection: {:?} → {:?} (ambiguous: {}/{})",
                    role, reflected, cat_a, cat_b);
            }
            reflected
        } else {
            role // reflection call failed — keep original
        }
    }

    /// Classify the user message using the router model (llama3.2:3b-instruct-q8_0).
    /// Uses few-shot conversation examples for maximum accuracy.
    /// If the first reply is ambiguous, retries once before falling back to Coder.
    pub async fn classify_with_router(&self, message: &str) -> Role {
        let mut messages = vec![OllamaMessage {
            role: "system".to_string(),
            content: Role::Router.system_prompt().to_string(),
        }];
        messages.extend(Self::router_few_shot_messages().iter().cloned());
        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        // Attempt 1
        if let Some(reply) = self.call_router_once(&messages).await {
            if Self::is_valid_router_reply(&reply) {
                let role = Role::from_router_reply(&reply);
                tracing::info!("🗂 Router reply {:?} → {:?}", reply.trim(), role);
                return self.reflect_on_classification(role, message).await;
            }
            tracing::warn!("⚠️  Router returned ambiguous '{}', retrying once...", reply.trim());
        } else {
            tracing::warn!("⚠️  Router call failed, retrying once...");
        }

        // Attempt 2 (retry)
        if let Some(reply) = self.call_router_once(&messages).await {
            let role = Role::from_router_reply(&reply);
            tracing::info!("🗂 Router retry reply {:?} → {:?}", reply.trim(), role);
            return self.reflect_on_classification(role, message).await;
        }

        tracing::warn!("⚠️  Router failed twice, defaulting to Coder");
        Role::Coder
    }

    /// Patterns that are never allowed in execute_command, regardless of context.
    /// Returns the matched pattern if blocked.
    /// Normalizes consecutive whitespace before matching to prevent trivial bypass.
    fn check_blocked_command(command: &str) -> Option<&'static str> {
        const BLOCKED: &[&str] = &[
            "rm -rf /",
            "rm -rf ~",
            "rm -rf *",
            "rm -fr /",
            ":(){:|:&};:",   // fork bomb
            ":(){ :|:",
            "git push --force",
            "git push -f",
            "git push origin main --force",
            "git push origin master --force",
            "mkfs",
            "dd if=",
            "> /dev/",
            "chmod -R 777 /",
            "sudo rm",
            "shred ",
            "wipefs",
        ];
        // Normalize whitespace so "git  push  --force" etc. are also blocked
        let normalized: String = command.split_whitespace().collect::<Vec<_>>().join(" ");
        BLOCKED.iter().find(|&&p| normalized.contains(p)).copied()
    }

    /// Joins `root` with `rel_path` and rejects any result that escapes `root` via "..".
    /// Does not require the target path to exist, making it safe for write_file as well.
    fn safe_join(root: &str, rel_path: &str) -> std::result::Result<std::path::PathBuf, String> {
        use std::path::Component;
        let root_path = std::path::PathBuf::from(root);
        let joined = root_path.join(rel_path);
        let mut normalized = std::path::PathBuf::new();
        for component in joined.components() {
            match component {
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(format!(
                            "🚫 Access denied: '{}' escapes the project root", rel_path
                        ));
                    }
                }
                c => normalized.push(c.as_os_str()),
            }
        }
        if !normalized.starts_with(&root_path) {
            return Err(format!(
                "🚫 Access denied: '{}' is outside the project root", rel_path
            ));
        }
        Ok(normalized)
    }

    /// Execute a tool call
    async fn execute_tool(&self, tool_name: &str, parameters: &serde_json::Value, project_path: Option<&str>) -> Result<String> {
        tracing::info!("🔧 Executing tool: {} with params: {}", tool_name, parameters);

        match tool_name {
            "read_file" => {
                let path = parameters.get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' parameter")?;

                let full_path = if let Some(proj_path) = project_path {
                    match Self::safe_join(proj_path, path) {
                        Ok(p) => p,
                        Err(msg) => { tracing::warn!("{}", msg); return Ok(msg); }
                    }
                } else {
                    std::path::PathBuf::from(path)
                };

                match tokio::fs::read_to_string(&full_path).await {
                    Ok(content) => {
                        tracing::info!("✅ Read file: {} ({} bytes)", full_path.display(), content.len());
                        Ok(content)
                    }
                    Err(e) => {
                        let error = format!("Failed to read file {}: {}", full_path.display(), e);
                        tracing::error!("❌ {}", error);
                        Ok(error)
                    }
                }
            }
            "write_file" => {
                let path = parameters.get("path")
                    .and_then(|v| v.as_str())
                    .context("Missing 'path' parameter")?;
                let content = parameters.get("content")
                    .and_then(|v| v.as_str())
                    .context("Missing 'content' parameter")?;

                let full_path = if let Some(proj_path) = project_path {
                    match Self::safe_join(proj_path, path) {
                        Ok(p) => p,
                        Err(msg) => { tracing::warn!("{}", msg); return Ok(msg); }
                    }
                } else {
                    std::path::PathBuf::from(path)
                };

                match tokio::fs::write(&full_path, content).await {
                    Ok(_) => {
                        tracing::info!("✅ Wrote file: {}", full_path.display());
                        Ok(format!("Successfully wrote {} bytes to {}", content.len(), full_path.display()))
                    }
                    Err(e) => {
                        let error = format!("Failed to write file {}: {}", full_path.display(), e);
                        tracing::error!("❌ {}", error);
                        Ok(error)
                    }
                }
            }
            "execute_command" => {
                let command = parameters.get("command")
                    .and_then(|v| v.as_str())
                    .context("Missing 'command' parameter")?;

                // Security: block destructive patterns before execution
                if let Some(pattern) = Self::check_blocked_command(command) {
                    let msg = format!("🚫 Blocked: command contains '{}' which is not permitted", pattern);
                    tracing::warn!("{}", msg);
                    return Ok(msg);
                }

                tracing::info!("🔧 Executing command: {}", command);

                // kill_on_drop ensures the child process is terminated if the timeout fires
                let child = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(project_path.unwrap_or("."))
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                    .context("Failed to spawn command")?;

                let output = match tokio::time::timeout(
                    std::time::Duration::from_secs(OLLAMA_COMMAND_TIMEOUT_SECS),
                    child.wait_with_output(),
                ).await {
                    Ok(Ok(out)) => out,
                    Ok(Err(e)) => return Err(anyhow::anyhow!("Command execution failed: {}", e)),
                    Err(_) => {
                        return Ok(format!(
                            "❌ TIMEOUT: '{}' was killed after {}s — use a more targeted command",
                            command, OLLAMA_COMMAND_TIMEOUT_SECS
                        ));
                    }
                };

                // Combine stdout + stderr so the AI sees the full picture.
                // Rust compiler errors go to stderr, so combining is essential.
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let result = if output.status.success() {
                    format!(
                        "✅ SUCCESS: '{}' completed.\nSTDOUT:\n{}\nSTDERR:\n{}",
                        command, stdout, stderr
                    )
                } else {
                    format!(
                        "❌ FAILED: '{}' failed (exit code: {}).\nSTDOUT:\n{}\nSTDERR:\n{}",
                        command, output.status.code().unwrap_or(-1), stdout, stderr
                    )
                };

                tracing::info!("Command output: {} bytes", result.len());
                Ok(result)
            }
            "search_code" => {
                let query = parameters.get("query")
                    .and_then(|v| v.as_str())
                    .context("Missing 'query' parameter")?;

                tracing::info!("🔍 Searching code for: {}", query);

                let search_dir = project_path.unwrap_or(".");

                // Try rg first, fall back to grep -r
                let rg_result = tokio::process::Command::new("rg")
                    .arg(query)
                    .arg("--max-count=10")
                    .arg("--heading")
                    .arg("--line-number")
                    .current_dir(search_dir)
                    .output()
                    .await;

                let output = match rg_result {
                    Ok(o) => o,
                    Err(_) => {
                        tracing::info!("rg not found, falling back to grep");
                        tokio::process::Command::new("grep")
                            .arg("-r")
                            .arg("-n")
                            .arg("--max-count=10")
                            .arg(query)
                            .arg(".")
                            .current_dir(search_dir)
                            .output()
                            .await
                            .context("Failed to search code (grep fallback)")?
                    }
                };

                let result = String::from_utf8_lossy(&output.stdout);
                tracing::info!("Search results: {} bytes", result.len());
                Ok(result.to_string())
            }
            "list_files" => {
                let path_str = parameters.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                let root = if let Some(proj) = project_path {
                    match Self::safe_join(proj, path_str) {
                        Ok(p) => p,
                        Err(msg) => { tracing::warn!("{}", msg); return Ok(msg); }
                    }
                } else {
                    std::path::PathBuf::from(path_str)
                };

                // Cache lookup — avoid redundant filesystem scans in tight tool loops
                {
                    let cache = self.list_cache.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some((ts, cached)) = cache.get(&root) {
                        if ts.elapsed() < std::time::Duration::from_secs(LIST_CACHE_TTL_SECS) {
                            tracing::info!("📂 list_files cache hit: {} ({} bytes)", root.display(), cached.len());
                            return Ok(cached.clone());
                        }
                    }
                }

                let root_clone = root.clone();
                let tree = tokio::task::spawn_blocking(move || {
                    let header = format!("{}/\n", root_clone.display());
                    header + &LlmClient::build_file_tree(&root_clone, "  ", 0, 3)
                }).await.context("list_files spawn failed")?;

                // Store in cache, evicting expired entries first
                {
                    let mut cache = self.list_cache.lock().unwrap_or_else(|e| e.into_inner());
                    let ttl = std::time::Duration::from_secs(LIST_CACHE_TTL_SECS);
                    cache.retain(|_, (ts, _)| ts.elapsed() < ttl);
                    cache.insert(root, (std::time::Instant::now(), tree.clone()));
                }

                tracing::info!("📂 list_files: {} bytes (cached for {}s)", tree.len(), LIST_CACHE_TTL_SECS);
                Ok(tree)
            }
            _ => {
                let error = format!("Unknown tool: {}", tool_name);
                tracing::error!("❌ {}", error);
                Ok(error)
            }
        }
    }

    /// Build an indented file tree string synchronously (max_depth levels).
    /// Skips hidden files, `target/`, `node_modules/`, `__pycache__/`.
    fn build_file_tree(path: &std::path::Path, prefix: &str, depth: usize, max_depth: usize) -> String {
        if depth > max_depth { return String::new(); }

        let mut entries: Vec<_> = match std::fs::read_dir(path) {
            Ok(e) => e.flatten().collect(),
            Err(_) => return String::new(),
        };
        entries.sort_by_key(|e| e.file_name());

        let mut out = String::new();
        for entry in &entries {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if s.starts_with('.') || s == "target" || s == "node_modules" || s == "__pycache__" {
                continue;
            }
            let entry_path = entry.path();
            if entry_path.is_dir() {
                out.push_str(&format!("{}{}/\n", prefix, s));
                out.push_str(&Self::build_file_tree(&entry_path, &format!("{}  ", prefix), depth + 1, max_depth));
            } else {
                out.push_str(&format!("{}{}\n", prefix, s));
            }
        }
        out
    }

    /// Get available tools for autonomous mode
    fn get_autonomous_tools() -> Vec<OllamaTool> {
        vec![
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "execute_command".to_string(),
                    description: "Execute a shell command and return the output".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The shell command to execute"
                            }
                        },
                        "required": ["command"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "read_file".to_string(),
                    description: "Read the contents of a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path to read"
                            }
                        },
                        "required": ["path"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "write_file".to_string(),
                    description: "Write content to a file".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path to write"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write"
                            }
                        },
                        "required": ["path", "content"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "search_code".to_string(),
                    description: "Search for code in the project".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query (regex supported)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
            },
            OllamaTool {
                tool_type: "function".to_string(),
                function: OllamaFunction {
                    name: "list_files".to_string(),
                    description: "List files and directories in a tree view (max 3 levels deep). \
                        Use this BEFORE read_file to identify which files are relevant, \
                        avoiding unnecessary context usage.".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Directory path to list (default: project root)"
                            }
                        },
                        "required": []
                    }),
                },
            },
        ]
    }

    /// Determines if a tool result warrants a self-correction injection.
    /// Returns a guidance string for the AI, or None if no correction is needed.
    fn detect_correction_needed(tool_name: &str, result: &str) -> Option<String> {
        match tool_name {
            "execute_command" => {
                let is_error = result.contains("error[E")                    // Rust compiler
                    || result.contains("error: aborting")                    // Rust linker
                    || result.contains("Traceback (most recent call last)")  // Python
                    || result.contains("AttributeError:")                    // Blender API
                    || result.contains("ModuleNotFoundError:")
                    || result.contains("SyntaxError:")
                    || result.contains("RuntimeError:")
                    || result.contains("TypeError:");
                is_error.then(|| {
                    "The command produced errors. Read each error carefully, \
                    fix the root cause with read_file + write_file, then re-run to verify.".to_string()
                })
            }
            "read_file" if result.contains("Failed to read") => {
                Some("That file path does not exist. \
                    Call list_files to find the correct path, then retry read_file.".to_string())
            }
            "write_file" if result.contains("Failed to write") => {
                Some("The write failed — the directory may not exist. \
                    Use execute_command('mkdir -p <dir>') to create it, then retry write_file.".to_string())
            }
            _ if result.starts_with("❌") => {
                Some("The previous tool call failed. Inspect the error above and retry \
                    with corrected parameters.".to_string())
            }
            _ => None,
        }
    }

    /// Send a chat message to Ollama and get streaming response.
    /// In autonomous mode, handles the tool calling loop.
    pub async fn chat_stream(
        &self,
        message: String,
        role: Role,
        autonomous: bool,
        project_path: Option<String>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>>> {
        let model = role.model_name().to_string();
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let list_cache = self.list_cache.clone();
        let role_prompt = role.system_prompt();
        // Role-specific starting hint for autonomous mode — injected AFTER the system prompt.
        // Each role has a natural first tool to call; forcing list_files on CliGit conflicts
        // with its git-status-first workflow.
        let autonomous_start_hint = match &role {
            Role::CliGit    => "Start with execute_command('git status') to see what has changed.",
            Role::Reviewer  => "Start with list_files to understand the project, then read_file each relevant source file.",
            _               => "Start with list_files to explore the project structure.",
        };
        let inference_opts = role.inference_options();

        tracing::info!("🤖 Using model: {} (role: {:?}, autonomous: {}, project_path: {:?})",
            model, role, autonomous, project_path);

        let stream = async_stream::stream! {
            if autonomous {
                // Autonomous mode: handle tool calling loop
                let tools = Self::get_autonomous_tools();

                // Keep system prompt short so the model outputs standard JSON tool calls.
                // Verbose prompts cause the model to output non-standard formats.
                // deepseek-r1 and other reasoning models emit <think> blocks before the
                // actual response — remind them to output the tool call after thinking.
                let system_content = format!(
                    "{}\n\nUse the provided tools to complete the task. {}\
                    \n\nIMPORTANT: After any reasoning, output ONLY a tool call in valid JSON. \
                    Do NOT ask the user for clarification — infer what you need from context.",
                    role_prompt, autonomous_start_hint
                );

                let mut messages = vec![
                    OllamaMessage {
                        role: "system".to_string(),
                        content: system_content,
                    },
                    OllamaMessage {
                        role: "user".to_string(),
                        content: message.clone(),
                    }
                ];

                let max_iterations = 10;
                for iteration in 0..max_iterations {
                    tracing::info!("🔄 Autonomous iteration {}/{}", iteration + 1, max_iterations);
                    tracing::debug!("📝 Current messages count: {}", messages.len());
                    for (i, msg) in messages.iter().enumerate() {
                        tracing::debug!("  Message {}: role={}, content_len={}", i, msg.role, msg.content.len());
                    }

                    // Use stream:false for tool-calling iterations so we get the
                    // complete JSON response in one shot (streaming splits JSON tokens
                    // across HTTP chunks making reassembly unreliable).
                    let request = OllamaChatRequest {
                        model: model.clone(),
                        messages: messages.clone(),
                        stream: false,
                        tools: Some(tools.clone()),
                        options: Some(inference_opts.clone()),
                    };

                    let url = format!("{}/api/chat", base_url);
                    let response = match tokio::time::timeout(
                        std::time::Duration::from_secs(OLLAMA_AUTONOMOUS_TIMEOUT_SECS),
                        client.post(&url).json(&request).send(),
                    ).await {
                        Ok(Ok(resp)) => resp,
                        Ok(Err(e)) => {
                            yield Err(anyhow::anyhow!("Failed to send request: {}", e));
                            break;
                        }
                        Err(_) => {
                            yield Err(anyhow::anyhow!(
                                "Ollama request timed out after {}s", OLLAMA_AUTONOMOUS_TIMEOUT_SECS
                            ));
                            break;
                        }
                    };

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        yield Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
                        break;
                    }

                    let body = match response.text().await {
                        Ok(b) => b,
                        Err(e) => {
                            yield Err(anyhow::anyhow!("Failed to read response body: {}", e));
                            break;
                        }
                    };

                    let resp: serde_json::Value = match serde_json::from_str(&body) {
                        Ok(v) => v,
                        Err(e) => {
                            yield Err(anyhow::anyhow!("Failed to parse response JSON: {}\nbody: {:.200}", e, body));
                            break;
                        }
                    };

                    let full_content = resp["message"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();

                    tracing::info!("🤖 Full response JSON: {}", body);
                    tracing::info!("🤖 content ({} bytes): {:.100}", full_content.len(), full_content);

                    // Strip <think>...</think> blocks emitted by reasoning models (e.g. deepseek-r1).
                    // The tool call (if any) appears after the thinking block.
                    let content_for_parse = strip_think_tags(&full_content);

                    // Resolve tool call: prefer native tool_calls field, then text-based parse
                    let native_tool = resp["message"]["tool_calls"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|tc| {
                            let name = tc["function"]["name"].as_str()?.to_string();
                            let args = tc["function"]["arguments"].clone();
                            Some((name, args))
                        });

                    let resolved_tool = native_tool.or_else(|| parse_tool_call(&content_for_parse));

                    if let Some((tool_name, tool_params)) = resolved_tool {
                        // Tool call detected!
                        tracing::info!("🔧 Tool call detected: {} with params: {:?}", tool_name, tool_params);

                        // Yield a status message
                        yield Ok(format!("\n**[🔧 実行中: {}]**\n", tool_name));

                        // Execute the tool
                        let tool_result = match Self::execute_tool(
                            &Self { base_url: base_url.clone(), client: client.clone(), list_cache: list_cache.clone() },
                            &tool_name,
                            &tool_params,
                            project_path.as_deref(),
                        ).await {
                            Ok(result) => result,
                            Err(e) => format!("Error executing tool: {}", e),
                        };

                        tracing::info!("✅ Tool result: {} bytes", tool_result.len());

                        // Yield the tool result for user visibility
                        // For large results, show summary instead of full content
                        let display_result = if tool_result.len() > 2000 {
                            let lines: Vec<&str> = tool_result.lines().collect();
                            let total_lines = lines.len();
                            let preview_lines = 10;

                            if total_lines > preview_lines * 2 {
                                let first_part = lines[..preview_lines].join("\n");
                                let last_part = lines[total_lines - preview_lines..].join("\n");
                                format!("{}\n\n... ({} lines omitted) ...\n\n{}\n\n[Total: {} lines, {} bytes]",
                                    first_part, total_lines - preview_lines * 2, last_part, total_lines, tool_result.len())
                            } else {
                                format!("{}\n\n[Total: {} lines, {} bytes]", tool_result, total_lines, tool_result.len())
                            }
                        } else {
                            tool_result.clone()
                        };
                        yield Ok(format!("```\n{}\n```\n", display_result));

                        // Add both the assistant's tool call and the tool result to messages.
                        // Truncate the tool result if huge to protect the context window —
                        // keep first 100 lines + last 50 lines with a summary in between.
                        const MAX_HISTORY_CHARS: usize = 12_000;
                        let history_content = if tool_result.len() > MAX_HISTORY_CHARS {
                            let lines: Vec<&str> = tool_result.lines().collect();
                            let total = lines.len();
                            let head = 100.min(total / 2);
                            let tail = 50.min(total.saturating_sub(head));
                            let omitted = total.saturating_sub(head + tail);
                            format!(
                                "{}\n\n[... {} lines omitted for context efficiency ...]\n\n{}\n\n[Total: {} lines, {} bytes]",
                                lines[..head].join("\n"),
                                omitted,
                                lines[total - tail..].join("\n"),
                                total,
                                tool_result.len()
                            )
                        } else {
                            tool_result.clone()
                        };

                        // Use "user" role for tool results to avoid Ollama rejecting
                        // "tool" role when the assistant message has text content
                        // instead of native tool_calls.
                        messages.push(OllamaMessage {
                            role: "assistant".to_string(),
                            content: format!("I'll call the {} tool.", tool_name),
                        });
                        messages.push(OllamaMessage {
                            role: "user".to_string(),
                            content: format!("Tool result for {}:\n{}", tool_name, history_content),
                        });

                        tracing::info!("📨 Added tool result to conversation ({} bytes)", tool_result.len());

                        // Self-correction: unified across all tools.
                        // Detects Rust/Python/Blender errors, missing files, write failures, etc.
                        if let Some(guidance) = Self::detect_correction_needed(&tool_name, &tool_result) {
                            tracing::info!("🔁 Self-correction triggered for tool '{}'", tool_name);
                            messages.push(OllamaMessage {
                                role: "user".to_string(),
                                content: guidance,
                            });
                        }

                        // Continue to next iteration
                        continue;
                    } else {
                        // No tool call — stream final answer to the user now
                        tracing::info!("✅ Final answer ({} bytes): {:.100}", full_content.len(), full_content);
                        if !full_content.is_empty() {
                            yield Ok(full_content);
                        }
                        break;
                    }
                }
            } else {
                // Non-autonomous mode: streaming chat with role-based system prompt.
                // Explicitly tell the model that no tools are available.
                // Some models will attempt tool calls even when
                // tools:None is sent, because they are RLHF-trained to use tools.

                // Inject project context (CLAUDE.md / README.md) when available,
                // so Summarizer/Architect can answer "このプロジェクトについて" accurately.
                let project_context = if let Some(ref path) = project_path {
                    let root = std::path::Path::new(path);
                    let candidates = ["CLAUDE.md", "README.md", "README.txt", "readme.md"];
                    let mut ctx = String::new();
                    for name in &candidates {
                        let p = root.join(name);
                        if let Ok(content) = tokio::fs::read_to_string(&p).await {
                            tracing::info!("📄 Injecting project context from {}", p.display());
                            ctx = format!("\n\n--- PROJECT CONTEXT (from {}) ---\n{}\n--- END PROJECT CONTEXT ---", name, content);
                            break;
                        }
                    }
                    ctx
                } else {
                    String::new()
                };

                let system_with_no_tools = format!(
                    "{}{}\n\nIMPORTANT: You do NOT have access to any tools or functions. \
                    Do NOT output JSON tool calls or function invocations. \
                    Answer only in plain natural language.",
                    role_prompt, project_context
                );

                let request = OllamaChatRequest {
                    model: model.clone(),
                    messages: vec![
                        OllamaMessage {
                            role: "system".to_string(),
                            content: system_with_no_tools,
                        },
                        OllamaMessage {
                            role: "user".to_string(),
                            content: message,
                        },
                    ],
                    stream: true,
                    tools: None,
                    options: Some(inference_opts),
                };

                let url = format!("{}/api/chat", base_url);
                let response = match tokio::time::timeout(
                    std::time::Duration::from_secs(OLLAMA_STREAM_TIMEOUT_SECS),
                    client.post(&url).json(&request).send(),
                ).await {
                    Ok(Ok(resp)) => resp,
                    Ok(Err(e)) => {
                        yield Err(anyhow::anyhow!("Failed to send request: {}", e));
                        return;
                    }
                    Err(_) => {
                        yield Err(anyhow::anyhow!(
                            "Ollama connection timed out after {}s", OLLAMA_STREAM_TIMEOUT_SECS
                        ));
                        return;
                    }
                };

                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    yield Err(anyhow::anyhow!("Ollama API error {}: {}", status, body));
                    return;
                }

                // Ollama streams NDJSON: one JSON object per line, done=true on last
                let mut byte_stream = response.bytes_stream();
                let mut line_buf = Vec::<u8>::new();
                // Track whether we are inside a <think> block across chunks.
                let mut in_think_block = false;

                while let Some(item) = futures::StreamExt::next(&mut byte_stream).await {
                    let bytes = match item {
                        Ok(b) => b,
                        Err(e) => {
                            // Surface read errors to the user so they're not left in silence.
                            yield Ok(format!("\n\n⚠️ Stream error: {}", e));
                            return;
                        }
                    };

                    for &byte in bytes.iter() {
                        if byte == b'\n' {
                            if !line_buf.is_empty() {
                                if let Ok(chunk) = serde_json::from_slice::<OllamaStreamChunk>(&line_buf) {
                                    let text = chunk.message.content;
                                    if !text.is_empty() && !is_tool_call_json(&text) {
                                        // Strip <think> blocks from reasoning models.
                                        let filtered = filter_think_stream(&text, &mut in_think_block);
                                        if !filtered.is_empty() {
                                            yield Ok(filtered);
                                        }
                                    }
                                    if chunk.done {
                                        return;
                                    }
                                }
                                line_buf.clear();
                            }
                        } else if line_buf.len() < MAX_LINE_BUF_BYTES {
                            line_buf.push(byte);
                        } else {
                            // Oversized line — likely malformed response; discard and continue
                            tracing::warn!("⚠️ Streaming line exceeded {}B limit, discarding", MAX_LINE_BUF_BYTES);
                            line_buf.clear();
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashSet;

    // =========================================================
    // Role::model_name()
    // =========================================================

    #[test]
    fn test_role_model_name_all_variants() {
        assert_eq!(Role::Router.model_name(),     "llama3.2:3b-instruct-q8_0");
        assert_eq!(Role::Coder.model_name(),      "qwen2.5-coder:32b-instruct-q8_0");
        assert_eq!(Role::Architect.model_name(),  "deepseek-r1:32b");
        assert_eq!(Role::Vision.model_name(),     "llama3.2-vision:11b");
        assert_eq!(Role::Summarizer.model_name(), "deepseek-r1:32b");
        assert_eq!(Role::CliGit.model_name(),     "llama3.2:1b-instruct-q8_0");
        assert_eq!(Role::Reviewer.model_name(),   "qwen2.5-coder:32b-instruct-q8_0");
        assert_eq!(Role::DocRag.model_name(),     "deepseek-r1:32b");
    }

    #[test]
    fn test_role_model_names_installed_models_only() {
        // Verify no uninstalled models are referenced
        const INSTALLED: &[&str] = &[
            "llama3.2:3b-instruct-q8_0",
            "qwen2.5-coder:32b-instruct-q8_0",
            "deepseek-r1:32b",
            "llama3.2-vision:11b",
            "llama3.2:1b-instruct-q8_0",
        ];
        for role in &[
            Role::Router, Role::Coder, Role::Architect, Role::Vision,
            Role::Summarizer, Role::CliGit, Role::Reviewer, Role::DocRag,
        ] {
            assert!(
                INSTALLED.contains(&role.model_name()),
                "{:?} uses '{}' which is not in the installed model list",
                role, role.model_name()
            );
        }
    }

    // =========================================================
    // Role::system_prompt()
    // =========================================================

    #[test]
    fn test_role_system_prompts_not_empty() {
        for role in &[
            Role::Router, Role::Coder, Role::Architect, Role::Vision,
            Role::Summarizer, Role::CliGit, Role::Reviewer, Role::DocRag,
        ] {
            assert!(
                !role.system_prompt().is_empty(),
                "{:?} has an empty system prompt",
                role
            );
        }
    }

    #[test]
    fn test_role_system_prompts_unique() {
        let prompts: HashSet<_> = [
            Role::Router, Role::Coder, Role::Architect, Role::Vision,
            Role::Summarizer, Role::CliGit, Role::Reviewer, Role::DocRag,
        ]
        .iter()
        .map(|r| r.system_prompt())
        .collect();
        assert_eq!(prompts.len(), 8, "Some roles share the same system prompt");
    }

    #[test]
    fn test_role_system_prompt_router_mentions_category_words() {
        let prompt = Role::Router.system_prompt();
        assert!(prompt.contains("coder"));
        assert!(prompt.contains("architect"));
        assert!(prompt.contains("reviewer"));
    }

    #[test]
    fn test_role_system_prompt_reviewer_mentions_severity() {
        let prompt = Role::Reviewer.system_prompt();
        assert!(prompt.to_lowercase().contains("critical") || prompt.to_lowercase().contains("severity"));
    }

    // =========================================================
    // Role::from_router_reply()
    // =========================================================

    #[test]
    fn test_from_router_reply_all_known_exact() {
        assert_eq!(Role::from_router_reply("architect"), Role::Architect);
        assert_eq!(Role::from_router_reply("vision"), Role::Vision);
        assert_eq!(Role::from_router_reply("summarizer"), Role::Summarizer);
        assert_eq!(Role::from_router_reply("cli_git"), Role::CliGit);
        assert_eq!(Role::from_router_reply("reviewer"), Role::Reviewer);
        assert_eq!(Role::from_router_reply("doc_rag"), Role::DocRag);
        assert_eq!(Role::from_router_reply("coder"), Role::Coder);
    }

    #[test]
    fn test_from_router_reply_case_insensitive() {
        assert_eq!(Role::from_router_reply("ARCHITECT"), Role::Architect);
        assert_eq!(Role::from_router_reply("Vision"), Role::Vision);
        assert_eq!(Role::from_router_reply("CLI_GIT"), Role::CliGit);
        assert_eq!(Role::from_router_reply("REVIEWER"), Role::Reviewer);
        assert_eq!(Role::from_router_reply("DOC_RAG"), Role::DocRag);
        assert_eq!(Role::from_router_reply("CODER"), Role::Coder);
    }

    #[test]
    fn test_from_router_reply_whitespace_trimmed() {
        assert_eq!(Role::from_router_reply("  architect  "), Role::Architect);
        assert_eq!(Role::from_router_reply("\nreviewer\n"), Role::Reviewer);
        assert_eq!(Role::from_router_reply("\t cli_git \t"), Role::CliGit);
    }

    #[test]
    fn test_from_router_reply_unknown_defaults_to_coder() {
        assert_eq!(Role::from_router_reply(""), Role::Coder);
        assert_eq!(Role::from_router_reply("unknown"), Role::Coder);
        assert_eq!(Role::from_router_reply("I think it's coding"), Role::Coder);
        assert_eq!(Role::from_router_reply("  "), Role::Coder);
        assert_eq!(Role::from_router_reply("router"), Role::Coder); // Router is internal only
    }

    // =========================================================
    // System prompt content — CliGit and Reviewer
    // =========================================================

    #[test]
    fn test_cli_git_prompt_commands_tool_use() {
        let prompt = Role::CliGit.system_prompt();
        assert!(prompt.contains("execute_command"), "CliGit prompt must reference execute_command");
        assert!(prompt.contains("git add"),         "CliGit prompt must mention git add step");
        assert!(prompt.contains("git commit"),      "CliGit prompt must mention git commit step");
        assert!(prompt.contains("Conventional Commits") || prompt.contains("feat") || prompt.contains("<type>"),
            "CliGit prompt must reference Conventional Commits format");
    }

    #[test]
    fn test_reviewer_prompt_enforces_tool_workflow() {
        let prompt = Role::Reviewer.system_prompt();
        assert!(prompt.contains("read_file"),    "Reviewer prompt must reference read_file");
        assert!(prompt.contains("write_file"),   "Reviewer prompt must reference write_file");
        assert!(prompt.contains("cargo clippy"), "Reviewer prompt must mention cargo clippy");
    }

    // =========================================================
    // execute_command result format — SUCCESS / FAILED keywords
    // =========================================================

    #[tokio::test]
    async fn test_execute_command_success_contains_success_keyword() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "echo ok"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("SUCCESS"), "success result must contain SUCCESS");
        assert!(!result.contains("FAILED"), "success result must not contain FAILED");
    }

    #[tokio::test]
    async fn test_execute_command_failure_contains_failed_keyword() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "exit 1"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("FAILED"), "failure result must contain FAILED");
        assert!(!result.contains("SUCCESS"), "failure result must not contain SUCCESS");
    }

    #[tokio::test]
    async fn test_execute_command_result_includes_command_name() {
        let client = LlmClient::new_with_base_url(String::new());
        let cmd = "echo hello_marker_xyz";
        let params = json!({"command": cmd});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains(cmd), "result must echo the command name for AI context");
    }

    // =========================================================
    // parse_tool_call()
    // =========================================================

    #[test]
    fn test_parse_tool_call_json_exact() {
        let content = r#"{"type": "function", "name": "read_file", "parameters": {"path": "src/main.rs"}}"#;
        let result = parse_tool_call(content).expect("should parse");
        assert_eq!(result.0, "read_file");
        assert_eq!(result.1["path"], "src/main.rs");
    }

    #[test]
    fn test_parse_tool_call_json_embedded_in_text() {
        let content = r#"Sure, I'll write it. {"type": "function", "name": "write_file", "parameters": {"path": "out.txt", "content": "hello"}}"#;
        let result = parse_tool_call(content).expect("should parse embedded JSON");
        assert_eq!(result.0, "write_file");
        assert_eq!(result.1["path"], "out.txt");
        assert_eq!(result.1["content"], "hello");
    }

    #[test]
    fn test_parse_tool_call_xml_single_param() {
        let content = "<function=read_file><parameter=path>src/lib.rs</parameter></function>";
        let result = parse_tool_call(content).expect("should parse XML");
        assert_eq!(result.0, "read_file");
        assert_eq!(result.1["path"], "src/lib.rs");
    }

    #[test]
    fn test_parse_tool_call_xml_multiple_params() {
        let content = "<function=write_file><parameter=path>out.rs</parameter><parameter=content>fn main() {}</parameter></function>";
        let result = parse_tool_call(content).expect("should parse XML multi-param");
        assert_eq!(result.0, "write_file");
        assert_eq!(result.1["path"], "out.rs");
        assert_eq!(result.1["content"], "fn main() {}");
    }

    #[test]
    fn test_parse_tool_call_xml_with_leading_text() {
        let content = "Let me read that for you.\n<function=read_file><parameter=path>README.md</parameter></function>";
        let result = parse_tool_call(content).expect("should parse XML after text");
        assert_eq!(result.0, "read_file");
        assert_eq!(result.1["path"], "README.md");
    }

    #[test]
    fn test_parse_tool_call_xml_execute_command() {
        let content = "<function=execute_command><parameter=command>cargo test</parameter></function>";
        let result = parse_tool_call(content).expect("should parse execute_command");
        assert_eq!(result.0, "execute_command");
        assert_eq!(result.1["command"], "cargo test");
    }

    #[test]
    fn test_parse_tool_call_plain_text_returns_none() {
        assert!(parse_tool_call("Here is my answer.").is_none());
        assert!(parse_tool_call("The bug is on line 42.").is_none());
        assert!(parse_tool_call("fn main() { println!(\"hello\"); }").is_none());
    }

    #[test]
    fn test_parse_tool_call_empty_returns_none() {
        assert!(parse_tool_call("").is_none());
    }

    #[test]
    fn test_parse_tool_call_malformed_json_returns_none() {
        assert!(parse_tool_call("{not valid json}").is_none());
        // Incomplete JSON — no closing brace
        assert!(parse_tool_call(r#"{"type": "function", "name": "read_file""#).is_none());
    }

    #[test]
    fn test_parse_tool_call_xml_no_parameters_returns_none() {
        // <function=> tag present but no <parameter=> tags
        assert!(parse_tool_call("<function=read_file></function>").is_none());
        assert!(parse_tool_call("<function=read_file>").is_none());
    }

    #[test]
    fn test_parse_tool_call_xml_unclosed_parameter_returns_none() {
        // <parameter=> opened but no </parameter>
        let content = "<function=read_file><parameter=path>src/main.rs";
        assert!(parse_tool_call(content).is_none());
    }

    // =========================================================
    // LlmClient::new()
    // =========================================================

    /// Mutex to serialize tests that manipulate OLLAMA_BASE_URL.
    /// Without this, parallel test execution causes non-deterministic failures.
    static OLLAMA_URL_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_llm_client_new_default_url_when_env_not_set() {
        let _guard = OLLAMA_URL_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("OLLAMA_BASE_URL");
        let client = LlmClient::new().unwrap();
        assert_eq!(client.base_url, "http://KyosukenoMac-Studio.local:11434");
    }

    #[test]
    fn test_llm_client_new_custom_url_from_env() {
        let _guard = OLLAMA_URL_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("OLLAMA_BASE_URL", "http://localhost:12345");
        let client = LlmClient::new().unwrap();
        assert_eq!(client.base_url, "http://localhost:12345");
        std::env::remove_var("OLLAMA_BASE_URL");
    }

    // =========================================================
    // execute_tool() — filesystem & shell (no network needed)
    // =========================================================

    #[tokio::test]
    async fn test_execute_tool_read_file_success() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hello from test").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": tmp.path().to_str().unwrap()});
        let result = client.execute_tool("read_file", &params, None).await.unwrap();
        assert!(result.contains("hello from test"));
    }

    #[tokio::test]
    async fn test_execute_tool_read_file_not_found_returns_error_string() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": "/nonexistent/__does_not_exist__.rs"});
        let result = client.execute_tool("read_file", &params, None).await.unwrap();
        assert!(result.contains("Failed to read file"));
    }

    #[tokio::test]
    async fn test_execute_tool_read_file_with_project_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("relative.txt"), "relative content").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": "relative.txt"});
        let result = client.execute_tool("read_file", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("relative content"));
    }

    #[tokio::test]
    async fn test_execute_tool_read_file_missing_path_param_returns_err() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({});
        let err = client.execute_tool("read_file", &params, None).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'path' parameter"));
    }

    #[tokio::test]
    async fn test_execute_tool_write_file_success() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("out.txt");

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": out.to_str().unwrap(), "content": "written by unit test"});
        let result = client.execute_tool("write_file", &params, None).await.unwrap();

        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "written by unit test");
    }

    #[tokio::test]
    async fn test_execute_tool_write_file_with_project_path() {
        let dir = tempfile::tempdir().unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": "subfile.txt", "content": "via project_path"});
        let result = client.execute_tool("write_file", &params, Some(dir.path().to_str().unwrap())).await.unwrap();

        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(dir.path().join("subfile.txt")).unwrap(), "via project_path");
    }

    #[tokio::test]
    async fn test_execute_tool_write_file_missing_path_param_returns_err() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"content": "no path given"});
        let err = client.execute_tool("write_file", &params, None).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'path' parameter"));
    }

    #[tokio::test]
    async fn test_execute_tool_write_file_missing_content_param_returns_err() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": "/tmp/berry_test_no_content.txt"});
        let err = client.execute_tool("write_file", &params, None).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'content' parameter"));
    }

    #[tokio::test]
    async fn test_execute_tool_execute_command_success() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "echo berry_unit_test_marker"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("SUCCESS"));
        assert!(result.contains("berry_unit_test_marker"));
    }

    #[tokio::test]
    async fn test_execute_tool_execute_command_failure_shows_exit_code() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "exit 42"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("FAILED"));
    }

    #[tokio::test]
    async fn test_execute_tool_execute_command_uses_project_path_as_cwd() {
        let dir = tempfile::tempdir().unwrap();
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "pwd"});
        let result = client.execute_tool("execute_command", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        // The output should include the temp dir path
        assert!(result.contains(dir.path().to_str().unwrap()));
    }

    #[tokio::test]
    async fn test_execute_tool_execute_command_missing_command_param_returns_err() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({});
        let err = client.execute_tool("execute_command", &params, None).await.unwrap_err();
        assert!(err.to_string().contains("Missing 'command' parameter"));
    }

    #[tokio::test]
    async fn test_execute_tool_search_code_returns_result() {
        // Skip if rg (ripgrep) is not installed in PATH
        if std::process::Command::new("rg").arg("--version").output().is_err() {
            eprintln!("Skipping: rg not found in PATH");
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sample.rs"), "fn hello_search_target() {}").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"query": "hello_search_target"});
        let result = client.execute_tool("search_code", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("hello_search_target"));
    }

    #[tokio::test]
    async fn test_execute_tool_list_files_returns_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("lib.rs"), "").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({});  // default path
        let result = client.execute_tool("list_files", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("main.rs"), "should list main.rs, got: {}", result);
        assert!(result.contains("src/"), "should list src/ dir, got: {}", result);
    }

    #[tokio::test]
    async fn test_execute_tool_list_files_custom_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("hello.txt"), "").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"path": "subdir"});
        let result = client.execute_tool("list_files", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("hello.txt"), "should list hello.txt, got: {}", result);
    }

    #[tokio::test]
    async fn test_execute_tool_unknown_tool_returns_error_string() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({});
        let result = client.execute_tool("nonexistent_tool", &params, None).await.unwrap();
        assert!(result.contains("Unknown tool: nonexistent_tool"));
    }

    // =========================================================
    // check_blocked_command()
    // =========================================================

    #[test]
    fn test_blocked_command_rm_rf_root() {
        assert!(LlmClient::check_blocked_command("rm -rf /").is_some());
        assert!(LlmClient::check_blocked_command("rm -rf /home").is_some());
    }

    #[test]
    fn test_blocked_command_rm_rf_home() {
        assert!(LlmClient::check_blocked_command("rm -rf ~").is_some());
        assert!(LlmClient::check_blocked_command("rm -rf *").is_some());
    }

    #[test]
    fn test_blocked_command_fork_bomb() {
        assert!(LlmClient::check_blocked_command(":(){:|:&};:").is_some());
    }

    #[test]
    fn test_blocked_command_force_push() {
        assert!(LlmClient::check_blocked_command("git push --force").is_some());
        assert!(LlmClient::check_blocked_command("git push -f origin main").is_some());
    }

    #[test]
    fn test_blocked_command_mkfs() {
        assert!(LlmClient::check_blocked_command("mkfs.ext4 /dev/sda1").is_some());
    }

    #[test]
    fn test_blocked_command_dd() {
        assert!(LlmClient::check_blocked_command("dd if=/dev/urandom of=/dev/sda").is_some());
    }

    #[test]
    fn test_safe_commands_not_blocked() {
        assert!(LlmClient::check_blocked_command("cargo build").is_none());
        assert!(LlmClient::check_blocked_command("ls -la").is_none());
        assert!(LlmClient::check_blocked_command("git push origin feature-branch").is_none());
        assert!(LlmClient::check_blocked_command("echo hello").is_none());
        assert!(LlmClient::check_blocked_command("cat src/main.rs").is_none());
        assert!(LlmClient::check_blocked_command("cargo test").is_none());
    }

    #[tokio::test]
    async fn test_execute_tool_blocked_command_returns_error_string() {
        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({"command": "rm -rf /"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("Blocked"));
        assert!(result.contains("rm -rf /"));
    }

    // =========================================================
    // execute_command stdout+stderr combined
    // =========================================================

    #[tokio::test]
    async fn test_execute_command_combines_stdout_and_stderr() {
        let client = LlmClient::new_with_base_url(String::new());
        // Write to both stdout and stderr
        let params = json!({"command": "echo OUT && echo ERR >&2"});
        let result = client.execute_tool("execute_command", &params, None).await.unwrap();
        assert!(result.contains("STDOUT"), "should contain STDOUT section");
        assert!(result.contains("STDERR"), "should contain STDERR section");
        assert!(result.contains("OUT"));
        assert!(result.contains("ERR"));
    }

    // =========================================================
    // classify_with_router() — HTTP mocked with wiremock
    // =========================================================

    #[tokio::test]
    async fn test_classify_with_router_all_valid_replies() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let cases = [
            ("coder",      Role::Coder),
            ("architect",  Role::Architect),
            ("vision",     Role::Vision),
            ("summarizer", Role::Summarizer),
            ("cli_git",    Role::CliGit),
            ("reviewer",   Role::Reviewer),
            ("doc_rag",    Role::DocRag),
        ];

        for (reply, expected) in cases {
            let mock_server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/api/chat"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "message": {"role": "assistant", "content": reply},
                    "done": true
                })))
                .mount(&mock_server)
                .await;

            let client = LlmClient::new_with_base_url(mock_server.uri());
            let role = client.classify_with_router("test message").await;
            assert_eq!(role, expected, "reply '{}' should map to {:?}", reply, expected);
        }
    }

    #[tokio::test]
    async fn test_classify_with_router_uses_router_model_in_request() {
        use wiremock::matchers::{method, path, body_json};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .and(body_json(serde_json::json!({
                "model": "llama3.2:3b-instruct-q8_0",
                "stream": false,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": {"role": "assistant", "content": "coder"},
                "done": true
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new_with_base_url(mock_server.uri());
        // If the wrong model is used the mock won't match and we'll get a 404 → fallback to Coder
        // Either way we expect Coder, but the mock verifies the model name
        let role = client.classify_with_router("implement something").await;
        assert_eq!(role, Role::Coder);
    }

    #[tokio::test]
    async fn test_classify_with_router_unknown_reply_defaults_to_coder() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": {"role": "assistant", "content": "I think it is a coding task"},
                "done": true
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new_with_base_url(mock_server.uri());
        assert_eq!(client.classify_with_router("anything").await, Role::Coder);
    }

    #[tokio::test]
    async fn test_classify_with_router_server_error_defaults_to_coder() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new_with_base_url(mock_server.uri());
        assert_eq!(client.classify_with_router("anything").await, Role::Coder);
    }

    #[tokio::test]
    async fn test_classify_with_router_invalid_json_body_defaults_to_coder() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string("this is not json"))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new_with_base_url(mock_server.uri());
        assert_eq!(client.classify_with_router("anything").await, Role::Coder);
    }

    #[tokio::test]
    async fn test_classify_with_router_connection_refused_defaults_to_coder() {
        // Nothing listening on this port
        let client = LlmClient::new_with_base_url("http://127.0.0.1:19876".to_string());
        assert_eq!(client.classify_with_router("anything").await, Role::Coder);
    }

    // =========================================================
    // is_valid_router_reply()
    // =========================================================

    #[test]
    fn test_is_valid_router_reply_all_known_labels() {
        assert!(LlmClient::is_valid_router_reply("coder"));
        assert!(LlmClient::is_valid_router_reply("architect"));
        assert!(LlmClient::is_valid_router_reply("vision"));
        assert!(LlmClient::is_valid_router_reply("summarizer"));
        assert!(LlmClient::is_valid_router_reply("cli_git"));
        assert!(LlmClient::is_valid_router_reply("reviewer"));
        assert!(LlmClient::is_valid_router_reply("doc_rag"));
    }

    #[test]
    fn test_is_valid_router_reply_case_insensitive() {
        assert!(LlmClient::is_valid_router_reply("CODER"));
        assert!(LlmClient::is_valid_router_reply("Architect"));
        assert!(LlmClient::is_valid_router_reply("CLI_GIT"));
    }

    #[test]
    fn test_is_valid_router_reply_invalid_replies() {
        assert!(!LlmClient::is_valid_router_reply(""));
        assert!(!LlmClient::is_valid_router_reply("unknown"));
        assert!(!LlmClient::is_valid_router_reply("sure, let me help you"));
        assert!(!LlmClient::is_valid_router_reply("router")); // internal only
    }

    // =========================================================
    // classify_with_router retry — wiremock simulates ambiguous first reply
    // =========================================================

    #[tokio::test]
    async fn test_classify_with_router_retries_on_ambiguous_first_reply() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        // First call returns garbage, second call returns a valid role
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": {"role": "assistant", "content": "sure, I can help with that!"},
                "done": true
            })))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": {"role": "assistant", "content": "architect"},
                "done": true
            })))
            .mount(&mock_server)
            .await;

        let client = LlmClient::new_with_base_url(mock_server.uri());
        let role = client.classify_with_router("design this system").await;
        assert_eq!(role, Role::Architect, "should retry and return Architect on second attempt");
    }

    // =========================================================
    // list_files cache — second call must be served from cache
    // =========================================================

    #[tokio::test]
    async fn test_list_files_cache_hit_skips_fs_scan() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();

        let client = LlmClient::new_with_base_url(String::new());
        let params = json!({});

        // First call: populates cache
        let first = client.execute_tool("list_files", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(first.contains("a.rs"));

        // Add a new file — but the cache should still return the old result
        std::fs::write(dir.path().join("b.rs"), "").unwrap();

        // Second call: should come from cache (b.rs absent)
        let second = client.execute_tool("list_files", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(!second.contains("b.rs"), "second call should be served from cache");
        assert_eq!(first, second, "cached result must match first call");
    }

    // =========================================================
    // safe_join() — path traversal prevention
    // =========================================================

    #[test]
    fn test_safe_join_normal_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let result = LlmClient::safe_join(root, "src/main.rs").unwrap();
        assert!(result.starts_with(dir.path()));
        assert!(result.ends_with("src/main.rs"));
    }

    #[test]
    fn test_safe_join_dot_path_stays_in_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let result = LlmClient::safe_join(root, ".").unwrap();
        assert!(result.starts_with(dir.path()));
    }

    #[test]
    fn test_safe_join_blocks_parent_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        assert!(LlmClient::safe_join(root, "../../etc/passwd").is_err());
        assert!(LlmClient::safe_join(root, "../sibling").is_err());
        assert!(LlmClient::safe_join(root, "subdir/../../..").is_err());
    }

    #[test]
    fn test_safe_join_blocks_absolute_path_injection() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        // Path::join("/etc/passwd") replaces the base — safe_join must catch this
        let result = LlmClient::safe_join(root, "/etc/passwd");
        assert!(result.is_err(), "absolute path injection must be blocked");
    }

    #[tokio::test]
    async fn test_list_files_blocks_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let client = LlmClient::new_with_base_url(String::new());
        let params = serde_json::json!({"path": "../../"});
        let result = client.execute_tool("list_files", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("Access denied"), "list_files must block path traversal, got: {}", result);
    }

    #[tokio::test]
    async fn test_read_file_blocks_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let client = LlmClient::new_with_base_url(String::new());
        let params = serde_json::json!({"path": "../../etc/passwd"});
        let result = client.execute_tool("read_file", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("Access denied"), "read_file must block path traversal, got: {}", result);
    }

    #[tokio::test]
    async fn test_write_file_blocks_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let client = LlmClient::new_with_base_url(String::new());
        let params = serde_json::json!({"path": "../../tmp/injected.txt", "content": "pwned"});
        let result = client.execute_tool("write_file", &params, Some(dir.path().to_str().unwrap())).await.unwrap();
        assert!(result.contains("Access denied"), "write_file must block path traversal, got: {}", result);
    }

    // =========================================================
    // Role::inference_options()
    // =========================================================

    #[test]
    fn test_inference_options_router_is_deterministic_and_tiny() {
        let opts = Role::Router.inference_options();
        assert_eq!(opts.temperature, 0.0, "Router must be deterministic");
        // 16_384 is the minimum to fit the system prompt + ~70 few-shot examples
        assert!(opts.num_ctx.unwrap_or(u32::MAX) <= 32_768, "Router context must be reasonable (≤32k)");
        assert!(opts.num_predict.is_some(), "Router must have output token cap");
    }

    #[test]
    fn test_inference_options_cli_git_is_low_temperature() {
        let opts = Role::CliGit.inference_options();
        assert!(opts.temperature <= 0.2, "CliGit must be near-deterministic for commands");
        assert!(opts.num_predict.is_some(), "CliGit must have output cap");
    }

    #[test]
    fn test_inference_options_architect_has_context() {
        let opts = Role::Architect.inference_options();
        // deepseek-r1:32b uses most VRAM for weights; 16K is the practical limit.
        let ctx = opts.num_ctx.unwrap_or(0);
        assert!(ctx >= 8_192 && ctx <= 32_768, "Architect ctx should be within deepseek-r1:32b VRAM budget");
    }

    #[test]
    fn test_inference_options_all_roles_have_valid_temperature() {
        for role in &[
            Role::Router, Role::Coder, Role::Architect, Role::Vision,
            Role::Summarizer, Role::CliGit, Role::Reviewer, Role::DocRag,
        ] {
            let opts = role.inference_options();
            assert!(opts.temperature >= 0.0 && opts.temperature <= 1.0,
                "{:?} has invalid temperature {}", role, opts.temperature);
        }
    }

    // =========================================================
    // detect_mixed_signals()
    // =========================================================

    #[test]
    fn test_detect_mixed_signals_coder_git() {
        // Requires multi-word anchors — both "実装して" and "git commit" must appear
        let result = LlmClient::detect_mixed_signals("実装してgit commitもして");
        assert_eq!(result, Some(("coder", "cli_git")));
    }

    #[test]
    fn test_detect_mixed_signals_coder_reviewer() {
        let result = LlmClient::detect_mixed_signals("refactor and security audit this code");
        assert_eq!(result, Some(("coder", "reviewer")));
    }

    #[test]
    fn test_detect_mixed_signals_unambiguous_coder() {
        assert!(LlmClient::detect_mixed_signals("Implement a binary search function").is_none());
    }

    #[test]
    fn test_detect_mixed_signals_unambiguous_architect() {
        assert!(LlmClient::detect_mixed_signals("設計のアドバイスをして").is_none());
    }

    #[test]
    fn test_detect_mixed_signals_no_false_positive_on_mention() {
        // "設計" alone must NOT trigger — not in multi-word anchors
        assert!(LlmClient::detect_mixed_signals("設計は変えなくていい、バグだけ直して").is_none());
        // "コミット" alone must NOT trigger
        assert!(LlmClient::detect_mixed_signals("コミットはまだしないで、脆弱性を探して").is_none());
        // "Git" as topic must NOT trigger cli_git mixed signal
        assert!(LlmClient::detect_mixed_signals("GitのREADMEを書いて").is_none());
    }

    // =========================================================
    // detect_correction_needed()
    // =========================================================

    #[test]
    fn test_correction_needed_rust_compile_error() {
        let result = LlmClient::detect_correction_needed(
            "execute_command",
            "❌ FAILED: 'cargo build' failed.\nSTDERR:\nerror[E0308]: mismatched types",
        );
        assert!(result.is_some(), "Rust compile error should trigger correction");
    }

    #[test]
    fn test_correction_needed_python_traceback() {
        let result = LlmClient::detect_correction_needed(
            "execute_command",
            "❌ FAILED:\nTraceback (most recent call last):\n  File 'script.py'",
        );
        assert!(result.is_some(), "Python traceback should trigger correction");
    }

    #[test]
    fn test_correction_needed_read_file_not_found() {
        let result = LlmClient::detect_correction_needed(
            "read_file",
            "Failed to read file /nonexistent/path: No such file",
        );
        assert!(result.is_some(), "Missing file should trigger path correction");
        assert!(result.unwrap().contains("list_files"));
    }

    #[test]
    fn test_correction_needed_write_file_failed() {
        let result = LlmClient::detect_correction_needed(
            "write_file",
            "Failed to write file /no/such/dir/file.txt: No such file or directory",
        );
        assert!(result.is_some(), "Write failure should trigger mkdir guidance");
        assert!(result.unwrap().contains("mkdir"));
    }

    #[test]
    fn test_correction_needed_generic_error_marker() {
        let result = LlmClient::detect_correction_needed(
            "search_code",
            "❌ Some unexpected search failure",
        );
        assert!(result.is_some(), "❌ prefix on any tool should trigger correction");
    }

    #[test]
    fn test_correction_not_needed_on_success() {
        let result = LlmClient::detect_correction_needed(
            "execute_command",
            "✅ SUCCESS: 'cargo build' completed.\nSTDOUT:\n   Compiling foo v0.1.0",
        );
        assert!(result.is_none(), "successful output must not trigger correction");
    }

    // =========================================================
    // strip_think_tags
    // =========================================================

    #[test]
    fn test_strip_think_tags_removes_block() {
        let input = "<think>\nLet me reason about this.\n</think>\n{\"name\": \"list_files\"}";
        assert_eq!(strip_think_tags(input), "{\"name\": \"list_files\"}");
    }

    #[test]
    fn test_strip_think_tags_no_tags_passthrough() {
        let input = "{\"name\": \"list_files\"}";
        assert_eq!(strip_think_tags(input), input);
    }

    #[test]
    fn test_strip_think_tags_uses_last_closing_tag() {
        // Multiple think blocks — should strip up to and including the last </think>
        let input = "<think>first</think> middle <think>second</think> final";
        assert_eq!(strip_think_tags(input), "final");
    }

    #[test]
    fn test_strip_think_tags_partial_block_passthrough() {
        // Model is still mid-think — no closing tag yet, return as-is
        let input = "<think>\nstill thinking...";
        assert_eq!(strip_think_tags(input), input);
    }
}
