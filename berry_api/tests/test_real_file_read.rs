use berry_api::llm::{LlmClient, ModelType};
use futures::StreamExt;

#[tokio::test]
async fn test_read_actual_claude_md() {
    // Create LLM client
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Read CLAUDE.md from parent directory ===");

    // Use parent directory as project path (where CLAUDE.md actually is)
    let project_path = Some("/Users/kyosukeishizu/oracleberry".to_string());

    let message = "Read the file CLAUDE.md and give me a brief summary of what this project is about".to_string();
    let model_type = ModelType::Design;
    let autonomous = true;

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path)
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut response_parts = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                println!("{}", chunk);
                response_parts.push(chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                panic!("Stream error: {}", e);
            }
        }
    }

    println!("\n=== Analysis ===");

    // Check if file was read successfully
    let full_response = response_parts.join("");
    let has_tool_execution = response_parts.iter().any(|part| part.contains("実行中: read_file"));
    let has_file_content = response_parts.iter().any(|part| part.contains("BerryCode") || part.contains("egui"));
    let has_error = full_response.contains("No such file") || full_response.contains("Failed to read");

    println!("Tool execution detected: {}", has_tool_execution);
    println!("File content detected: {}", has_file_content);
    println!("Error detected: {}", has_error);

    assert!(!response_parts.is_empty(), "No response received");
    assert!(has_tool_execution, "Tool was not executed");
    assert!(!has_error, "Failed to read file");

    // Check if AI actually summarized the content
    let final_answer: String = response_parts.iter()
        .filter(|part| !part.contains("実行中") && !part.starts_with("```"))
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("");

    println!("\n=== Final Answer ===");
    println!("{}", final_answer);

    assert!(final_answer.len() > 100, "Answer is too short - AI didn't provide a summary");

    println!("\n✅ Test passed - AI successfully read and summarized CLAUDE.md!");
}

#[tokio::test]
async fn test_multiple_tool_calls() {
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Multiple Tool Calls ===");

    let project_path = Some("/Users/kyosukeishizu/oracleberry".to_string());
    let message = "Read CLAUDE.md and also check what files are in the berry_api/src directory using ls command".to_string();
    let model_type = ModelType::Design;
    let autonomous = true;

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path)
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut tool_call_count = 0;
    let mut response_parts = Vec::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if chunk.contains("実行中:") {
                    tool_call_count += 1;
                }
                println!("{}", chunk);
                response_parts.push(chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                panic!("Stream error: {}", e);
            }
        }
    }

    println!("\n=== Analysis ===");
    println!("Total tool calls: {}", tool_call_count);

    assert!(tool_call_count >= 2, "Expected at least 2 tool calls, got {}", tool_call_count);

    println!("\n✅ Test passed - AI made multiple tool calls!");
}
