/// Test to see what Ollama actually returns for tool calling

#[tokio::test]
async fn test_ollama_direct_tool_call() {
    let client = reqwest::Client::new();

    // Strong system prompt
    let request = serde_json::json!({
        "model": "qwen3-coder:30b",
        "messages": [
            {
                "role": "system",
                "content": "You MUST use the read_file tool when asked to read a file. NEVER just say you will read it - ACTUALLY call the tool."
            },
            {
                "role": "user",
                "content": "Read the file buggy.rs and tell me what's in it"
            }
        ],
        "stream": false,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read the contents of a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "The file path to read"
                            }
                        },
                        "required": ["path"]
                    }
                }
            }
        ]
    });

    println!("\n=== Sending request to Ollama ===");
    println!("{}", serde_json::to_string_pretty(&request).unwrap());

    let response = client
        .post("http://KyosukenoMac-Studio.local:11434/api/chat")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read response");

    println!("\n=== Ollama Response ===");
    println!("{}", body);

    let json_response: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse JSON");

    if let Some(message) = json_response.get("message") {
        if let Some(content) = message.get("content") {
            println!("\n=== Message Content ===");
            println!("{}", content.as_str().unwrap());

            // Check if it's a tool call
            let is_tool_call = content.as_str().unwrap().contains("<function=")
                || content.as_str().unwrap().contains("{\"type\"");

            println!("\n=== Analysis ===");
            println!("Is tool call: {}", is_tool_call);

            if is_tool_call {
                println!("✅ Model returned a tool call!");
            } else {
                println!("❌ Model did NOT return a tool call - just text");
            }
        }
    }
}

#[tokio::test]
async fn test_simple_create_file_request() {
    let client = reqwest::Client::new();

    let request = serde_json::json!({
        "model": "qwen3-coder:30b",
        "messages": [
            {
                "role": "system",
                "content": "You MUST use tools. When asked to create a file, call write_file immediately."
            },
            {
                "role": "user",
                "content": "Create a file test.txt with content 'Hello World'"
            }
        ],
        "stream": false,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "write_file",
                    "description": "Write content to a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string"
                            },
                            "content": {
                                "type": "string"
                            }
                        },
                        "required": ["path", "content"]
                    }
                }
            }
        ]
    });

    println!("\n=== Testing Simple Create File ===");

    let response = client
        .post("http://KyosukenoMac-Studio.local:11434/api/chat")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read response");

    println!("\n=== Response ===");
    println!("{}", body);

    let json_response: serde_json::Value = serde_json::from_str(&body).unwrap();
    if let Some(message) = json_response.get("message") {
        if let Some(content) = message.get("content") {
            let content_str = content.as_str().unwrap();
            println!("\nContent: {}", content_str);

            let is_tool_call =
                content_str.contains("<function=") || content_str.contains("{\"type\"");
            println!("Is tool call: {}", is_tool_call);
        }
    }
}
