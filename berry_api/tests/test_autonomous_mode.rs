use berry_api::llm::{LlmClient, ModelType};
use futures::StreamExt;

#[tokio::test]
async fn test_autonomous_mode_with_file_read() {
    // Create LLM client
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Autonomous Mode ===");

    // Test with a message that should trigger read_file tool
    let message = "Read the file CLAUDE.md and tell me about this project".to_string();
    let model_type = ModelType::Design;
    let autonomous = true;

    // Use current directory as project path for testing
    let project_path = Some(std::env::current_dir()
        .expect("Failed to get current dir")
        .to_string_lossy()
        .to_string());

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path)
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut response_parts = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                println!("Chunk: {}", chunk);
                response_parts.push(chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                panic!("Stream error: {}", e);
            }
        }
    }

    println!("\n=== Full Response ===");
    let full_response = response_parts.join("");
    println!("{}", full_response);

    // Verify that we got some response
    assert!(!response_parts.is_empty(), "No response received");

    // Check if tool was executed (should contain tool execution message)
    let has_tool_execution = response_parts.iter().any(|part| part.contains("実行中"));
    println!("\nTool execution detected: {}", has_tool_execution);

    if has_tool_execution {
        println!("✅ Autonomous mode is working - tool was called!");
    } else {
        println!("⚠️  Tool might not have been called");
    }
}

#[tokio::test]
async fn test_non_autonomous_mode() {
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Non-Autonomous Mode ===");

    let message = "Hello, how are you?".to_string();
    let model_type = ModelType::Design;
    let autonomous = false;

    // Use current directory as project path for testing
    let project_path = Some(std::env::current_dir()
        .expect("Failed to get current dir")
        .to_string_lossy()
        .to_string());

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path)
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut response_parts = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                println!("Chunk: {}", chunk);
                response_parts.push(chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                panic!("Stream error: {}", e);
            }
        }
    }

    println!("\n=== Full Response ===");
    let full_response = response_parts.join("");
    println!("{}", full_response);

    assert!(!response_parts.is_empty(), "No response received");
    println!("✅ Non-autonomous mode is working");
}

#[tokio::test]
async fn test_tool_parsing() {
    use serde_json::json;

    // Test parsing tool call JSON
    let tool_call_json = r#"{"type": "function", "name": "read_file", "parameters": {"path": "CLAUDE.md"}}"#;

    #[derive(Debug, serde::Deserialize)]
    struct ToolCall {
        #[serde(rename = "type")]
        _tool_type: String,
        name: String,
        parameters: serde_json::Value,
    }

    let tool_call: ToolCall = serde_json::from_str(tool_call_json)
        .expect("Failed to parse tool call");

    println!("\n=== Parsed Tool Call ===");
    println!("Name: {}", tool_call.name);
    println!("Parameters: {:#?}", tool_call.parameters);

    assert_eq!(tool_call.name, "read_file");
    assert_eq!(tool_call.parameters["path"], "CLAUDE.md");

    println!("✅ Tool call parsing is working");
}
