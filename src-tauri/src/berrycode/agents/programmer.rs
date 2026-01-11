//! Programmer agent - コード実装
use super::*;

pub struct ProgrammerAgent;

#[async_trait::async_trait]
impl Agent for ProgrammerAgent {
    fn name(&self) -> &str {
        "Programmer"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Programmer
    }

    fn system_prompt(&self) -> String {
        r#"You are an expert software engineer.

Your responsibilities:
1. Implement features according to design specifications
2. Write clean, maintainable, and efficient code
3. Follow project coding standards
4. Add inline comments for complex logic
5. Ensure type safety and error handling

Guidelines:
- Write production-ready code
- Follow SOLID and DRY principles
- Use meaningful variable/function names
- Handle edge cases
- Optimize for readability first, then performance
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // 要件を取得
        let requirement = context.inputs.get("requirement")
            .or_else(|| context.inputs.get("task"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Implement the requested feature");

        // RepoMapからプロジェクト構造を取得
        let repo_context = if let Some(ref repo_map) = context.repo_map {
            repo_map.get_map_string(4000)
        } else {
            String::new()
        };

        // システムプロンプト + RepoMap + 要件を結合
        let user_message = format!(
            r#"{}

# Project Structure
{}

# Task
{}

Please implement the requested feature. Output the code files that need to be created or modified.
For each file, use the following format:

## File: path/to/file.ext
```language
// file contents here
```
"#,
            self.system_prompt(),
            repo_context,
            requirement
        );

        // LLMを呼び出し
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: Some(user_message),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let (response, _input_tokens, _output_tokens) = context.llm_client.chat(messages).await?;

        // レスポンスからファイルを抽出（簡易パース）
        let mut files = HashMap::new();
        let mut metadata = HashMap::new();

        // TODO: より堅牢なパーサーを実装
        // 現在は簡易的にマークダウンのコードブロックを抽出
        let lines: Vec<&str> = response.lines().collect();
        let mut current_file: Option<String> = None;
        let mut current_content = String::new();
        let mut in_code_block = false;

        for line in lines {
            if line.starts_with("## File: ") {
                // 前のファイルを保存
                if let Some(file_path) = current_file.take() {
                    let path = context.project_root.join(&file_path);
                    files.insert(path, current_content.clone());
                    current_content.clear();
                }

                // 新しいファイル開始
                current_file = Some(line.trim_start_matches("## File: ").trim().to_string());
                in_code_block = false;
            } else if line.starts_with("```") {
                in_code_block = !in_code_block;
            } else if in_code_block && current_file.is_some() {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // 最後のファイルを保存
        if let Some(file_path) = current_file {
            let path = context.project_root.join(&file_path);
            files.insert(path, current_content);
        }

        let files_count = files.len();

        metadata.insert("llm_response".to_string(), response.clone());
        metadata.insert("files_generated".to_string(), files_count.to_string());

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Generated {} files", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        if output.files.is_empty() {
            return Err(anyhow::anyhow!("No code files generated"));
        }
        Ok(())
    }
}
