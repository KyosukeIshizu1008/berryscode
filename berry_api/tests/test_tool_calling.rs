use serde_json::json;

#[tokio::test]
async fn test_ollama_tool_calling() {
    let client = reqwest::Client::new();

    // Test 1: Simple tool calling with chat API
    println!("\n=== Test 1: Tool calling with /api/chat ===");
    let request = json!({
        "model": "llama4:scout",
        "messages": [
            {
                "role": "user",
                "content": "Read the file CLAUDE.md and tell me about this project"
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

    let response = client
        .post("http://KyosukenoMac-Studio.local:11434/api/chat")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    let status = response.status();
    let body = response.text().await.expect("Failed to read response");

    println!("Status: {}", status);
    println!("Response: {}", body);

    // Parse response
    let json_response: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON");

    println!("Parsed response: {:#?}", json_response);

    if let Some(message) = json_response.get("message") {
        if let Some(content) = message.get("content") {
            let content_str = content.as_str().unwrap();
            println!("\n=== Message content ===");
            println!("{}", content_str);

            // Try to parse as tool call
            if let Ok(tool_call) = serde_json::from_str::<serde_json::Value>(content_str) {
                println!("\n=== Parsed as JSON (potential tool call) ===");
                println!("{:#?}", tool_call);
            }
        }
    }

    assert!(status.is_success(), "Request failed with status: {}", status);
}

#[tokio::test]
async fn test_ollama_models() {
    let client = reqwest::Client::new();

    println!("\n=== Checking available models ===");
    let response = client
        .get("http://KyosukenoMac-Studio.local:11434/api/tags")
        .send()
        .await
        .expect("Failed to get models");

    let body = response.text().await.expect("Failed to read response");
    println!("{}", body);

    let json: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON");

    if let Some(models) = json.get("models") {
        if let Some(models_array) = models.as_array() {
            println!("\nAvailable models:");
            for model in models_array {
                if let Some(name) = model.get("name") {
                    println!("  - {}", name);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_model_capabilities() {
    let client = reqwest::Client::new();

    println!("\n=== Testing llama4:scout capabilities ===");

    // Test with a more explicit instruction
    let request = json!({
        "model": "llama4:scout",
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful assistant with access to tools. When asked to read a file, you MUST use the read_file tool. Always use tools when available."
            },
            {
                "role": "user",
                "content": "Use the read_file tool to read CLAUDE.md"
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

    let response = client
        .post("http://KyosukenoMac-Studio.local:11434/api/chat")
        .json(&request)
        .send()
        .await
        .expect("Failed to send request");

    let body = response.text().await.expect("Failed to read response");
    println!("Response: {}", body);

    let json_response: serde_json::Value = serde_json::from_str(&body)
        .expect("Failed to parse JSON");

    if let Some(message) = json_response.get("message") {
        if let Some(content) = message.get("content") {
            println!("\nContent: {}", content.as_str().unwrap());
        }
    }
}
