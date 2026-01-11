//! Architect agent - システム設計・要件定義
use super::*;

pub struct ArchitectAgent;

#[async_trait::async_trait]
impl Agent for ArchitectAgent {
    fn name(&self) -> &str {
        "Architect"
    }

    fn role(&self) -> AgentRole {
        AgentRole::Architect
    }

    fn system_prompt(&self) -> String {
        r#"You are a senior software architect with expertise in system design.

Your responsibilities:
1. Analyze requirements and create technical specifications
2. Design system architecture (modules, layers, dependencies)
3. Create database schemas
4. Define API contracts
5. Choose appropriate technologies and patterns

Output format:
- DESIGN.md: System architecture and technical decisions
- SCHEMA.sql: Database schema (if applicable)
- API.md: API specifications (if applicable)

Best practices:
- Keep designs simple and maintainable
- Follow SOLID principles
- Consider scalability and performance
- Document all design decisions with rationale
"#.to_string()
    }

    async fn execute(&self, context: &AgentContext) -> Result<AgentOutput> {
        use crate::berrycode::llm::Message;

        // 要件を取得
        let requirement = context.inputs.get("requirement")
            .or_else(|| context.inputs.get("task"))
            .or_else(|| context.inputs.get("0"))
            .map(|s| s.as_str())
            .unwrap_or("Design the system architecture");

        // RepoMapからプロジェクト構造を取得
        let repo_context = if let Some(ref repo_map) = context.repo_map {
            repo_map.get_map_string(4000)
        } else {
            String::new()
        };

        // プロンプト作成
        let user_message = format!(
            r#"{}

# Existing Project Structure
{}

# Requirements
{}

Please create a technical design document. Include:
1. System architecture overview
2. Module breakdown and dependencies
3. Database schema (if applicable)
4. API contracts (if applicable)
5. Technical decisions and rationale

Output the design document in markdown format:

## File: DESIGN.md
```markdown
// design document here
```

If a database schema is needed:

## File: schema.sql
```sql
// schema here
```
"#,
            self.system_prompt(),
            repo_context,
            requirement
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
        metadata.insert("files_generated".to_string(), files_count.to_string());

        Ok(AgentOutput {
            files,
            metadata,
            success: true,
            message: format!("Design completed - {} files generated", files_count),
        })
    }

    fn validate_output(&self, output: &AgentOutput) -> Result<()> {
        if output.files.is_empty() {
            return Err(anyhow::anyhow!("No design files generated"));
        }
        Ok(())
    }
}
