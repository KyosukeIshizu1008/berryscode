use berry_api::llm::{LlmClient, ModelType};
use futures::StreamExt;
use std::path::Path;

#[tokio::test]
async fn test_autonomous_code_writing() {
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Autonomous Code Writing ===");

    // Use /tmp for testing to avoid polluting the project
    let test_dir = "/tmp/berry_test_autonomous";
    std::fs::create_dir_all(test_dir).expect("Failed to create test dir");

    let project_path = Some(test_dir.to_string());

    // Ask AI to write a simple Rust function
    let message = "Create a file called hello.rs with a simple Rust function that prints 'Hello, Autonomous AI!'".to_string();
    let model_type = ModelType::Coding; // Use coding model for code generation
    let autonomous = true;

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path.clone())
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut response_parts = Vec::new();
    let mut write_file_called = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if chunk.contains("実行中: write_file") {
                    write_file_called = true;
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
    println!("write_file called: {}", write_file_called);

    // Check if the file was actually created
    let file_path = Path::new(test_dir).join("hello.rs");
    let file_exists = file_path.exists();
    println!("File created: {}", file_exists);

    if file_exists {
        let content = std::fs::read_to_string(&file_path)
            .expect("Failed to read created file");
        println!("\n=== Created File Content ===");
        println!("{}", content);

        // Verify the content contains expected elements
        assert!(content.contains("fn") || content.contains("main"), "File should contain a function");
        assert!(content.contains("Hello") || content.contains("print"), "File should contain print statement");

        // Cleanup
        std::fs::remove_file(&file_path).ok();
    }

    std::fs::remove_dir(test_dir).ok();

    assert!(write_file_called, "write_file tool should have been called");
    assert!(file_exists, "File should have been created");

    println!("\n✅ Test passed - AI autonomously wrote code!");
}

#[tokio::test]
async fn test_autonomous_bug_fix() {
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Autonomous Bug Fixing ===");

    let test_dir = "/tmp/berry_test_bugfix";
    std::fs::create_dir_all(test_dir).expect("Failed to create test dir");

    // Create a buggy file
    let buggy_code = r#"fn add(a: i32, b: i32) -> i32 {
    a - b  // Bug: should be + not -
}

fn main() {
    println!("2 + 3 = {}", add(2, 3));
}
"#;
    let buggy_file = Path::new(test_dir).join("buggy.rs");
    std::fs::write(&buggy_file, buggy_code).expect("Failed to write buggy file");

    let project_path = Some(test_dir.to_string());

    // Ask AI to find and fix the bug
    let message = "Read the file buggy.rs, find the bug in the add function (it should add numbers but it's doing something wrong), and fix it by writing the corrected version".to_string();
    let model_type = ModelType::Coding;
    let autonomous = true;

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path.clone())
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut read_file_called = false;
    let mut write_file_called = false;

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if chunk.contains("実行中: read_file") {
                    read_file_called = true;
                }
                if chunk.contains("実行中: write_file") {
                    write_file_called = true;
                }
                println!("{}", chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    println!("\n=== Analysis ===");
    println!("read_file called: {}", read_file_called);
    println!("write_file called: {}", write_file_called);

    // Check if the file was fixed
    if buggy_file.exists() {
        let fixed_content = std::fs::read_to_string(&buggy_file)
            .expect("Failed to read fixed file");
        println!("\n=== Fixed File Content ===");
        println!("{}", fixed_content);

        // Check if the bug was fixed
        let bug_fixed = fixed_content.contains("a + b") ||
                       (fixed_content.contains('+') && !fixed_content.contains("a - b"));
        println!("Bug fixed: {}", bug_fixed);

        if bug_fixed {
            println!("✅ AI successfully identified and fixed the bug!");
        } else {
            println!("⚠️  Bug might not have been fixed");
        }
    }

    // Cleanup
    std::fs::remove_file(&buggy_file).ok();
    std::fs::remove_dir(test_dir).ok();

    assert!(read_file_called, "read_file should have been called to read the buggy code");
    assert!(write_file_called || read_file_called, "AI should have at least read the file");

    println!("\n✅ Test completed!");
}

#[tokio::test]
async fn test_autonomous_multi_step_task() {
    let client = LlmClient::new().expect("Failed to create LLM client");

    println!("\n=== Testing Multi-Step Autonomous Task ===");

    let test_dir = "/tmp/berry_test_multistep";
    std::fs::create_dir_all(test_dir).expect("Failed to create test dir");

    let project_path = Some(test_dir.to_string());

    // Complex task requiring multiple tools
    let message = "Create a simple Rust library with:
1. A file lib.rs with a function `calculate(x: i32) -> i32` that returns x * 2
2. A file test.rs that tests this function
Then use the execute_command tool to check if the files were created with 'ls -la'".to_string();

    let model_type = ModelType::Coding;
    let autonomous = true;

    let mut stream = client
        .chat_stream(message, model_type, autonomous, project_path.clone())
        .await
        .expect("Failed to create chat stream");

    println!("\n=== Streaming Response ===");
    let mut tool_counts = std::collections::HashMap::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                // Count tool calls
                if chunk.contains("実行中: write_file") {
                    *tool_counts.entry("write_file").or_insert(0) += 1;
                }
                if chunk.contains("実行中: execute_command") {
                    *tool_counts.entry("execute_command").or_insert(0) += 1;
                }
                println!("{}", chunk);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    println!("\n=== Tool Call Summary ===");
    for (tool, count) in &tool_counts {
        println!("{}: {} times", tool, count);
    }

    // Check if files were created
    let lib_file = Path::new(test_dir).join("lib.rs");
    let test_file = Path::new(test_dir).join("test.rs");

    println!("\n=== Files Created ===");
    println!("lib.rs exists: {}", lib_file.exists());
    println!("test.rs exists: {}", test_file.exists());

    if lib_file.exists() {
        let content = std::fs::read_to_string(&lib_file).expect("Failed to read lib.rs");
        println!("\nlib.rs content:");
        println!("{}", content);
    }

    if test_file.exists() {
        let content = std::fs::read_to_string(&test_file).expect("Failed to read test.rs");
        println!("\ntest.rs content:");
        println!("{}", content);
    }

    // Cleanup
    std::fs::remove_file(&lib_file).ok();
    std::fs::remove_file(&test_file).ok();
    std::fs::remove_dir(test_dir).ok();

    let write_count = tool_counts.get("write_file").unwrap_or(&0);
    println!("\n=== Final Assessment ===");
    println!("Total write_file calls: {}", write_count);

    if *write_count >= 2 {
        println!("✅ AI successfully completed multi-step task!");
    } else {
        println!("⚠️  AI might not have completed all steps");
    }

    println!("\n✅ Multi-step test completed!");
}
