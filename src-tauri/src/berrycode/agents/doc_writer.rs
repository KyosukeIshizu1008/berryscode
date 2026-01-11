//! Documentation Writer agent - ドキュメント生成
use super::*;

pub struct DocWriterAgent;

#[async_trait::async_trait]
impl Agent for DocWriterAgent {
    fn name(&self) -> &str {
        "Documentation Writer"
    }

    fn role(&self) -> AgentRole {
        AgentRole::DocWriter
    }

    fn system_prompt(&self) -> String {
        r#"You are a technical writer specializing in software documentation.

Your responsibilities:
1. Write clear README files
2. Create API documentation
3. Document setup and usage instructions
4. Write changelog entries
5. Create architecture diagrams (as text/mermaid)

Documentation guidelines:
- Write for beginners and experts
- Include examples
- Keep it up to date
- Use diagrams where helpful
- Follow project style
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // ドキュメント化対象を取得
        let target = context.inputs.get("code")
            .or_else(|| context.inputs.get("feature"))
            .or_else(|| context.inputs.get("task"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Create comprehensive documentation");

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

# Subject to Document
{}

Please create comprehensive documentation. Include:
1. README.md with setup instructions
2. API documentation (if applicable)
3. Usage examples
4. Architecture diagrams (as text/mermaid)
5. Changelog entries (if applicable)

Guidelines:
- Write for both beginners and experts
- Include practical examples
- Keep it up to date with the code
- Use diagrams where helpful

Output documentation files in the following format:

## File: README.md
```markdown
// README content here
```

## File: docs/API.md
```markdown
// API documentation here
```

## File: CHANGELOG.md
```markdown
// Changelog entry here
```
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
        metadata.insert("doc_files_generated".to_string(), files_count.to_string());

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Generated {} documentation files", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        Ok(())
    }
}
