//! Tests for BerryFlow agents

#[cfg(test)]
mod tests {
    use crate::berrycode::agents::*;
    use std::collections::HashMap;

    #[test]
    fn test_agent_roles() {
        let architect = architect::ArchitectAgent;
        assert_eq!(architect.role(), AgentRole::Architect);
        assert_eq!(architect.name(), "Architect");

        let programmer = programmer::ProgrammerAgent;
        assert_eq!(programmer.role(), AgentRole::Programmer);
        assert_eq!(programmer.name(), "Programmer");

        let qa = qa_engineer::QAEngineerAgent;
        assert_eq!(qa.role(), AgentRole::QAEngineer);
        assert_eq!(qa.name(), "QA Engineer");

        let bug_fixer = bug_fixer::BugFixerAgent;
        assert_eq!(bug_fixer.role(), AgentRole::BugFixer);
        assert_eq!(bug_fixer.name(), "Bug Fixer");

        let refactorer = refactorer::RefactorerAgent;
        assert_eq!(refactorer.role(), AgentRole::Refactorer);
        assert_eq!(refactorer.name(), "Refactorer");

        let doc_writer = doc_writer::DocWriterAgent;
        assert_eq!(doc_writer.role(), AgentRole::DocWriter);
        assert_eq!(doc_writer.name(), "Documentation Writer");
    }

    #[test]
    fn test_agent_system_prompts() {
        let architect = architect::ArchitectAgent;
        let prompt = architect.system_prompt();
        assert!(prompt.contains("software architect"));
        assert!(prompt.contains("system design"));

        let programmer = programmer::ProgrammerAgent;
        let prompt = programmer.system_prompt();
        assert!(prompt.contains("software engineer"));
        assert!(prompt.contains("code"));

        let qa = qa_engineer::QAEngineerAgent;
        let prompt = qa.system_prompt();
        assert!(prompt.contains("QA engineer"));
        assert!(prompt.contains("test"));
    }

    #[test]
    fn test_create_agent_factory() {
        let architect = create_agent(AgentRole::Architect);
        assert_eq!(architect.role(), AgentRole::Architect);

        let programmer = create_agent(AgentRole::Programmer);
        assert_eq!(programmer.role(), AgentRole::Programmer);

        let qa = create_agent(AgentRole::QAEngineer);
        assert_eq!(qa.role(), AgentRole::QAEngineer);
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.model, "claude-sonnet-4-5@20250929");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_agent_output_structure() {
        let mut files = HashMap::new();
        files.insert(std::path::PathBuf::from("test.rs"), "fn main() {}".to_string());

        let mut metadata = HashMap::new();
        metadata.insert("test_key".to_string(), "test_value".to_string());

        let output = AgentOutput {
            files,
            metadata,
            success: true,
            message: "Test completed".to_string(),
        };

        assert_eq!(output.success, true);
        assert_eq!(output.message, "Test completed");
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.metadata.len(), 1);
    }

    #[test]
    fn test_programmer_validation() {
        let programmer = programmer::ProgrammerAgent;

        // Valid output with files
        let mut files = HashMap::new();
        files.insert(std::path::PathBuf::from("src/main.rs"), "fn main() {}".to_string());
        let output = AgentOutput {
            files,
            metadata: HashMap::new(),
            success: true,
            message: "OK".to_string(),
        };
        assert!(programmer.validate_output(&output).is_ok());

        // Invalid output with no files
        let empty_output = AgentOutput {
            files: HashMap::new(),
            metadata: HashMap::new(),
            success: true,
            message: "OK".to_string(),
        };
        assert!(programmer.validate_output(&empty_output).is_err());
    }

    #[test]
    fn test_architect_validation() {
        let architect = architect::ArchitectAgent;

        // Valid output with design files
        let mut files = HashMap::new();
        files.insert(std::path::PathBuf::from("DESIGN.md"), "# Design".to_string());
        let output = AgentOutput {
            files,
            metadata: HashMap::new(),
            success: true,
            message: "OK".to_string(),
        };
        assert!(architect.validate_output(&output).is_ok());

        // Invalid output with no files
        let empty_output = AgentOutput {
            files: HashMap::new(),
            metadata: HashMap::new(),
            success: true,
            message: "OK".to_string(),
        };
        assert!(architect.validate_output(&empty_output).is_err());
    }

    #[test]
    fn test_agent_role_serialization() {
        use serde_json;

        let role = AgentRole::Architect;
        let json = serde_json::to_string(&role).unwrap();
        let deserialized: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(role, deserialized);
    }
}
