//! Bug Fixer agent - バグ修正
use super::*;

pub struct BugFixerAgent;

#[async_trait::async_trait]
impl Agent for BugFixerAgent {
    fn name(&self) -> &str {
        "Bug Fixer"
    }

    fn role(&self) -> AgentRole {
        AgentRole::BugFixer
    }

    fn system_prompt(&self) -> String {
        r#"You are a debugging specialist.

Your responsibilities:
1. Analyze error logs and stack traces
2. Identify root causes of bugs
3. Fix bugs without introducing new issues
4. Add tests to prevent regression

Debugging process:
1. Understand the error
2. Locate the problematic code
3. Identify root cause
4. Apply minimal fix
5. Verify fix doesn't break other functionality
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // エラー情報を取得
        let error_info = context.inputs.get("error")
            .or_else(|| context.inputs.get("stack_trace"))
            .or_else(|| context.inputs.get("test_failures"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Fix bugs in the codebase");

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

# Error Information / Stack Trace
{}

Please analyze the error and fix the bug. Steps:
1. Understand the error message and stack trace
2. Locate the problematic code
3. Identify the root cause
4. Apply minimal fix (don't break other functionality)
5. Explain the fix

Output fixed files in the following format:

## File: path/to/buggy_file.rs
```rust
// fixed code here
```

## Explanation
Brief explanation of what was wrong and how you fixed it.
"#,
            self.system_prompt(),
            repo_context,
            error_info
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
        let mut explanation = String::new();
        let mut in_explanation = false;

        for line in lines {
            if line.starts_with("## File: ") {
                if let Some(file_path) = current_file.take() {
                    let path = context.project_root.join(&file_path);
                    files.insert(path, current_content.clone());
                    current_content.clear();
                }
                current_file = Some(line.trim_start_matches("## File: ").trim().to_string());
                in_code_block = false;
                in_explanation = false;
            } else if line.starts_with("## Explanation") {
                in_explanation = true;
                in_code_block = false;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block && current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            } else if in_explanation {
                explanation.push_str(line);
                explanation.push('\n');
            }
        }

        if let Some(file_path) = current_file {
            let path = context.project_root.join(&file_path);
            files.insert(path, current_content);
        }

        let files_count = files.len();
        metadata.insert("llm_response".to_string(), response);
        metadata.insert("files_fixed".to_string(), files_count.to_string());
        if !explanation.is_empty() {
            metadata.insert("explanation".to_string(), explanation);
        }

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Fixed {} files", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        Ok(())
    }
}
