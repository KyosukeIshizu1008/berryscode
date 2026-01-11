//! Refactorer agent - コードリファクタリング
use super::*;

pub struct RefactorerAgent;

#[async_trait::async_trait]
impl Agent for RefactorerAgent {
    fn name(&self) -> &str {
        "Refactorer"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Refactorer
    }

    fn system_prompt(&self) -> String {
        r#"You are a code quality specialist.

Your responsibilities:
1. Improve code readability
2. Eliminate code smells
3. Apply design patterns where appropriate
4. Optimize performance
5. Maintain behavior (no functional changes)

Refactoring principles:
- Red-Green-Refactor (tests must pass before and after)
- Small, incremental changes
- Extract methods/classes
- Reduce complexity
- Improve naming
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // リファクタリング対象を取得
        let target = context.inputs.get("code")
            .or_else(|| context.inputs.get("file"))
            .or_else(|| context.inputs.get("task"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Refactor and improve code quality");

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

# Code to Refactor
{}

Please refactor the code to improve quality. Focus on:
1. Improve readability and naming
2. Eliminate code smells
3. Apply design patterns where appropriate
4. Reduce complexity
5. Maintain behavior (no functional changes)

Important: Ensure all tests still pass after refactoring.

Output refactored files in the following format:

## File: path/to/file.rs
```rust
// refactored code here
```

## Changes Made
- Brief list of changes
- Rationale for each change
"#,
            self.system_prompt(),
            repo_context,
            target
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
        let mut changes = String::new();
        let mut in_changes = false;

        for line in lines {
            if line.starts_with("## File: ") {
                if let Some(file_path) = current_file.take() {
                    let path = context.project_root.join(&file_path);
                    files.insert(path, current_content.clone());
                    current_content.clear();
                }
                current_file = Some(line.trim_start_matches("## File: ").trim().to_string());
                in_code_block = false;
                in_changes = false;
            } else if line.starts_with("## Changes Made") {
                in_changes = true;
                in_code_block = false;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block && current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            } else if in_changes {
                changes.push_str(line);
                changes.push('\n');
            }
        }

        if let Some(file_path) = current_file {
            let path = context.project_root.join(&file_path);
            files.insert(path, current_content);
        }

        let files_count = files.len();
        metadata.insert("llm_response".to_string(), response);
        metadata.insert("files_refactored".to_string(), files_count.to_string());
        if !changes.is_empty() {
            metadata.insert("changes_made".to_string(), changes);
        }

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Refactored {} files", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        Ok(())
    }
}
