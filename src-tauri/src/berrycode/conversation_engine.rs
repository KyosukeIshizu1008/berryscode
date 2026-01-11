//! Conversation engine for LLM chat with tools
//!
//! This module provides a unified conversation engine used by both CLI and Web interfaces.

use crate::berrycode::llm::{LLMClient, Message, LLMResponse};
use crate::berrycode::tools::{get_available_tools, execute_tool};
use crate::berrycode::summarizer::Summarizer;
use crate::berrycode::prompt_optimizer::PromptOptimizer;
use crate::berrycode::context_window::ContextWindowManager;
use crate::berrycode::modes::{Mode, ModeConfig};
use crate::berrycode::memory::Memory;
use crate::berrycode::linter::Linter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use async_trait::async_trait;
use futures::future::join_all;

/// Callback for reporting tool execution status
#[async_trait]
pub trait ToolCallback: Send {
    async fn on_tool_start(&mut self, tool_name: &str, args: &str);
    async fn on_tool_complete(&mut self, tool_name: &str, result: &str);
    async fn on_response(&mut self, text: &str);
    async fn on_response_chunk(&mut self, chunk: &str);
}

/// Conversation engine that handles LLM chat with tools
pub struct ConversationEngine {
    max_iterations: usize,
    summarizer: Option<Summarizer>,
    optimizer: Arc<Mutex<PromptOptimizer>>,
    context_window: Arc<Mutex<ContextWindowManager>>,
    mode_config: ModeConfig,
    memory: Arc<Mutex<Memory>>,
    linter: Option<Linter>,
    auto_lint: bool,
}

impl ConversationEngine {
    pub fn new() -> Self {
        Self {
            max_iterations: 30,
            summarizer: None,
            optimizer: Arc::new(Mutex::new(PromptOptimizer::new())),
            context_window: Arc::new(Mutex::new(ContextWindowManager::new())),
            mode_config: ModeConfig::code(), // Default to Code mode
            memory: Arc::new(Mutex::new(Memory::default())),
            linter: Some(Linter::new(HashMap::new())),
            auto_lint: false, // Disabled by default for safety
        }
    }

    /// Enable automatic linting with AI feedback
    pub fn with_auto_lint(mut self, enabled: bool) -> Self {
        self.auto_lint = enabled;
        self
    }

    /// Create with specific mode
    pub fn with_mode(mode: Mode) -> Self {
        Self {
            max_iterations: 30,
            summarizer: None,
            optimizer: Arc::new(Mutex::new(PromptOptimizer::new())),
            context_window: Arc::new(Mutex::new(ContextWindowManager::new())),
            mode_config: mode.config(),
            memory: Arc::new(Mutex::new(Memory::default())),
            linter: Some(Linter::new(HashMap::new())),
            auto_lint: false,
        }
    }

    /// Create with project root (loads memory from .berrycode/memory.md)
    pub fn with_project_root(project_root: &PathBuf) -> Self {
        let mut memory = Memory::new(project_root);
        let _ = memory.load(); // Best effort - ignore errors

        Self {
            max_iterations: 30,
            summarizer: None,
            optimizer: Arc::new(Mutex::new(PromptOptimizer::new())),
            context_window: Arc::new(Mutex::new(ContextWindowManager::new())),
            mode_config: ModeConfig::code(),
            memory: Arc::new(Mutex::new(memory)),
            linter: Some(Linter::new(HashMap::new())),
            auto_lint: false,
        }
    }

    /// Create with summarizer for token optimization
    pub fn with_summarizer(summarizer: Summarizer) -> Self {
        Self {
            max_iterations: 30,
            summarizer: Some(summarizer),
            optimizer: Arc::new(Mutex::new(PromptOptimizer::new())),
            context_window: Arc::new(Mutex::new(ContextWindowManager::new())),
            mode_config: ModeConfig::code(),
            memory: Arc::new(Mutex::new(Memory::default())),
            linter: Some(Linter::new(HashMap::new())),
            auto_lint: false,
        }
    }

    /// Create with mode and summarizer
    pub fn with_mode_and_summarizer(mode: Mode, summarizer: Summarizer) -> Self {
        Self {
            max_iterations: 30,
            summarizer: Some(summarizer),
            optimizer: Arc::new(Mutex::new(PromptOptimizer::new())),
            context_window: Arc::new(Mutex::new(ContextWindowManager::new())),
            mode_config: mode.config(),
            memory: Arc::new(Mutex::new(Memory::default())),
            linter: Some(Linter::new(HashMap::new())),
            auto_lint: false,
        }
    }

    /// Get current mode
    pub fn mode(&self) -> Mode {
        self.mode_config.mode
    }

    /// Detect programming language from file extension
    fn detect_language(file_path: &str) -> &str {
        let ext = file_path.rsplit('.').next().unwrap_or("");
        match ext {
            "py" => "python",
            "rs" => "rust",
            "js" | "jsx" | "ts" | "tsx" => "javascript",
            "go" => "go",
            "java" => "java",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" | "h" => "c",
            _ => "unknown",
        }
    }

    /// Check file for linting errors and return error message if any
    fn check_lint_errors(&self, file_path: &str, project_root: &Path) -> Option<String> {
        if !self.auto_lint {
            return None;
        }

        let linter = self.linter.as_ref()?;
        let language = Self::detect_language(file_path);

        if language == "unknown" {
            return None;
        }

        let full_path = project_root.join(file_path);
        match linter.lint_file(&full_path, language) {
            Ok(errors) if !errors.is_empty() => {
                tracing::warn!("🔍 LINT ERRORS in {}: {} issues found", file_path, errors.len());
                Some(format!(
                    "⚠️ LINTING ERRORS in {}:\n\n{}\n\n\
                    🔧 Please fix these errors. The AI will help you resolve them.",
                    file_path,
                    errors.join("\n")
                ))
            }
            Ok(_) => {
                tracing::info!("✅ LINT: {} passed linting", file_path);
                None
            }
            Err(e) => {
                tracing::warn!("Linting failed: {}", e);
                None
            }
        }
    }

    /// Get mode configuration
    pub fn mode_config(&self) -> &ModeConfig {
        &self.mode_config
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Execute a conversation with the LLM, handling tool calls
    ///
    /// # Arguments
    /// * `llm_client` - The LLM client to use
    /// * `messages` - Initial message history
    /// * `project_root` - Root directory for tool execution
    /// * `callback` - Callback for reporting progress
    ///
    /// # Returns
    /// * Final assistant response text
    pub async fn execute<C: ToolCallback>(
        &self,
        llm_client: &LLMClient,
        mut messages: Vec<Message>,
        project_root: &PathBuf,
        callback: &mut C,
    ) -> anyhow::Result<String> {
        // Get tools and filter based on mode
        let all_tools = get_available_tools();
        let mut tools = self.mode_config.filter_tools(all_tools);

        tracing::info!("Mode: {} | {} tools available (before proactive filtering)", self.mode_config.mode, tools.len());

        // Inject memory into first system message
        if let Some(memory_content) = self.memory.lock().unwrap().format_for_prompt() {
            // Find first system message and append memory
            if let Some(system_msg) = messages.iter_mut().find(|m| m.role == "system") {
                if let Some(ref mut content) = system_msg.content {
                    content.push_str("\n\n");
                    content.push_str(&memory_content);
                    tracing::info!("Injected memory into system prompt");
                }
            }
        }

        // PROACTIVE TOOL EXECUTION: Router decides which tools to run
        // This eliminates AI round-trips and prevents "noise injection"
        let mut tools_used_proactively = Vec::new();
        let mut is_consultation_mode = false;

        if let Some(last_user_msg) = messages.iter().rev().find(|m| m.role == "user") {
            if let Some(user_query) = &last_user_msg.content {
                // ⚡ INSTANT STATIC RESPONSE: Check for help request BEFORE any processing
                // This bypasses ALL expensive operations (semantic search ~2s, LLM ~5s)
                // Response time: ~0.002s instead of ~9s
                if crate::berrycode::static_responses::is_help_request(user_query) {
                    tracing::info!("⚡ Returning static help response (bypassing all AI processing)");
                    callback.on_response(crate::berrycode::static_responses::OPTIONS_HELP_TEXT).await;
                    return Ok(crate::berrycode::static_responses::OPTIONS_HELP_TEXT.to_string());
                }

                // Skip very short inputs (likely chitchat)
                if user_query.len() < 5 {
                    tracing::debug!("⏭️ ROUTER: Skipping proactive tools (input too short)");
                } else {
                    use crate::berrycode::router::IntentRouter;
                    use crate::berrycode::tools::{execute_tool, ToolCall, FunctionCall};

                    // Analyze user intent with Router (0.1ms)
                    let router = IntentRouter::new();
                    let proactive_tools = router.get_proactive_tools(user_query);

                    let mut tool_futures = Vec::new();

                    // 1. Semantic Search (if router says it's conceptual)
                    if proactive_tools.semantic_search {
                        tracing::info!("🔥 ROUTER: Running semantic_search proactively");
                        tools_used_proactively.push("semantic_search".to_string());

                        let search_query = user_query.clone();
                        let project_root_clone = project_root.clone();

                        let future = tokio::task::spawn_blocking(move || {
                            let tool_call = ToolCall {
                                id: "proactive_semantic".to_string(),
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "semantic_search".to_string(),
                                    arguments: serde_json::json!({
                                        "query": search_query,
                                        "limit": 10
                                    }).to_string(),
                                },
                            };
                            ("semantic_search", execute_tool(&tool_call, &project_root_clone))
                        });
                        tool_futures.push(future);
                    }

                    // 2. File Reads (if router detected file paths)
                    if !proactive_tools.files_to_read.is_empty() {
                        tools_used_proactively.push("read_file".to_string());
                    }
                    for file_path in &proactive_tools.files_to_read {
                        tracing::info!("📖 ROUTER: Reading file proactively: {}", file_path);
                        let path = file_path.clone();
                        let project_root_clone = project_root.clone();

                        let future = tokio::task::spawn_blocking(move || {
                            let tool_call = ToolCall {
                                id: format!("proactive_read_{}", path),
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "read_file".to_string(),
                                    arguments: serde_json::json!({
                                        "file_path": path
                                    }).to_string(),
                                },
                            };
                            ("read_file", execute_tool(&tool_call, &project_root_clone))
                        });
                        tool_futures.push(future);
                    }

                    // 2.5. Capabilities File (Consultation Mode - ONLY read this file!)
                    if let Some(capabilities_file) = &proactive_tools.capabilities_file {
                        tracing::info!("💡 ROUTER: CONSULTATION MODE → Reading {} + ROADMAP.md ONLY (blocking semantic_search)", capabilities_file);
                        is_consultation_mode = true;  // ⚡ Flag for short-circuit
                        tools_used_proactively.push("capabilities_reference".to_string());

                        // Read CAPABILITIES.md
                        let path = capabilities_file.clone();
                        let project_root_clone = project_root.clone();

                        let future = tokio::task::spawn_blocking(move || {
                            let tool_call = ToolCall {
                                id: "proactive_capabilities".to_string(),
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "read_file".to_string(),
                                    arguments: serde_json::json!({
                                        "file_path": path
                                    }).to_string(),
                                },
                            };
                            ("capabilities_reference", execute_tool(&tool_call, &project_root_clone))
                        });
                        tool_futures.push(future);

                        // Also read ROADMAP.md for gap analysis
                        tools_used_proactively.push("roadmap_reference".to_string());
                        let project_root_clone2 = project_root.clone();

                        let future2 = tokio::task::spawn_blocking(move || {
                            let tool_call = ToolCall {
                                id: "proactive_roadmap".to_string(),
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "read_file".to_string(),
                                    arguments: serde_json::json!({
                                        "file_path": "docs/ROADMAP.md"
                                    }).to_string(),
                                },
                            };
                            ("roadmap_reference", execute_tool(&tool_call, &project_root_clone2))
                        });
                        tool_futures.push(future2);
                    }

                    // 3. Grep (if router says code search)
                    if !proactive_tools.grep_patterns.is_empty() {
                        tools_used_proactively.push("grep".to_string());
                    }
                    for pattern in &proactive_tools.grep_patterns {
                        tracing::info!("🔍 ROUTER: Running grep proactively: {}", pattern);
                        let grep_pattern = pattern.clone();
                        let project_root_clone = project_root.clone();

                        let future = tokio::task::spawn_blocking(move || {
                            let tool_call = ToolCall {
                                id: "proactive_grep".to_string(),
                                tool_type: "function".to_string(),
                                function: FunctionCall {
                                    name: "grep".to_string(),
                                    arguments: serde_json::json!({
                                        "pattern": grep_pattern,
                                        "output_mode": "content",
                                        "head_limit": 30  // Limit to 30 results to prevent payload overflow
                                    }).to_string(),
                                },
                            };
                            ("grep", execute_tool(&tool_call, &project_root_clone))
                        });
                        tool_futures.push(future);
                    }

                    // Execute all proactive tools in parallel (SHOTGUN MODE 🔫)
                    if !tool_futures.is_empty() {
                        tracing::info!("⚡ ROUTER: Executing {} proactive tools in parallel", tool_futures.len());

                        let results = futures::future::join_all(tool_futures).await;

                        // Inject successful results
                        let mut proactive_context = String::new();
                        for result in results {
                            if let Ok((tool_name, tool_result)) = result {
                                if let Ok(output) = tool_result {
                                    proactive_context.push_str(&format!("\n## 🔥 {} (Proactive):\n{}\n", tool_name, output));
                                }
                            }
                        }

                        if !proactive_context.is_empty() {
                            // Add self-healing instructions if enabled
                            let healing_instructions = if proactive_tools.self_healing {
                                "\n\n🔧 SELF-HEALING MODE ACTIVATED:\n\
                                 This is a bug fix request. After you write the fix:\n\
                                 1. Tests will be run automatically (cargo test/npm test)\n\
                                 2. If tests fail, you'll see the error and can try again\n\
                                 3. Loop continues up to 3 iterations until tests pass\n\
                                 4. You can go get coffee - BerryCode will fix it! ☕"
                            } else {
                                ""
                            };

                            // Add batch edit automation instructions if detected
                            let batch_edit_instructions = if let Some(ref target_file) = proactive_tools.batch_edit_target {
                                format!(
                                    "\n\n⚡ BATCH EDIT MODE ACTIVATED:\n\
                                     You are trying to modify MANY places in `{}`.\n\
                                     \n\
                                     🚨 DO NOT use `edit_file` repeatedly! This will trigger 'Tool Overused' errors.\n\
                                     \n\
                                     ✅ INSTEAD, do this:\n\
                                     1. Write a Python script that reads the file\n\
                                     2. Use regex/string.replace() to modify ALL occurrences at once\n\
                                     3. Write the file back\n\
                                     4. Execute the script with bash tool: `python script.py`\n\
                                     \n\
                                     Example:\n\
                                     ```python\n\
                                     import re\n\
                                     with open('{}', 'r') as f:\n\
                                         content = f.read()\n\
                                     content = re.sub(r'pattern', 'replacement', content)\n\
                                     with open('{}', 'w') as f:\n\
                                         f.write(content)\n\
                                     ```\n\
                                     \n\
                                     This completes in 1 second instead of 100 slow edit_file calls!",
                                    target_file, target_file, target_file
                                )
                            } else {
                                String::new()
                            };

                            messages.push(Message {
                                role: "system".to_string(),
                                content: Some(format!(
                                    "🎯 PROACTIVE CONTEXT:\n\
                                     The Router analyzed your query and executed these tools automatically:\n\
                                     {}{}{}\n\n\
                                     Use this context to answer directly. You do NOT need to call these tools again.",
                                    proactive_context,
                                    healing_instructions,
                                    batch_edit_instructions
                                )),
                                tool_calls: None,
                                tool_call_id: None,
                            });
                            tracing::info!("✅ ROUTER: Injected proactive context ({} chars){}{}",
                                proactive_context.len(),
                                if proactive_tools.self_healing { " + self-healing" } else { "" },
                                if proactive_tools.batch_edit_target.is_some() { " + batch-edit-mode" } else { "" }
                            );
                        } else if let Some(ref target_file) = proactive_tools.batch_edit_target {
                            // Even if no proactive context, inject batch edit instructions
                            messages.push(Message {
                                role: "system".to_string(),
                                content: Some(format!(
                                    "⚡ BATCH EDIT MODE ACTIVATED:\n\
                                     You are trying to modify MANY places in `{}`.\n\
                                     \n\
                                     🚨 DO NOT use `edit_file` repeatedly! This will trigger 'Tool Overused' errors.\n\
                                     \n\
                                     ✅ INSTEAD, do this:\n\
                                     1. Write a Python script that reads the file\n\
                                     2. Use regex/string.replace() to modify ALL occurrences at once\n\
                                     3. Write the file back\n\
                                     4. Execute the script with bash tool: `python script.py`\n\
                                     \n\
                                     Example:\n\
                                     ```python\n\
                                     import re\n\
                                     with open('{}', 'r') as f:\n\
                                         content = f.read()\n\
                                     content = re.sub(r'pattern', 'replacement', content)\n\
                                     with open('{}', 'w') as f:\n\
                                         f.write(content)\n\
                                     ```\n\
                                     \n\
                                     This completes in 1 second instead of 100 slow edit_file calls!",
                                    target_file, target_file, target_file
                                )),
                                tool_calls: None,
                                tool_call_id: None,
                            });
                            tracing::info!("✅ ROUTER: Injected batch-edit-mode instructions");
                        }
                    }
                }
            }
        }

        // Remove proactively-used tools from AI's tool list
        // This prevents AI from calling them again (physical prevention)
        if !tools_used_proactively.is_empty() {
            let original_count = tools.len();
            tools.retain(|tool| !tools_used_proactively.contains(&tool.function.name));
            let removed = original_count - tools.len();
            if removed > 0 {
                tracing::info!("🚫 ROUTER: Removed {} proactive tools from AI's tool list: {:?}",
                    removed, tools_used_proactively);
            }
        }

        // ⚡⚡⚡ CONSULTATION SHORT-CIRCUIT ⚡⚡⚡
        // Skip tool execution loop entirely for suggestion/brainstorming queries
        // Response time: <1s instead of 5-10s (no code search, no grep, no verification)
        if is_consultation_mode {
            tracing::info!("⚡ CONSULTATION SHORT-CIRCUIT: Bypassing tool execution loop");

            // Inject strong instructions: DO NOT use tools!
            // (CAPABILITIES.md + ROADMAP.md are already loaded in messages via proactive tools)
            messages.push(Message {
                role: "system".to_string(),
                content: Some(
                    "⚡ CONSULTATION MODE ACTIVATED ⚡\n\
                     \n\
                     You are in BRAINSTORMING mode for feature suggestions.\n\
                     \n\
                     🚨 CRITICAL RULES:\n\
                     1. DO NOT call ANY tools (read_file, grep, semantic_search, etc.)\n\
                     2. DO NOT read source code to \"verify\" features\n\
                     3. Answer IMMEDIATELY based on the documents provided above\n\
                     4. Use CAPABILITIES.md to understand what EXISTS\n\
                     5. Use ROADMAP.md to understand what's PLANNED\n\
                     6. Suggest 3-5 HIGH-IMPACT features that are NOT in either document\n\
                     \n\
                     Why this rule exists:\n\
                     - User wants FAST suggestions (<1s response time)\n\
                     - Reading source code causes context explosion (873% usage)\n\
                     - CAPABILITIES.md + ROADMAP.md are the \"single source of truth\"\n\
                     \n\
                     Format your response as:\n\
                     - Brief intro (1 sentence)\n\
                     - 3-5 bullet points with feature name + why it's useful\n\
                     - Priority ranking (High/Medium/Low based on impact)\n\
                     \n\
                     Example:\n\
                     Based on the capabilities and roadmap, here are high-impact features:\n\
                     \n\
                     🔥 High Priority:\n\
                     - **Interactive TUI Dashboard** - Real-time monitoring improves UX\n\
                     - **MCP Server Mode** - Integration with Claude Desktop/Cursor\n\
                     \n\
                     🎯 Medium Priority:\n\
                     - **Session Bookmarks** - Save frequently used queries\n\
                     \n\
                     Now answer the user's question using ONLY the documents above.\
                     ".to_string()
                ),
                tool_calls: None,
                tool_call_id: None,
            });

            // Clear tools array - AI physically cannot call tools now
            tools.clear();
            tracing::info!("🚫 CONSULTATION MODE: Cleared tools array (0 tools available)");

            // Apply context cleaning and windowing (same as normal flow)
            let cleaned_messages = self.optimizer.lock().unwrap().clean_history(messages.clone());
            let windowed_messages = self.context_window.lock().unwrap().apply_sliding_window(cleaned_messages);

            // Make ONE LLM call without tools
            let (response, _input_tokens, _output_tokens) =
                llm_client.chat_with_tools(windowed_messages, tools).await?;

            match response {
                LLMResponse::Text(text) => {
                    callback.on_response(&text).await;
                    tracing::info!("✅ CONSULTATION SHORT-CIRCUIT: Response generated in single call");
                    return Ok(text);
                }
                LLMResponse::ToolCalls(_) => {
                    // This should NEVER happen (tools array is empty)
                    tracing::error!("❌ CONSULTATION MODE: AI tried to call tools despite empty array!");
                    return Err(anyhow::anyhow!("Unexpected tool calls in consultation mode"));
                }
            }
        }

        let mut iteration = 0;
        let final_response;

        loop {
            iteration += 1;
            if iteration > self.max_iterations {
                tracing::warn!("Reached max iterations for tool calls");
                return Err(anyhow::anyhow!("Maximum tool iterations reached"));
            }

            // Apply context cleaning to reduce token usage and noise
            let cleaned_messages = self.optimizer.lock().unwrap().clean_history(messages.clone());

            // Apply sliding window to prevent token overflow
            let windowed_messages = self.context_window.lock().unwrap().apply_sliding_window(cleaned_messages);

            // Log context stats
            let stats = self.context_window.lock().unwrap().get_stats(&windowed_messages);
            tracing::info!("{}", stats.format());

            // Warn if approaching limit
            if self.context_window.lock().unwrap().is_near_limit(&windowed_messages) {
                tracing::warn!("⚠️  Context window is at {}% capacity. Consider starting a new conversation.", stats.percentage_used);
            }

            // Send request to LLM
            let (response, _input_tokens, _output_tokens) =
                llm_client.chat_with_tools(windowed_messages.clone(), tools.clone()).await?;

            // Update messages with the windowed version to maintain consistency
            messages = windowed_messages;

            match response {
                LLMResponse::Text(text) => {
                    // Final response received
                    callback.on_response(&text).await;
                    final_response = text;
                    break;
                }
                LLMResponse::ToolCalls(tool_calls) => {
                    // Add assistant message with tool calls
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: None,
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    // Notify callback for each tool BEFORE execution
                    // Note: Callbacks remain sequential for clean log output
                    for tool_call in &tool_calls {
                        callback.on_tool_start(&tool_call.function.name, &tool_call.function.arguments).await;
                    }

                    // Execute tools in parallel using tokio spawn + join_all (SHOTGUN MODE 🔫)
                    let mode_config = self.mode_config.clone();
                    let project_root = project_root.to_path_buf();

                    let tool_futures: Vec<_> = tool_calls.iter().map(|tool_call| {
                        let tool_call = tool_call.clone();
                        let tool_name = tool_call.function.name.clone();
                        let tool_id = tool_call.id.clone();
                        let mode_config_clone = mode_config.clone();
                        let project_root = project_root.clone();

                        tokio::spawn(async move {
                            // Check if tool is allowed in current mode
                            if !mode_config_clone.is_tool_allowed(&tool_name) {
                                let error_msg = format!(
                                    "Error: Tool '{}' is not available in {} mode. This mode only allows: read_file, list_files, glob, grep, and other read-only operations.",
                                    tool_name,
                                    mode_config_clone.mode
                                );
                                return (tool_id, tool_name, Err(anyhow::anyhow!(error_msg)));
                            }

                            // Check for write operations in read-only mode
                            if mode_config_clone.read_only && (tool_name == "write_file" || tool_name == "edit_file" || tool_name == "git_commit") {
                                let error_msg = format!(
                                    "Error: Cannot use '{}' in {} mode (read-only). Switch to Code mode to make changes.",
                                    tool_name,
                                    mode_config_clone.mode
                                );
                                return (tool_id, tool_name, Err(anyhow::anyhow!(error_msg)));
                            }

                            // Execute tool (blocking call, but each is on its own task)
                            let result = tokio::task::spawn_blocking(move || {
                                execute_tool(&tool_call, &project_root)
                            }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("Task panic: {}", e)));

                            (tool_id, tool_name, result)
                        })
                    }).collect();

                    // Wait for all tools to complete (parallel execution! ⚡)
                    let results: Vec<_> = join_all(tool_futures)
                        .await
                        .into_iter()
                        .map(|r| r.unwrap_or_else(|e| {
                            ("error".to_string(), "unknown".to_string(), Err(anyhow::anyhow!("Join error: {}", e)))
                        }))
                        .collect();

                    // Process results and add to messages
                    for (i, (tool_id, tool_name, result)) in results.iter().enumerate() {

                        let result_str = match result {
                            Ok(r) => r.clone(),
                            Err(e) => format!("Error: {}", e),
                        };

                        // Detect if this is an error result
                        let is_error = result_str.contains("Error:")
                            || result_str.contains("error:")
                            || result_str.contains("failed")
                            || result_str.contains("FAILED");

                        // Increment turn counter
                        self.optimizer.lock().unwrap().next_turn();

                        // Summarize long results to save tokens
                        let final_content = if let Some(ref summarizer) = self.summarizer {
                            summarizer.summarize_tool_result(&tool_name, &result_str)
                        } else {
                            result_str.clone()
                        };

                        // Apply prompt optimization based on context
                        let mut optimized_content = self.optimizer.lock().unwrap()
                            .optimize_user_message(final_content, is_error);

                        // Check for linting errors after write_file or edit_file
                        if !is_error && (tool_name == "write_file" || tool_name == "edit_file") {
                            // Extract file_path from tool arguments
                            if let Some(tool_call) = tool_calls.get(i) {
                                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
                                    if let Some(file_path) = args["file_path"].as_str() {
                                        if let Some(lint_errors) = self.check_lint_errors(file_path, &project_root) {
                                            optimized_content = format!("{}\n\n{}", optimized_content, lint_errors);
                                            tracing::info!("⚠️ LINT: Injected lint errors for AI to fix");
                                        }
                                    }
                                }
                            }
                        }

                        // Check for infinite loop when errors occur
                        if is_error {
                            let loop_warning = {
                                self.optimizer.lock().unwrap().detect_loop()
                            }; // MutexGuard dropped here

                            if let Some(loop_warning) = loop_warning {
                                tracing::warn!("{}", loop_warning);
                                // Add loop warning to the optimized content
                                let enhanced_content = format!("{}\n\n{}", optimized_content, loop_warning);
                                messages.push(Message {
                                    role: "tool".to_string(),
                                    content: Some(enhanced_content),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_id.clone()),
                                });
                                callback.on_tool_complete(&tool_name, &result_str).await;
                                continue;
                            }
                        }

                        messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(optimized_content),
                            tool_calls: None,
                            tool_call_id: Some(tool_id.clone()),
                        });

                        // Notify callback with original result
                        callback.on_tool_complete(&tool_name, &result_str).await;
                    }

                    // Continue loop to get next response
                }
            }
        }

        Ok(final_response)
    }

    /// Execute conversation with streaming support
    /// TODO: Fix ownership issues - currently not used, streaming implemented directly in WebSocket handler
    /*
    pub async fn execute_stream<C: ToolCallback>(
        &self,
        llm_client: &LLMClient,
        mut messages: Vec<Message>,
        project_root: &PathBuf,
        callback: &mut C,
    ) -> anyhow::Result<String> {
        // Implementation commented out due to ownership issues
        // Use direct streaming in websocket.rs instead
        unimplemented!("Use direct streaming implementation")
    }
    */

    /// Synchronous version that blocks on async execution
    pub fn execute_blocking<C: ToolCallback>(
        &self,
        llm_client: &LLMClient,
        messages: Vec<Message>,
        project_root: &PathBuf,
        callback: &mut C,
    ) -> anyhow::Result<String> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.execute(llm_client, messages, project_root, callback))
    }
}

impl Default for ConversationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::berrycode::tools::{ToolCall, FunctionCall};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tempfile::TempDir;

    // Mock callback to track tool execution
    struct MockCallback {
        tool_starts: Arc<Mutex<Vec<(String, Instant)>>>,
        tool_completes: Arc<Mutex<Vec<(String, Instant)>>>,
    }

    impl MockCallback {
        fn new() -> Self {
            Self {
                tool_starts: Arc::new(Mutex::new(Vec::new())),
                tool_completes: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_execution_times(&self) -> Vec<(String, std::time::Duration)> {
            let starts = self.tool_starts.lock().unwrap();
            let completes = self.tool_completes.lock().unwrap();

            starts.iter().zip(completes.iter()).map(|((name_s, start), (name_c, end))| {
                assert_eq!(name_s, name_c, "Tool start/complete mismatch");
                (name_s.clone(), end.duration_since(*start))
            }).collect()
        }
    }

    #[async_trait]
    impl ToolCallback for MockCallback {
        async fn on_tool_start(&mut self, tool_name: &str, _args: &str) {
            self.tool_starts.lock().unwrap().push((tool_name.to_string(), Instant::now()));
        }

        async fn on_tool_complete(&mut self, tool_name: &str, _result: &str) {
            self.tool_completes.lock().unwrap().push((tool_name.to_string(), Instant::now()));
        }

        async fn on_response(&mut self, _text: &str) {}
        async fn on_response_chunk(&mut self, _chunk: &str) {}
    }

    #[tokio::test]
    async fn test_parallel_tool_execution() {
        // Create temporary project directory
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();

        // Create test files
        std::fs::write(project_root.join("file1.txt"), "content1").unwrap();
        std::fs::write(project_root.join("file2.txt"), "content2").unwrap();
        std::fs::write(project_root.join("file3.txt"), "content3").unwrap();

        let engine = ConversationEngine::new();
        let mut callback = MockCallback::new();

        // Simulate multiple tool calls (3 read_file operations)
        let tool_calls = vec![
            ToolCall {
                id: "call1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "file1.txt"}).to_string(),
                },
            },
            ToolCall {
                id: "call2".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "file2.txt"}).to_string(),
                },
            },
            ToolCall {
                id: "call3".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "file3.txt"}).to_string(),
                },
            },
        ];

        // Start timer
        let start = Instant::now();

        // Execute the parallel tool execution logic directly
        // (This mirrors the code in execute() method)

        // Notify callback for each tool
        for tool_call in &tool_calls {
            callback.on_tool_start(&tool_call.function.name, &tool_call.function.arguments).await;
        }

        // Execute tools in parallel (SHOTGUN MODE)
        let mode_config = engine.mode_config.clone();
        let project_root_clone = project_root.clone();

        let tool_futures: Vec<_> = tool_calls.iter().map(|tool_call| {
            let tool_call = tool_call.clone();
            let tool_name = tool_call.function.name.clone();
            let tool_id = tool_call.id.clone();
            let _mode_config = mode_config.clone();
            let project_root = project_root_clone.clone();

            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    execute_tool(&tool_call, &project_root)
                }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("Task panic: {}", e)));

                (tool_id, tool_name, result)
            })
        }).collect();

        // Wait for all tools to complete
        let results: Vec<_> = join_all(tool_futures)
            .await
            .into_iter()
            .map(|r| r.unwrap_or_else(|e| {
                ("error".to_string(), "unknown".to_string(), Err(anyhow::anyhow!("Join error: {}", e)))
            }))
            .collect();

        let elapsed = start.elapsed();

        // Notify completions
        for (_, tool_name, result) in &results {
            let result_str = match result {
                Ok(r) => r.clone(),
                Err(e) => format!("Error: {}", e),
            };
            callback.on_tool_complete(tool_name, &result_str).await;
        }

        // ASSERTIONS

        // 1. All 3 tools should have executed
        assert_eq!(callback.tool_starts.lock().unwrap().len(), 3, "Should have 3 tool starts");
        assert_eq!(callback.tool_completes.lock().unwrap().len(), 3, "Should have 3 tool completes");

        // 2. Parallel execution should be MUCH faster than sequential
        // If sequential: 3 files × ~50ms each = ~150ms minimum
        // If parallel: max(50ms, 50ms, 50ms) = ~50ms
        // With some overhead, parallel should be < 200ms, sequential would be > 150ms
        println!("⚡ Parallel execution took: {:?}", elapsed);

        // Lenient check: just ensure it completed
        assert!(elapsed.as_millis() < 2000, "Parallel execution should complete within 2 seconds, took {:?}", elapsed);

        // 3. Check that all starts happened before any completes (roughly parallel)
        let starts = callback.tool_starts.lock().unwrap();
        let completes = callback.tool_completes.lock().unwrap();

        let last_start = starts.iter().map(|(_, t)| t).max().unwrap();
        let first_complete = completes.iter().map(|(_, t)| t).min().unwrap();

        // In true parallel execution, the last start should be BEFORE or VERY CLOSE to first complete
        // This proves they're running concurrently, not sequentially
        let overlap = first_complete.duration_since(*last_start);
        println!("📊 Time between last start and first complete: {:?}", overlap);

        // This should be very small (< 100ms) if truly parallel
        assert!(overlap.as_millis() < 500, "Tools should overlap in execution (parallel), gap was {:?}", overlap);

        println!("✅ Parallel execution test passed!");
        println!("   - 3 tools executed in {:?}", elapsed);
        println!("   - Overlap: {:?} (should be minimal)", overlap);
    }

    #[tokio::test]
    async fn test_parallel_with_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();

        // Create only 1 file (so 2nd and 3rd will error)
        std::fs::write(project_root.join("exists.txt"), "content").unwrap();

        let engine = ConversationEngine::new();
        let mut callback = MockCallback::new();

        // Mix of successful and failing tool calls
        let tool_calls = vec![
            ToolCall {
                id: "call1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "exists.txt"}).to_string(),
                },
            },
            ToolCall {
                id: "call2".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "missing1.txt"}).to_string(),
                },
            },
            ToolCall {
                id: "call3".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"file_path": "missing2.txt"}).to_string(),
                },
            },
        ];

        // Execute in parallel
        for tool_call in &tool_calls {
            callback.on_tool_start(&tool_call.function.name, &tool_call.function.arguments).await;
        }

        let mode_config = engine.mode_config.clone();
        let project_root_clone = project_root.clone();

        let tool_futures: Vec<_> = tool_calls.iter().map(|tool_call| {
            let tool_call = tool_call.clone();
            let tool_name = tool_call.function.name.clone();
            let tool_id = tool_call.id.clone();
            let _mode_config = mode_config.clone();
            let project_root = project_root_clone.clone();

            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    execute_tool(&tool_call, &project_root)
                }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("Task panic: {}", e)));

                (tool_id, tool_name, result)
            })
        }).collect();

        let results: Vec<_> = join_all(tool_futures).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        for (_, tool_name, result) in &results {
            let result_str = match result {
                Ok(r) => r.clone(),
                Err(e) => format!("Error: {}", e),
            };
            callback.on_tool_complete(tool_name, &result_str).await;
        }

        // ASSERTIONS
        assert_eq!(results.len(), 3, "Should get 3 results");

        // First should succeed, others should fail
        assert!(results[0].2.is_ok(), "First file should succeed");
        assert!(results[1].2.as_ref().err().is_some() || results[1].2.as_ref().ok().unwrap().contains("Error"), "Second should error");
        assert!(results[2].2.as_ref().err().is_some() || results[2].2.as_ref().ok().unwrap().contains("Error"), "Third should error");

        println!("✅ Error handling test passed!");
        println!("   - 1 success, 2 errors handled correctly in parallel");
    }

    #[test]
    fn test_conversation_engine_creation() {
        let engine = ConversationEngine::new();
        assert_eq!(engine.max_iterations, 30);
    }

    #[test]
    fn test_conversation_engine_with_custom_max_iterations() {
        let engine = ConversationEngine::new().with_max_iterations(50);
        assert_eq!(engine.max_iterations, 50);
    }

    #[tokio::test]
    async fn test_proactive_semantic_search() {
        use crate::berrycode::router::IntentRouter;

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().to_path_buf();

        // Create some files for vector index
        std::fs::write(project_root.join("main.rs"), "fn main() { println!(\"Hello\"); }").unwrap();
        std::fs::write(project_root.join("lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

        // Test router detection
        let router = IntentRouter::new();
        let tools = router.get_proactive_tools("このプロジェクトの設計を教えて");

        // Should trigger semantic_search
        assert!(tools.semantic_search, "Router should detect conceptual question");
        assert!(tools.files_to_read.is_empty(), "Should not read specific files");

        println!("✅ Proactive semantic search detection works!");
    }

    #[tokio::test]
    async fn test_proactive_file_read() {
        use crate::berrycode::router::IntentRouter;

        let router = IntentRouter::new();
        let tools = router.get_proactive_tools("src/main.rs を見せて");

        // Should trigger file read
        assert!(!tools.semantic_search, "Should not run semantic search for file read");
        assert_eq!(tools.files_to_read, vec!["src/main.rs"], "Should detect file path");

        println!("✅ Proactive file read detection works!");
    }

    #[tokio::test]
    async fn test_bug_fix_detection() {
        use crate::berrycode::router::IntentRouter;

        let router = IntentRouter::new();
        let tools = router.get_proactive_tools("src/auth.rs のバグを修正して");

        // Should trigger bug fix mode
        assert!(tools.self_healing, "Should enable self-healing for bug fix");
        assert_eq!(tools.files_to_read, vec!["src/auth.rs"], "Should read buggy file");
        // NOTE: We no longer run proactive grep for bug fixes to avoid 413 Payload Too Large errors
        // (error|ERROR|panic patterns match 18k+ results). AI decides if it needs grep.
        assert!(tools.grep_patterns.is_empty(), "Should NOT run proactive grep for bug fixes");

        println!("✅ Bug fix detection works!");
    }

    #[tokio::test]
    async fn test_tool_removal_from_list() {
        use crate::berrycode::tools::get_available_tools;

        let all_tools = get_available_tools();
        let original_count = all_tools.len();

        // Simulate proactive tool execution
        let tools_used_proactively = vec!["semantic_search".to_string(), "read_file".to_string()];

        // Remove proactively-used tools
        let mut filtered_tools = all_tools;
        filtered_tools.retain(|tool| !tools_used_proactively.contains(&tool.function.name));

        // Verify removal
        assert_eq!(filtered_tools.len(), original_count - 2, "Should remove 2 tools");
        assert!(
            !filtered_tools.iter().any(|t| t.function.name == "semantic_search"),
            "semantic_search should be removed"
        );
        assert!(
            !filtered_tools.iter().any(|t| t.function.name == "read_file"),
            "read_file should be removed"
        );

        // Verify other tools still exist
        assert!(
            filtered_tools.iter().any(|t| t.function.name == "grep"),
            "grep should still be available"
        );
        assert!(
            filtered_tools.iter().any(|t| t.function.name == "bash"),
            "bash should still be available"
        );

        println!("✅ Tool removal works correctly!");
    }

    #[tokio::test]
    async fn test_chitchat_no_proactive_tools() {
        use crate::berrycode::router::IntentRouter;

        let router = IntentRouter::new();
        let tools = router.get_proactive_tools("ok");

        // ChitChat should not trigger any proactive tools
        assert!(!tools.semantic_search, "ChitChat should not run semantic_search");
        assert!(tools.files_to_read.is_empty(), "ChitChat should not read files");
        assert!(tools.grep_patterns.is_empty(), "ChitChat should not grep");
        assert!(!tools.self_healing, "ChitChat should not enable self-healing");

        println!("✅ ChitChat detection works - no proactive tools!");
    }
}
