//! User onboarding utilities

use crate::berrycode::Result;
use crate::berrycode::io::InputOutput;
use dialoguer::{Select, Confirm};
use anyhow::anyhow;

pub fn offer_openrouter_oauth() -> Result<bool> {
    // Offer OpenRouter OAuth flow for users without API keys
    let mut io = InputOutput::new(
        false, // pretty
        None,  // yes_always
        None,  // input_history_file
        None,  // chat_history_file
        None,  // user_input_color
        None,  // tool_output_color
        None,  // tool_error_color
        None,  // tool_warning_color
        None,  // assistant_output_color
        "default".to_string(), // code_theme
        "utf-8".to_string(),   // encoding
        false, // dry_run
    );

    io.tool_output("\n=== API Key Setup ===");
    io.tool_output("You don't have an API key configured.");
    io.tool_output("\nOptions:");
    io.tool_output("1. Use OpenRouter (supports multiple models with one key)");
    io.tool_output("2. Use OpenAI API key");
    io.tool_output("3. Use Anthropic API key");
    io.tool_output("4. Skip for now");

    if !Confirm::new()
        .with_prompt("Would you like to set up an API key now?")
        .default(true)
        .interact()?
    {
        return Ok(false);
    }

    let choices = vec![
        "OpenRouter (Recommended - Multiple models, one key)",
        "OpenAI API",
        "Anthropic API",
        "Skip",
    ];

    let selection = Select::new()
        .with_prompt("Choose your API provider")
        .items(&choices)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // OpenRouter OAuth flow
            io.tool_output("\n=== OpenRouter Setup ===");
            io.tool_output("1. Visit: https://openrouter.ai/keys");
            io.tool_output("2. Sign in or create an account");
            io.tool_output("3. Generate a new API key");
            io.tool_output("4. Copy your API key");

            let key = io.get_input("\nPaste your OpenRouter API key (or press Enter to skip): ")?;
            if !key.trim().is_empty() {
                std::env::set_var("OPENROUTER_API_KEY", key.trim());
                io.tool_output("✓ OpenRouter API key configured!");
                return Ok(true);
            }
        }
        1 => {
            // OpenAI setup
            io.tool_output("\n=== OpenAI Setup ===");
            io.tool_output("1. Visit: https://platform.openai.com/api-keys");
            io.tool_output("2. Create a new secret key");
            io.tool_output("3. Copy your API key");

            let key = io.get_input("\nPaste your OpenAI API key (or press Enter to skip): ")?;
            if !key.trim().is_empty() {
                std::env::set_var("OPENAI_API_KEY", key.trim());
                io.tool_output("✓ OpenAI API key configured!");
                return Ok(true);
            }
        }
        2 => {
            // Anthropic setup
            io.tool_output("\n=== Anthropic Setup ===");
            io.tool_output("1. Visit: https://console.anthropic.com/settings/keys");
            io.tool_output("2. Create a new API key");
            io.tool_output("3. Copy your API key");

            let key = io.get_input("\nPaste your Anthropic API key (or press Enter to skip): ")?;
            if !key.trim().is_empty() {
                std::env::set_var("ANTHROPIC_API_KEY", key.trim());
                io.tool_output("✓ Anthropic API key configured!");
                return Ok(true);
            }
        }
        _ => {}
    }

    Ok(false)
}

pub fn select_default_model() -> Result<Option<String>> {
    let io = InputOutput::new(
        false, // pretty
        None,  // yes_always
        None,  // input_history_file
        None,  // chat_history_file
        None,  // user_input_color
        None,  // tool_output_color
        None,  // tool_error_color
        None,  // tool_warning_color
        None,  // assistant_output_color
        "default".to_string(), // code_theme
        "utf-8".to_string(),   // encoding
        false, // dry_run
    );

    io.tool_output("\n=== Model Selection ===");

    // Group models by provider
    let openai_models = vec![
        ("gpt-4o", "GPT-4o - Latest, fast, multimodal"),
        ("gpt-4o-mini", "GPT-4o Mini - Fast and affordable"),
        ("gpt-4-turbo", "GPT-4 Turbo - Large context window"),
        ("gpt-3.5-turbo", "GPT-3.5 Turbo - Fast and economical"),
    ];

    let anthropic_models = vec![
        ("claude-3.5-sonnet", "Claude 3.5 Sonnet - Best for coding"),
        ("claude-3-opus", "Claude 3 Opus - Most capable"),
        ("claude-3-sonnet", "Claude 3 Sonnet - Balanced"),
        ("claude-3-haiku", "Claude 3 Haiku - Fast and affordable"),
    ];

    // Check which API keys are available
    let has_openai = std::env::var("OPENAI_API_KEY").is_ok()
        || std::env::var("OPENROUTER_API_KEY").is_ok();
    let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok()
        || std::env::var("OPENROUTER_API_KEY").is_ok();

    if !has_openai && !has_anthropic {
        return Err(anyhow!("No API key found. Please configure an API key first."));
    }

    let mut all_choices = Vec::new();
    let mut model_names = Vec::new();

    if has_openai {
        io.tool_output("\nOpenAI Models:");
        for (name, desc) in &openai_models {
            all_choices.push(desc.to_string());
            model_names.push(name.to_string());
        }
    }

    if has_anthropic {
        io.tool_output("\nAnthropic Models:");
        for (name, desc) in &anthropic_models {
            all_choices.push(desc.to_string());
            model_names.push(name.to_string());
        }
    }

    all_choices.push("Skip - I'll specify later".to_string());

    let selection = Select::new()
        .with_prompt("Choose your default model")
        .items(&all_choices)
        .default(0)
        .interact()?;

    if selection < model_names.len() {
        let selected_model = model_names[selection].clone();
        io.tool_output(&format!("\n✓ Selected model: {}", selected_model));
        Ok(Some(selected_model))
    } else {
        Ok(None)
    }
}
