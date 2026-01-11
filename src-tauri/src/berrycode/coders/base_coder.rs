//! Base coder implementation

use crate::berrycode::{
    Result,
    io::InputOutput,
    display::DisplayManager,
    models::Model,
    repo::GitRepo,
    repomap::RepoMap,
    prompts::{PromptGenerator, EditBlock},
    llm::{LLMClient, Message},
    diff::DiffApplier,
    context::ContextManager,
    summarizer::Summarizer,
    tool_monitor::ToolMonitor,
    agent::SmartAgent,
    task_spawner::TaskSpawner,
    plan_mode::PlanMode,
    conversation_engine::{ConversationEngine, ToolCallback},
    speculative_executor::SpeculativeExecutor,
    proactive_agent::ProactiveAgent,
};
use std::path::PathBuf;
use std::fs;

pub struct Coder {
    pub io: InputOutput,
    pub model: Model,
    pub git_repo: Option<GitRepo>,
    pub files: Vec<PathBuf>,
    pub read_only_files: Vec<PathBuf>,
    pub chat_history: Vec<ChatMessage>,
    pub llm_client: Option<LLMClient>,
    pub prompt_generator: PromptGenerator,
    pub repo_map: Option<RepoMap>,
    pub auto_commits: bool,
    pub dry_run: bool,
    // Smart systems
    pub context_manager: ContextManager,
    pub summarizer: Summarizer,
    pub tool_monitor: ToolMonitor,
    pub agent: SmartAgent,
    pub task_spawner: TaskSpawner,
    pub plan_mode: PlanMode,
    // Advanced AI features
    pub speculative_executor: Option<SpeculativeExecutor>,
    pub proactive_agent: Option<ProactiveAgent>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// CLI-specific callback for tool execution
struct CliToolCallback<'a> {
    io: &'a InputOutput,
    tool_monitor: &'a mut ToolMonitor,
    context_manager: &'a mut ContextManager,
    summarizer: &'a Summarizer,
}

#[async_trait::async_trait]
impl<'a> ToolCallback for CliToolCallback<'a> {
    async fn on_tool_start(&mut self, tool_name: &str, args: &str) {
        // Parse args and extract key information for display
        let display_info = if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(args) {
            match tool_name {
                "read_file" => {
                    if let Some(path) = parsed.get("file_path").and_then(|v| v.as_str()) {
                        format!("  → read_file: {}", path)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "write_file" | "edit_file" => {
                    if let Some(path) = parsed.get("file_path").and_then(|v| v.as_str()) {
                        format!("  → {}: {}", tool_name, path)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "bash" => {
                    if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
                        // Truncate long commands
                        let display_cmd = if cmd.len() > 60 {
                            format!("{}...", &cmd[..60])
                        } else {
                            cmd.to_string()
                        };
                        format!("  → bash: {}", display_cmd)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "grep" => {
                    if let Some(pattern) = parsed.get("pattern").and_then(|v| v.as_str()) {
                        format!("  → grep: \"{}\"", pattern)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "glob" | "search_files" => {
                    if let Some(pattern) = parsed.get("pattern").and_then(|v| v.as_str()) {
                        format!("  → {}: {}", tool_name, pattern)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "list_files" => {
                    if let Some(dir) = parsed.get("directory").and_then(|v| v.as_str()) {
                        format!("  → list_files: {}", dir)
                    } else {
                        format!("  → list_files: .")
                    }
                }
                "web_fetch" => {
                    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
                        format!("  → web_fetch: {}", url)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                "web_search" => {
                    if let Some(query) = parsed.get("query").and_then(|v| v.as_str()) {
                        format!("  → web_search: \"{}\"", query)
                    } else {
                        format!("  → {}", tool_name)
                    }
                }
                _ => format!("  → {}", tool_name)
            }
        } else {
            format!("  → {}", tool_name)
        };

        self.io.tool_output(&display_info);
    }

    async fn on_tool_complete(&mut self, tool_name: &str, result: &str) {
        // Summarize large tool results to save context
        let processed_result = self.summarizer.summarize_tool_result(tool_name, result);

        // Estimate and track tokens (using processed/summarized result)
        let result_tokens = self.context_manager.estimate_tokens(&processed_result);
        self.context_manager.add_tokens(result_tokens);

        // Monitor tool usage
        self.tool_monitor.record_call(tool_name, result_tokens);

        // Check for overuse (but be less aggressive for bash/git tools)
        if self.tool_monitor.is_overused(tool_name) {
            // Only warn for non-git/bash tools, as git operations are often legitimate
            if !matches!(tool_name, "bash" | "git_diff" | "git_commit" | "git_pr_create") {
                self.io.tool_warning(&format!(
                    "⚠️  Tool '{}' is being overused. Consider a different approach.",
                    tool_name
                ));
            }
        }

        // Check context usage
        if self.context_manager.is_near_limit() {
            self.io.tool_warning(&format!(
                "⚠️  Context limit approaching: {:.1}% used",
                self.context_manager.usage_percentage()
            ));
        }

        // Suggestions are now logged but not shown to user to reduce noise
        // (They were mostly about bash command combining which is an AI optimization)
        if let Some(_suggestion) = self.tool_monitor.suggest_optimization() {
            // Silently log optimization opportunities without bothering the user
            tracing::debug!("Tool optimization suggestion available");
        }
    }

    async fn on_response(&mut self, text: &str) {
        self.io.ai_output(text);
    }

    async fn on_response_chunk(&mut self, _chunk: &str) {
        // CLI doesn't show chunks for now (can be added later for better UX)
    }
}

impl Coder {
    pub fn new(
        io: InputOutput,
        model: Model,
        git_repo: Option<GitRepo>,
        files: Vec<PathBuf>,
        read_only_files: Vec<PathBuf>,
    ) -> Self {
        let edit_format = model.edit_format.clone().unwrap_or_else(|| "diff".to_string());
        let prompt_generator = PromptGenerator::new(edit_format);

        let repo_map = git_repo.as_ref().map(|repo| {
            RepoMap::new(repo.root().to_path_buf())
        });

        let context_manager = ContextManager::new(model.name.clone());

        // Initialize advanced AI features if we have a git repo
        let (speculative_executor, proactive_agent) = if let Some(ref repo) = git_repo {
            let project_root = repo.root();

            // Initialize speculative executor
            let spec_exec = SpeculativeExecutor::new(project_root).ok();

            // Initialize proactive agent
            let proac_agent = ProactiveAgent::new(project_root).ok();

            (spec_exec, proac_agent)
        } else {
            (None, None)
        };

        Self {
            io,
            model,
            git_repo,
            files,
            read_only_files,
            chat_history: Vec::new(),
            llm_client: None,
            prompt_generator,
            repo_map,
            auto_commits: true,
            dry_run: false,
            context_manager,
            summarizer: Summarizer::new(),
            tool_monitor: ToolMonitor::new(),
            agent: SmartAgent::new(),
            task_spawner: TaskSpawner::new(),
            plan_mode: PlanMode::new(),
            speculative_executor,
            proactive_agent,
        }
    }

    /// Initialize LLM client with API key
    pub fn init_llm(&mut self, api_key: String) -> Result<()> {
        self.llm_client = Some(LLMClient::new(&self.model, api_key)?);
        Ok(())
    }

    /// Run the main chat loop
    pub fn run(&mut self) -> Result<()> {
        // 🔍 Proactive Agent: Start background file monitoring
        let _proactive_handle = if let Some(ref agent) = self.proactive_agent {
            self.io.tool_output("🔍 Proactive monitoring enabled - watching for file changes and errors");
            Some(agent.start())
        } else {
            None
        };

        // 🎯 Vector Search: Start background indexing
        let _indexing_handle = if let Some(ref repo) = self.git_repo {
            use crate::berrycode::vector_search::VectorSearch;
            self.io.tool_output("🎯 Vector search: Starting background indexing...");
            Some(VectorSearch::start_background_indexing(repo.root().to_path_buf()))
        } else {
            None
        };

        loop {
            let input = self.io.get_input(">")?;

            if input.trim().is_empty() {
                continue;
            }

            if input.starts_with('/') {
                // Handle command
                let mut commands = crate::berrycode::commands::Commands::new(self.io.clone());
                if !commands.execute(&input, self)? {
                    break;
                }
            } else {
                // Handle chat message
                if let Err(e) = self.handle_message(&input) {
                    self.io.tool_error(&format!("Error: {}", e));
                }
            }
        }

        Ok(())
    }

    fn handle_message(&mut self, message: &str) -> Result<()> {
        // Display user input in a beautiful box
        self.io.user_input_display(message);

        // On first message, pre-load critical files for context
        // This gives AI instant knowledge of project structure
        if self.chat_history.is_empty() {
            self.preload_critical_files();
        }

        // Reset agent for new query
        self.agent.reset();

        // Detect strategy from user query
        self.agent.detect_strategy(message);

        // Check if this requires planning
        if self.plan_mode.requires_planning(message) {
            return self.handle_plan_mode(message);
        }

        // Check if query can be decomposed into subtasks
        let subtasks = self.task_spawner.decompose_query(message);
        if !subtasks.is_empty() {
            return self.handle_task_spawning(message, subtasks);
        }

        // Add user message to history
        self.chat_history.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        // 🚀 Speculative Execution: Predict and pre-execute likely tool calls
        if let Some(ref executor) = self.speculative_executor {
            executor.analyze_stream(message);
        }

        // Check if LLM client is initialized
        if self.llm_client.is_none() {
            self.io.tool_warning("LLM client not initialized. Set API key with OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable.");
            return Ok(());
        }

        // Execute with LLM
        self.execute_with_llm()
    }

    pub fn apply_edits(&mut self, response: &str) -> Result<()> {
        let edit_blocks = self.prompt_generator.parse_edit_response(response)?;

        if edit_blocks.is_empty() {
            return Ok(());
        }

        self.io.tool_output(&format!("Applying {} edits...", edit_blocks.len()));

        let diff_applier = DiffApplier::new(self.dry_run);
        let mut modified_files = Vec::new();

        for block in &edit_blocks {
            match self.apply_single_edit(&diff_applier, block) {
                Ok(file_path) => {
                    if let Some(path) = file_path {
                        modified_files.push(path);
                    }
                }
                Err(e) => {
                    self.io.tool_error(&format!("Failed to apply edit: {}", e));
                }
            }
        }

        // Auto commit if enabled
        if self.auto_commits && !modified_files.is_empty() && !self.dry_run {
            self.auto_commit(&modified_files)?;
        }

        Ok(())
    }

    fn apply_single_edit(&mut self, diff_applier: &DiffApplier, block: &EditBlock) -> Result<Option<PathBuf>> {
        use crate::berrycode::prompts::EditBlockType;

        match &block.block_type {
            EditBlockType::SearchReplace => {
                if let (Some(file_path), Some(search), Some(replace)) =
                    (&block.file_path, &block.search, &block.replace) {
                    let path = PathBuf::from(file_path);

                    // Read original content for diff
                    let original_content = fs::read_to_string(&path)?;
                    let new_content = original_content.replace(search, replace);

                    // Show diff before applying
                    if original_content != new_content {
                        self.show_diff_preview(file_path, &original_content, &new_content);
                    }

                    if !self.dry_run {
                        fs::write(&path, &new_content)?;
                    }

                    if self.io.pretty {
                        let display = DisplayManager::new();
                        display.print_success(&format!("Modified: {}", file_path));
                    } else {
                        self.io.tool_output(&format!("✓ Modified: {}", file_path));
                    }
                    return Ok(Some(path));
                } else {
                    // Parse failure - log error
                    self.io.tool_error("⚠️  Failed to parse Search/Replace block: Missing file path, search pattern, or replace content.");
                    if block.file_path.is_none() {
                        self.io.tool_error("   → No file path found. Make sure the filename is specified before the code block.");
                    }
                    return Ok(None);
                }
            }
            EditBlockType::WholeFile => {
                if let (Some(file_path), Some(content)) = (&block.file_path, &block.content) {
                    let path = PathBuf::from(file_path);

                    // Show diff if file exists
                    if path.exists() {
                        if let Ok(original_content) = fs::read_to_string(&path) {
                            self.show_diff_preview(file_path, &original_content, content);
                        }
                    }

                    if !self.dry_run {
                        fs::write(&path, content)?;
                    }

                    if self.io.pretty {
                        let display = DisplayManager::new();
                        display.print_success(&format!("Modified: {}", file_path));
                    } else {
                        self.io.tool_output(&format!("✓ Modified: {}", file_path));
                    }
                    return Ok(Some(path));
                } else {
                    self.io.tool_error("⚠️  Failed to parse Whole File block: Missing file path or content.");
                    return Ok(None);
                }
            }
        }
    }

    /// Show a preview of changes using diff
    fn show_diff_preview(&self, file_path: &str, original: &str, new_content: &str) {
        use colored::Colorize;

        self.io.tool_output(&format!("\n📝 Changes to {}:", file_path));

        // Simple line-by-line diff
        let original_lines: Vec<&str> = original.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let max_lines_to_show = 20;
        let mut shown = 0;

        for (i, (old, new)) in original_lines.iter().zip(new_lines.iter()).enumerate() {
            if old != new {
                if shown < max_lines_to_show {
                    self.io.tool_output(&format!("  {}: {}", i + 1, old.red()));
                    self.io.tool_output(&format!("  {}: {}", i + 1, new.green()));
                    shown += 1;
                } else if shown == max_lines_to_show {
                    self.io.tool_output("  ... (more changes not shown)");
                    break;
                }
            }
        }

        // Show added/removed lines
        if new_lines.len() > original_lines.len() {
            self.io.tool_output(&format!("  + {} lines added", new_lines.len() - original_lines.len()).green());
        } else if original_lines.len() > new_lines.len() {
            self.io.tool_output(&format!("  - {} lines removed", original_lines.len() - new_lines.len()).red());
        }

        self.io.tool_output("");
    }

    fn auto_commit(&mut self, modified_files: &[PathBuf]) -> Result<()> {
        if let Some(ref repo) = self.git_repo {
            let file_paths: Vec<&std::path::Path> = modified_files.iter().map(|p| p.as_path()).collect();
            repo.add(&file_paths)?;

            let commit_msg = format!(
                "aider: Modified {} file(s)\n\nFiles changed:\n{}",
                modified_files.len(),
                modified_files.iter()
                    .map(|p| format!("  - {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            repo.commit(&commit_msg)?;
            self.io.tool_output("Changes committed to git");
        }

        Ok(())
    }

    /// Handle plan mode for complex tasks
    fn handle_plan_mode(&mut self, message: &str) -> Result<()> {
        self.io.tool_output("\n🎯 プランモードに入ります");
        self.io.tool_output("このタスクは実装前に計画が必要です。\n");

        // Generate plan
        let plan = self.plan_mode.generate_plan(message);

        // Display plan (will be formatted with colors via ai_output)
        self.io.ai_output(&plan.format_plan());

        // Ask for approval
        let response = self.io.get_input("\nこの計画を承認しますか？ (yes/no): ")?;

        if response.trim().to_lowercase() == "yes" || response.trim().to_lowercase() == "y" {
            self.plan_mode.approve_plan();
            self.io.tool_output("✓ 計画が承認されました。実装を開始します...\n");

            // Continue with normal execution but with plan context
            self.chat_history.push(ChatMessage {
                role: "user".to_string(),
                content: format!("{}\n\n実装計画:\n{}", message, plan.format_plan()),
            });

            self.execute_with_llm()
        } else {
            self.io.tool_output("✗ 計画が却下されました。リクエストを修正してください。");
            self.plan_mode.clear_plan();
            Ok(())
        }
    }

    /// Handle task spawning for parallel execution
    fn handle_task_spawning(&mut self, message: &str, subtasks: Vec<crate::berrycode::task_spawner::Task>) -> Result<()> {
        self.io.tool_output("\n🚀 TASK SPAWNING MODE");
        self.io.tool_output(&format!("Decomposing query into {} subtasks...\n", subtasks.len()));

        // Display subtasks
        for (i, task) in subtasks.iter().enumerate() {
            self.io.tool_output(&format!("{}. {} [{}]", i + 1, task.description,
                match task.task_type {
                    crate::berrycode::task_spawner::TaskType::Explore { .. } => "Explore",
                    crate::berrycode::task_spawner::TaskType::Find { .. } => "Find",
                    crate::berrycode::task_spawner::TaskType::Analyze { .. } => "Analyze",
                    crate::berrycode::task_spawner::TaskType::Implement { .. } => "Implement",
                    crate::berrycode::task_spawner::TaskType::Test { .. } => "Test",
                }));
        }

        self.io.tool_output("");

        // Add tasks to spawner
        for task in subtasks {
            self.task_spawner.add_task(task);
        }

        // For now, execute normally with task context
        // In a full implementation, we would execute tasks in parallel
        self.io.tool_output("Executing tasks...\n");

        self.chat_history.push(ChatMessage {
            role: "user".to_string(),
            content: format!("{}\n\nNote: This has been decomposed into subtasks.", message),
        });

        self.execute_with_llm()
    }

    /// Execute query with LLM (extracted for reuse)
    fn execute_with_llm(&mut self) -> Result<()> {
        // Check if LLM client is initialized
        if self.llm_client.is_none() {
            self.io.tool_warning("LLM client not initialized.");
            return Ok(());
        }

        // Reserve tokens for output (minimum 4096 tokens for LLM response)
        const RESERVED_FOR_OUTPUT: usize = 4096;
        let max_input_tokens = if self.context_manager.max_tokens > RESERVED_FOR_OUTPUT {
            self.context_manager.max_tokens - RESERVED_FOR_OUTPUT
        } else {
            self.context_manager.max_tokens / 2 // Fallback: use half for input, half for output
        };

        // Trim chat history if context is too large
        while self.context_manager.current_tokens > max_input_tokens && !self.chat_history.is_empty() {
            // Remove oldest user-assistant pair
            let tokens_to_remove = self.context_manager.estimate_tokens(&self.chat_history[0].content);
            self.chat_history.remove(0);
            self.context_manager.current_tokens = self.context_manager.current_tokens.saturating_sub(tokens_to_remove);

            // Warn user about history trimming
            if self.chat_history.is_empty() || self.chat_history.len() % 4 == 0 {
                self.io.tool_warning(&format!(
                    "⚠️  Trimming old conversation history to fit context ({:.1}% → {:.1}%)",
                    (self.context_manager.current_tokens as f64 / self.context_manager.max_tokens as f64) * 100.0,
                    (self.context_manager.current_tokens as f64 / self.context_manager.max_tokens as f64) * 100.0
                ));
            }
        }

        // Generate prompt with agent planning guidance
        let mut system_prompt = self.prompt_generator.generate_system_prompt(
            &self.files.iter().map(|p| p.as_path()).collect::<Vec<_>>()
        );

        // Add repository map for context (gives AI a "map" of the codebase)
        if self.repo_map.is_some() {
            let project_root = self.git_repo.as_ref()
                .map(|r| r.root().to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap());

            // 🚀 Load repo map from cache (or build if cache miss)
            // This is 100x faster than building from scratch: 5ms vs 750ms
            let mentioned_files: Vec<PathBuf> = self.files.clone();

            if let Ok(repo_map_mutable) = crate::berrycode::repomap::RepoMap::load_or_build(project_root.clone(), &mentioned_files) {
                let map_string = repo_map_mutable.get_map_string(2000);
                if !map_string.is_empty() {
                    system_prompt.push_str("\n\n");
                    system_prompt.push_str(&map_string);
                }
            }
        }

        // Add file structure for quick navigation (like a "table of contents")
        if let Some(ref repo) = self.git_repo {
            if let Ok(output) = std::process::Command::new("git")
                .arg("ls-files")
                .current_dir(repo.root())
                .output()
            {
                if let Ok(file_list) = String::from_utf8(output.stdout) {
                    if !file_list.trim().is_empty() {
                        let lines: Vec<&str> = file_list.lines().collect();
                        let preview = if lines.len() > 100 {
                            format!("{}\n... ({} more files)", lines[..100].join("\n"), lines.len() - 100)
                        } else {
                            file_list
                        };

                        system_prompt.push_str("\n\n# File Structure\n");
                        system_prompt.push_str("Quick reference of all files in this project:\n```\n");
                        system_prompt.push_str(&preview);
                        system_prompt.push_str("\n```\n");
                    }
                }
            }
        }

        // Add agent planning guidance
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&self.agent.generate_plan_guidance());

        // Add context status with output reservation info
        system_prompt.push_str(&format!(
            "\n\nContext Usage: {:.1}% ({} / {} tokens used for input, {} reserved for your response)\n",
            self.context_manager.usage_percentage(),
            self.context_manager.current_tokens,
            max_input_tokens,
            RESERVED_FOR_OUTPUT
        ));

        // Build messages for LLM
        let mut messages = vec![Message::system(system_prompt.clone())];

        // Add chat history
        for msg in &self.chat_history {
            messages.push(Message {
                role: msg.role.clone(),
                content: Some(msg.content.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Send to LLM with tools - show spinner
        let display = DisplayManager::new();
        let spinner = display.show_spinner("Thinking deeply...");

        // Get project root
        let project_root = self.git_repo.as_ref()
            .map(|r| r.root().to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        // Create conversation engine with summarizer and callback
        let engine = ConversationEngine::with_summarizer(self.summarizer.clone());
        let mut callback = CliToolCallback {
            io: &self.io,
            tool_monitor: &mut self.tool_monitor,
            context_manager: &mut self.context_manager,
            summarizer: &self.summarizer,
        };

        // Execute conversation with tool loop
        let llm_client = self.llm_client.as_ref().unwrap();
        let response_text = engine.execute_blocking(
            llm_client,
            messages,
            &project_root,
            &mut callback
        )?;

        // Stop spinner
        spinner.finish_and_clear();

        // Add assistant response to history
        self.chat_history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response_text.clone(),
        });

        // Parse and apply edits if any
        self.apply_edits(&response_text)?;

        // Show efficiency report after conversation
        let efficiency = self.agent.get_efficiency_score();
        if efficiency < 70 {
            self.io.tool_warning(&format!(
                "⚠️  Efficiency score: {}% - Consider more targeted tool usage",
                efficiency
            ));
        }

        // Show tool usage summary if verbose
        if self.tool_monitor.total_calls() > 10 {
            self.io.tool_output(&self.tool_monitor.get_report());
        }

        Ok(())
    }

    /// Add a file to the chat
    pub fn add_file(&mut self, file: PathBuf) -> Result<()> {
        if !self.files.contains(&file) {
            self.files.push(file);
            self.io.tool_output(&format!("Added file to chat"));
        }
        Ok(())
    }

    /// Remove a file from the chat
    pub fn drop_file(&mut self, file: &PathBuf) -> Result<()> {
        self.files.retain(|f| f != file);
        self.io.tool_output("Removed file from chat");
        Ok(())
    }

    /// Get diff of uncommitted changes
    pub fn show_diff(&mut self) -> Result<()> {
        if let Some(ref repo) = self.git_repo {
            tracing::debug!("Fetching diff from git repository");
            let diff = repo.diff()?;
            tracing::debug!("Diff length: {} bytes", diff.len());

            if diff.is_empty() {
                self.io.tool_output("No uncommitted changes");
            } else {
                // Use tool_output which will format the diff properly
                self.io.tool_output(&diff);
            }
        } else {
            self.io.tool_warning("Not in a git repository");
        }
        Ok(())
    }

    /// Undo last commit
    pub fn undo_last_commit(&mut self) -> Result<()> {
        if let Some(ref repo) = self.git_repo {
            // Check if there are uncommitted changes
            if repo.has_changes()? {
                self.io.tool_warning("⚠ Warning: You have uncommitted changes!");
                let response = self.io.get_input("Continue with reset? This will keep your working directory changes. (y/n): ")?;
                if response.trim().to_lowercase() != "y" {
                    self.io.tool_output("Undo cancelled");
                    return Ok(());
                }
            }

            // Get the previous commit
            let previous_commit = repo.get_previous_commit()?;
            let current_commit = repo.get_head_commit()?;

            self.io.tool_output(&format!("Resetting from {} to {}",
                &current_commit.to_string()[..8],
                &previous_commit.to_string()[..8]
            ));

            // Perform soft reset (keeps working directory and index)
            repo.reset_to_commit(previous_commit, git2::ResetType::Soft)?;

            self.io.tool_output("✓ Undone last commit (files remain in working directory)");
            self.io.tool_output("  Use /diff to see changes");
        } else {
            self.io.tool_warning("Not in a git repository");
        }
        Ok(())
    }

    /// Pre-load critical files to give AI instant project context
    /// This prevents AI from having to search for basic info like:
    /// - What dependencies exist? (Cargo.toml)
    /// - How to run the project? (README.md, src/main.rs)
    /// - What command-line options are available? (src/args.rs)
    fn preload_critical_files(&mut self) {
        let project_root = self.git_repo.as_ref()
            .map(|r| r.root().to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        // Critical files that give AI the "big picture" instantly
        let critical_files = vec![
            "Cargo.toml",           // Dependencies, project metadata
            "package.json",         // For JS/TS projects
            "README.md",            // Project documentation
            "src/main.rs",          // Entry point for Rust
            "src/args.rs",          // CLI arguments
            "src/lib.rs",           // Library interface
            "main.py",              // Entry point for Python
            "__init__.py",          // Python package
        ];

        for relative_path in critical_files {
            let full_path = project_root.join(relative_path);

            // Only load files that exist and are reasonably sized
            if full_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    // Skip files larger than 50KB to avoid bloating context
                    if metadata.len() > 50_000 {
                        continue;
                    }

                    if let Ok(content) = std::fs::read_to_string(&full_path) {
                        // Add to chat history as system context
                        self.chat_history.push(ChatMessage {
                            role: "system".to_string(),
                            content: format!(
                                "## Pre-loaded File: {}\n\n```\n{}\n```\n\nThis file was automatically loaded to give you instant project context.",
                                relative_path,
                                content
                            ),
                        });

                        tracing::debug!("Pre-loaded critical file: {}", relative_path);
                    }
                }
            }
        }
    }
}
