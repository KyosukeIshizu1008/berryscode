//! QA Engineer agent - テスト生成
use super::*;

pub struct QAEngineerAgent;

#[async_trait::async_trait]
impl Agent for QAEngineerAgent {
    fn name(&self) -> &str {
        "QA Engineer"
    }

    fn role(&self) -> AgentRole {
        AgentRole::QAEngineer
    }

    fn system_prompt(&self) -> String {
        r#"You are a QA engineer specializing in test automation.

Your responsibilities:
1. Write comprehensive unit tests
2. Create integration tests
3. Cover edge cases and error scenarios
4. Ensure high test coverage
5. Write clear test descriptions

Test guidelines:
- Follow AAA pattern (Arrange, Act, Assert)
- One assertion per test
- Descriptive test names
- Test both happy path and error cases
- Mock external dependencies
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // テスト対象のコードを取得
        let target_code = context.inputs.get("code")
            .or_else(|| context.inputs.get("task"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Generate comprehensive tests");

        // RepoMapからプロジェクト構造を取得
        let repo_context = if let Some(ref repo_map) = context.repo_map {
            repo_map.get_map_string(4000)
        } else {
            String::new()
        };

        // プロンプト作成
        let user_message = format!(
            r#"{}

# Project Structure
{}

# Code to Test
{}

Please generate comprehensive test files. Include:
1. Unit tests for all functions/methods
2. Integration tests for key workflows
3. Edge cases and error scenarios
4. Mock objects for external dependencies

Output test files in the following format:

## File: tests/test_module.rs
```rust
// test code here
```

Or for other languages:

## File: test_module.test.js
```javascript
// test code here
```
"#,
            self.system_prompt(),
            repo_context,
            target_code
        );

        // LLMを呼び出し
        let messages = vec![Message {
            role: "user".to_string(),
            content: Some(user_message),
            tool_calls: None,
            tool_call_id: None,
        }];

        let (response, _input_tokens, _output_tokens) = context.llm_client.chat(messages).await?;

        // ファイルを抽出
        let mut files = HashMap::new();
        let mut metadata = HashMap::new();

        let lines: Vec<&str> = response.lines().collect();
        let mut current_file: Option<String> = None;
        let mut current_content = String::new();
        let mut in_code_block = false;

        for line in lines {
            if line.starts_with("## File: ") {
                if let Some(file_path) = current_file.take() {
                    let path = context.project_root.join(&file_path);
                    files.insert(path, current_content.clone());
                    current_content.clear();
                }
                current_file = Some(line.trim_start_matches("## File: ").trim().to_string());
                in_code_block = false;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block && current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        if let Some(file_path) = current_file {
            let path = context.project_root.join(&file_path);
            files.insert(path, current_content);
        }

        let files_count = files.len();
        metadata.insert("llm_response".to_string(), response);
        metadata.insert("test_files_generated".to_string(), files_count.to_string());

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Generated {} test files", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        Ok(())
    }
}
