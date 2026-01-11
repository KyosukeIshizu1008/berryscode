//! BerryCode - AI pair programming in your terminal
//! Rust implementation

use berry_editor_tauri::berrycode::{
    args::Args,
    io::InputOutput,
    models::Model,
    repo::{GitRepo, find_git_root},
    coders::Coder,
    welcome,
    project_manager::ProjectManager,
    Result,
};
use clap::Parser;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Setup graceful shutdown handler for Ctrl+C
    setup_signal_handler();

    // Load .env file if it exists (current directory or parent directories)
    dotenv::dotenv().ok();

    // Also try to load from ~/.berrycode/.env
    if let Some(home_dir) = dirs::home_dir() {
        let global_env = home_dir.join(".berrycode").join(".env");
        if global_env.exists() {
            dotenv::from_path(&global_env).ok();
        }
    }

    // Initialize tracing/logging
    tracing_subscriber::fmt::init();

    // Parse command-line arguments
    let mut args = Args::parse();
    args.resolve_git();

    // Handle special modes first
    if let Some(shell) = &args.shell_completions {
        generate_completions(shell);
        return Ok(());
    }

    if let Some(filter) = &args.list_models {
        list_models(Some(filter.as_str()));
        return Ok(());
    }

    if args.upgrade {
        println!("Upgrade functionality not yet implemented in Rust version");
        return Ok(());
    }

    if args.check_update {
        println!("Version check not yet implemented in Rust version");
        return Ok(());
    }

    if args.gui {
        println!("GUI mode not yet implemented in Rust version");
        return Ok(());
    }

    // Handle project selection mode
    if args.select_project {
        return select_and_launch_project();
    }

    // Find or create git repository
    let git_root = if let Some(ref git_root) = args.git_root {
        Some(git_root.clone())
    } else {
        find_git_root(None)?
    };

    // Initialize IO
    let mut io = InputOutput::new(
        !std::env::var("NO_COLOR").is_ok(),
        args.yes_always,
        Some(args.input_history_file.clone()),
        Some(args.chat_history_file.clone()),
        args.user_input_color.clone(),
        None, // tool_output_color
        args.tool_error_color.clone(),
        args.tool_warning_color.clone(),
        args.assistant_output_color.clone(),
        args.code_theme.clone(),
        args.encoding.clone(),
        args.dry_run,
    );

    // Print version
    if args.verbose {
        io.tool_output(&format!("BerryCode version: {}", berry_editor_tauri::berrycode::VERSION));
    }

    // Setup git repository
    let git_repo = if args.git {
        if let Some(git_root) = git_root {
            Some(setup_git(&git_root, &io)?)
        } else if io.confirm_ask("No git repo found, create one to track berrycode's changes (recommended)?") {
            let current_dir = std::env::current_dir()?;
            Some(GitRepo::init(&current_dir)?)
        } else {
            None
        }
    } else {
        None
    };

    // Print git info
    if let Some(ref repo) = git_repo {
        io.print_git_info(repo.root());
    }

    // Determine model to use
    let model_name = args.model.clone().unwrap_or_else(|| {
        // Default to local Ollama model for implementation
        "qwen3-coder:30b".to_string()
    });

    // Create model
    let model = Model::new(
        model_name.clone(),
        args.weak_model.clone(),
        args.editor_model.clone(),
        args.editor_edit_format.clone(),
        args.verbose,
    )?;

    io.print_model_info(&model.name);

    // Collect files to work with
    let files: Vec<PathBuf> = args.files.clone();
    let read_only_files: Vec<PathBuf> = Vec::new();

    if !files.is_empty() || !read_only_files.is_empty() {
        io.print_file_list(&files, &read_only_files);
    }

    // Create coder
    let mut coder = Coder::new(
        io.clone(),
        model.clone(),
        git_repo,
        files,
        read_only_files,
    );

    // Initialize LLM client with API key
    let api_key = if model.name.contains("gpt") || model.name.contains("o1") || model.name.contains("o3") {
        args.openai_api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    } else if model.name.contains("claude") || model.name.contains("sonnet") || model.name.contains("opus") {
        args.anthropic_api_key.clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
    } else {
        args.openai_api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| args.anthropic_api_key.clone())
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
    };

    if let Some(key) = api_key {
        coder.init_llm(key)?;
    } else {
        io.tool_warning("No API key found. Set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable.");
        io.tool_warning("You can still use commands, but AI features will be disabled.");
    }

    // Set coder options
    coder.auto_commits = args.auto_commits;
    coder.dry_run = args.dry_run;

    // Display welcome screen
    let api_provider = detect_api_provider(&model.name);
    welcome::display_welcome_screen(
        berry_editor_tauri::berrycode::VERSION,
        &model.name,
        api_provider,
    );

    // Run main loop
    coder.run()?;

    Ok(())
}

fn setup_git(git_root: &PathBuf, io: &InputOutput) -> Result<GitRepo> {
    let repo = GitRepo::new(Some(git_root))?;

    // Check and setup git user config
    let user_name = repo.get_config("user.name")?;
    let user_email = repo.get_config("user.email")?;

    if user_name.is_none() {
        repo.set_config("user", "Your Name")?;
        io.tool_warning("Update git name with: git config user.name \"Your Name\"");
    }

    if user_email.is_none() {
        repo.set_config("user.email", "you@example.com")?;
        io.tool_warning("Update git email with: git config user.email \"you@example.com\"");
    }

    Ok(repo)
}

fn generate_completions(shell: &str) {
    use clap::CommandFactory;
    use clap_complete::{generate, Shell};
    use std::io;

    let mut cmd = Args::command();
    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            eprintln!("Unsupported shell: {}", shell);
            return;
        }
    };

    generate(shell, &mut cmd, "berrycode", &mut io::stdout());
}

fn list_models(filter: Option<&str>) {
    let models = berry_editor_tauri::berrycode::models::list_models(filter);
    println!("Available models:");
    for model in models {
        println!("  - {}", model);
    }
}

fn detect_api_provider(model_name: &str) -> &'static str {
    if model_name.contains("gpt") || model_name.contains("o1") || model_name.contains("o3") {
        "OpenAI"
    } else if model_name.contains("claude") || model_name.contains("sonnet") || model_name.contains("opus") {
        "Anthropic"
    } else if model_name.contains("deepseek") {
        "DeepSeek"
    } else if model_name.contains("gemini") {
        "Google"
    } else if model_name.contains("llama") || model_name.contains("mistral") {
        "Ollama/Custom"
    } else {
        "Custom API"
    }
}

/// Select a project from history and launch berrycode in that directory
fn select_and_launch_project() -> Result<()> {
    println!("\n🍓 BerryCode - Project Selector\n");

    // Load project manager
    let mut pm = ProjectManager::new()?;
    let projects = pm.list_projects();

    if projects.is_empty() {
        println!("No projects in history yet.");
        println!("\nTo add a project, open berrycode in a directory:");
        println!("  cd /path/to/your/project");
        println!("  berrycode");
        return Ok(());
    }

    // Display projects
    println!("Recent Projects:\n");
    for (i, project) in projects.iter().enumerate() {
        let git_info = if project.git_status.is_git_repo {
            if let Some(branch) = &project.git_status.branch {
                let dirty = if project.git_status.is_dirty { " (dirty)" } else { "" };
                format!(" [{}{}]", branch, dirty)
            } else {
                " [git]".to_string()
            }
        } else {
            String::new()
        };

        println!("  [{}] {}{}", i + 1, project.name, git_info);
        println!("      {}", project.path.display());

        if let Some(msg) = &project.git_status.last_commit_msg {
            let short_msg = if msg.len() > 60 {
                format!("{}...", &msg[..57])
            } else {
                msg.clone()
            };
            println!("      {}", short_msg);
        }

        println!();
    }

    // Get user selection
    print!("Select project [1-{}] (or 'q' to quit): ", projects.len());
    use std::io::{self, Write};
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input == "q" || input == "quit" || input.is_empty() {
        println!("Cancelled.");
        return Ok(());
    }

    let selection: usize = input.parse()
        .map_err(|_| anyhow::anyhow!("Invalid selection"))?;

    if selection < 1 || selection > projects.len() {
        return Err(anyhow::anyhow!("Selection out of range"));
    }

    let selected_project = &projects[selection - 1];
    println!("\n✓ Selected: {}", selected_project.name);
    println!("  Path: {}", selected_project.path.display());

    // Change to project directory and re-exec berrycode
    std::env::set_current_dir(&selected_project.path)?;

    // Update project's last_opened time
    pm.add_or_update_project(selected_project.path.clone())?;

    println!("\nLaunching BerryCode...\n");

    // Re-exec berrycode without --select-project flag
    let exe = std::env::current_exe()?;
    let args: Vec<String> = std::env::args()
        .filter(|arg| arg != "--select-project" && arg != "-p")
        .collect();

    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(exe)
        .args(&args[1..]) // Skip program name
        .exec();

    // If exec fails, return the error
    Err(anyhow::anyhow!("Failed to exec: {}", err))
}

/// Setup graceful shutdown handler for Ctrl+C
fn setup_signal_handler() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    ctrlc::set_handler(move || {
        if shutdown_clone.load(Ordering::SeqCst) {
            // Second Ctrl+C - force quit
            eprintln!("\n🛑 Force quitting...");
            std::process::exit(1);
        } else {
            // First Ctrl+C - graceful shutdown
            eprintln!("\n🛑 Shutting down gracefully... (Press Ctrl+C again to force quit)");
            shutdown_clone.store(true, Ordering::SeqCst);

            // Perform cleanup
            // TODO: Add cleanup logic here:
            // - Stop running Docker containers
            // - Close file handles
            // - Save state if needed

            std::process::exit(0);
        }
    }).expect("Error setting Ctrl-C handler");

    tracing::info!("Graceful shutdown handler installed");
}
