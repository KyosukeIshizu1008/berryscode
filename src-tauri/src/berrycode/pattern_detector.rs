//! Pattern detector - finds code patterns and templates in the codebase

use std::path::Path;
use std::fs;

#[derive(Debug, Clone)]
pub struct CodePattern {
    pub pattern_type: String,
    pub description: String,
    pub example: String,
    pub file_path: String,
}

pub struct PatternDetector;

impl PatternDetector {
    /// Detect code patterns in the project
    pub fn detect_patterns(project_root: &Path, project_type: &str) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        match project_type {
            "Rust" => {
                patterns.extend(Self::detect_rust_patterns(project_root));
            }
            "JavaScript" | "TypeScript" => {
                patterns.extend(Self::detect_js_patterns(project_root));
            }
            "Python" => {
                patterns.extend(Self::detect_python_patterns(project_root));
            }
            _ => {}
        }

        patterns
    }

    fn detect_rust_patterns(project_root: &Path) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        // Detect error handling patterns
        if let Ok(entries) = glob::glob(&project_root.join("src/**/*.rs").to_string_lossy()) {
            for entry in entries.flatten().take(20) {
                if let Ok(content) = fs::read_to_string(&entry) {
                    // Detect Result<T> usage
                    if content.contains("Result<") {
                        let example = Self::extract_function_with_pattern(&content, "Result<");
                        if !example.is_empty() && patterns.iter().all(|p: &CodePattern| p.pattern_type != "error_handling") {
                            patterns.push(CodePattern {
                                pattern_type: "error_handling".to_string(),
                                description: "エラーハンドリングパターン (Result<T>)".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    // Detect async patterns
                    if content.contains("async fn") {
                        let example = Self::extract_function_with_pattern(&content, "async fn");
                        if !example.is_empty() && patterns.iter().all(|p| p.pattern_type != "async") {
                            patterns.push(CodePattern {
                                pattern_type: "async".to_string(),
                                description: "非同期処理パターン (async/await)".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    // Detect struct patterns
                    if content.contains("struct") && content.contains("#[derive") {
                        let example = Self::extract_struct_pattern(&content);
                        if !example.is_empty() && patterns.iter().all(|p| p.pattern_type != "struct") {
                            patterns.push(CodePattern {
                                pattern_type: "struct".to_string(),
                                description: "構造体定義パターン".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    if patterns.len() >= 5 {
                        break;
                    }
                }
            }
        }

        patterns
    }

    fn detect_js_patterns(project_root: &Path) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        if let Ok(entries) = glob::glob(&project_root.join("src/**/*.{js,jsx,ts,tsx}").to_string_lossy()) {
            for entry in entries.flatten().take(20) {
                if let Ok(content) = fs::read_to_string(&entry) {
                    // Detect React components
                    if content.contains("function") && (content.contains("return (") || content.contains("return(")) {
                        let example = Self::extract_function_with_pattern(&content, "function");
                        if !example.is_empty() && patterns.iter().all(|p: &CodePattern| p.pattern_type != "react_component") {
                            patterns.push(CodePattern {
                                pattern_type: "react_component".to_string(),
                                description: "Reactコンポーネントパターン".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    // Detect hooks
                    if content.contains("useState") || content.contains("useEffect") {
                        let example = Self::extract_hook_pattern(&content);
                        if !example.is_empty() && patterns.iter().all(|p| p.pattern_type != "hooks") {
                            patterns.push(CodePattern {
                                pattern_type: "hooks".to_string(),
                                description: "React Hooksパターン".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    if patterns.len() >= 5 {
                        break;
                    }
                }
            }
        }

        patterns
    }

    fn detect_python_patterns(project_root: &Path) -> Vec<CodePattern> {
        let mut patterns = Vec::new();

        if let Ok(entries) = glob::glob(&project_root.join("**/*.py").to_string_lossy()) {
            for entry in entries.flatten().take(20) {
                if let Ok(content) = fs::read_to_string(&entry) {
                    // Detect class patterns
                    if content.contains("class") {
                        let example = Self::extract_class_pattern(&content);
                        if !example.is_empty() && patterns.iter().all(|p: &CodePattern| p.pattern_type != "class") {
                            patterns.push(CodePattern {
                                pattern_type: "class".to_string(),
                                description: "クラス定義パターン".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    // Detect async patterns
                    if content.contains("async def") {
                        let example = Self::extract_function_with_pattern(&content, "async def");
                        if !example.is_empty() && patterns.iter().all(|p| p.pattern_type != "async") {
                            patterns.push(CodePattern {
                                pattern_type: "async".to_string(),
                                description: "非同期処理パターン (async/await)".to_string(),
                                example,
                                file_path: entry.to_string_lossy().to_string(),
                            });
                        }
                    }

                    if patterns.len() >= 5 {
                        break;
                    }
                }
            }
        }

        patterns
    }

    fn extract_function_with_pattern(content: &str, pattern: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains(pattern) {
                // Extract function/method (up to 10 lines)
                let end = (i + 10).min(lines.len());
                let mut function_lines = Vec::new();

                for j in i..end {
                    function_lines.push(lines[j]);
                    // Stop at closing brace at same indentation level
                    if j > i && lines[j].trim() == "}" {
                        break;
                    }
                }

                return function_lines.join("\n");
            }
        }

        String::new()
    }

    fn extract_struct_pattern(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("#[derive") {
                // Extract struct with derive (up to 15 lines)
                let end = (i + 15).min(lines.len());
                let mut struct_lines = Vec::new();

                for j in i..end {
                    struct_lines.push(lines[j]);
                    if lines[j].trim() == "}" {
                        break;
                    }
                }

                return struct_lines.join("\n");
            }
        }

        String::new()
    }

    fn extract_class_pattern(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("class ") {
                // Extract class (up to 15 lines)
                let end = (i + 15).min(lines.len());
                let class_lines: Vec<&str> = lines[i..end].to_vec();
                return class_lines.join("\n");
            }
        }

        String::new()
    }

    fn extract_hook_pattern(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("useState") || line.contains("useEffect") {
                // Extract surrounding context (3 lines before and after)
                let start = i.saturating_sub(3);
                let end = (i + 4).min(lines.len());
                return lines[start..end].join("\n");
            }
        }

        String::new()
    }

    pub fn patterns_to_prompt(patterns: &[CodePattern]) -> String {
        if patterns.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("# プロジェクトのコードパターン\n\n");
        prompt.push_str("既存のコードベースでは以下のパターンが使用されています。新しいコードを書く際は、これらのパターンに従って一貫性を保ってください:\n\n");

        for (i, pattern) in patterns.iter().enumerate() {
            prompt.push_str(&format!("{}. {} ({})\n", i + 1, pattern.description, pattern.file_path));
            prompt.push_str("```\n");
            prompt.push_str(&pattern.example);
            prompt.push_str("\n```\n\n");
        }

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_with_pattern() {
        let content = r#"
fn main() {
    println!("Hello");
}

async fn test() -> Result<()> {
    Ok(())
}
        "#;

        let result = PatternDetector::extract_function_with_pattern(content, "async fn");
        assert!(result.contains("async fn"));
        assert!(result.contains("Result"));
    }

    #[test]
    fn test_extract_struct_pattern() {
        let content = r#"
#[derive(Debug, Clone)]
struct MyStruct {
    field1: String,
    field2: i32,
}
        "#;

        let result = PatternDetector::extract_struct_pattern(content);
        assert!(result.contains("#[derive"));
        assert!(result.contains("struct MyStruct"));
    }

    #[test]
    fn test_extract_class_pattern() {
        let content = r#"
class MyClass:
    def __init__(self):
        self.value = 0

    def method(self):
        pass
        "#;

        let result = PatternDetector::extract_class_pattern(content);
        assert!(result.contains("class MyClass"));
        assert!(result.contains("__init__"));
    }

    #[test]
    fn test_extract_hook_pattern() {
        let content = r#"
function MyComponent() {
    const [state, setState] = useState(0);

    useEffect(() => {
        console.log('mounted');
    }, []);
}
        "#;

        let result = PatternDetector::extract_hook_pattern(content);
        assert!(result.contains("useState"));
    }

    #[test]
    fn test_patterns_to_prompt_empty() {
        let patterns = vec![];
        let prompt = PatternDetector::patterns_to_prompt(&patterns);
        assert_eq!(prompt, "");
    }

    #[test]
    fn test_patterns_to_prompt() {
        let patterns = vec![
            CodePattern {
                pattern_type: "error_handling".to_string(),
                description: "エラーハンドリングパターン".to_string(),
                example: "fn test() -> Result<()>".to_string(),
                file_path: "src/test.rs".to_string(),
            },
        ];

        let prompt = PatternDetector::patterns_to_prompt(&patterns);
        assert!(prompt.contains("プロジェクトのコードパターン"));
        assert!(prompt.contains("エラーハンドリングパターン"));
        assert!(prompt.contains("src/test.rs"));
        assert!(prompt.contains("```"));
    }

    #[test]
    fn test_code_pattern_creation() {
        let pattern = CodePattern {
            pattern_type: "test".to_string(),
            description: "Test pattern".to_string(),
            example: "example code".to_string(),
            file_path: "test.rs".to_string(),
        };

        assert_eq!(pattern.pattern_type, "test");
        assert_eq!(pattern.description, "Test pattern");
    }
}
