//! Dependency tracker - analyzes imports and suggests related files

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashSet;
use anyhow::Result;

pub struct DependencyTracker;

impl DependencyTracker {
    /// Extract dependencies from a file based on its language
    pub fn extract_dependencies(file_path: &Path, content: &str) -> Vec<String> {
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "rs" => Self::extract_rust_imports(content),
            "js" | "jsx" | "ts" | "tsx" => Self::extract_js_imports(content),
            "py" => Self::extract_python_imports(content),
            "go" => Self::extract_go_imports(content),
            _ => Vec::new(),
        }
    }

    /// Extract Rust imports (use statements and mod declarations)
    fn extract_rust_imports(content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // use statements: use crate::berrycode::module::Item;
            if trimmed.starts_with("use ") {
                if let Some(module_path) = Self::parse_rust_use(trimmed) {
                    imports.push(module_path);
                }
            }

            // mod declarations: mod module_name;
            if trimmed.starts_with("mod ") && trimmed.ends_with(';') {
                if let Some(module) = trimmed.strip_prefix("mod ")
                    .and_then(|s| s.strip_suffix(';')) {
                    imports.push(format!("{}.rs", module.trim()));
                }
            }
        }

        imports
    }

    fn parse_rust_use(use_stmt: &str) -> Option<String> {
        // use crate::berrycode::foo::bar; -> foo/bar.rs
        // use super::foo; -> ../foo.rs
        // use self::foo; -> ./foo.rs

        let without_use = use_stmt.strip_prefix("use ")?.trim();
        // Remove semicolon and any trailing content after it
        let without_suffix = without_use
            .split(';')
            .next()?
            .split('{') // Remove brace imports like use foo::{bar, baz};
            .next()?
            .trim();

        let path_part = without_suffix.split("::").collect::<Vec<_>>();

        if path_part.is_empty() {
            return None;
        }

        match path_part[0] {
            "crate" => {
                // crate::berrycode::foo::bar -> src/foo/bar.rs
                if path_part.len() > 1 {
                    let module_path = path_part[1..].join("/");
                    Some(format!("src/{}.rs", module_path))
                } else {
                    None
                }
            }
            "super" => {
                // super::foo -> ../foo.rs
                if path_part.len() > 1 {
                    Some(format!("../{}.rs", path_part[1]))
                } else {
                    None
                }
            }
            "self" => {
                // self::foo -> ./foo.rs
                if path_part.len() > 1 {
                    Some(format!("./{}.rs", path_part[1]))
                } else {
                    None
                }
            }
            _ => None, // External crates
        }
    }

    /// Extract JavaScript/TypeScript imports
    fn extract_js_imports(content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // import ... from './module'
            if trimmed.starts_with("import ") && trimmed.contains(" from ") {
                if let Some(path) = Self::parse_js_import(trimmed) {
                    imports.push(path);
                }
            }

            // const x = require('./module')
            if trimmed.contains("require(") {
                if let Some(path) = Self::parse_js_require(trimmed) {
                    imports.push(path);
                }
            }
        }

        imports
    }

    fn parse_js_import(import_stmt: &str) -> Option<String> {
        // import foo from './bar' -> bar.js/bar.ts/bar.tsx
        let parts: Vec<&str> = import_stmt.split(" from ").collect();
        if parts.len() < 2 {
            return None;
        }

        let path_part = parts[1].trim()
            .trim_matches(|c| c == '\'' || c == '"' || c == ';');

        // Only process relative imports
        if path_part.starts_with("./") || path_part.starts_with("../") {
            Some(Self::normalize_js_path(path_part))
        } else {
            None
        }
    }

    fn parse_js_require(require_stmt: &str) -> Option<String> {
        // require('./foo') -> foo.js
        let start = require_stmt.find("require(")?;
        let path_start = start + "require(".len();
        let remaining = &require_stmt[path_start..];

        let quote_char = remaining.chars().next()?;
        if quote_char != '\'' && quote_char != '"' {
            return None;
        }

        let path_end = remaining[1..].find(quote_char)?;
        let path = &remaining[1..path_end + 1];

        if path.starts_with("./") || path.starts_with("../") {
            Some(Self::normalize_js_path(path))
        } else {
            None
        }
    }

    fn normalize_js_path(path: &str) -> String {
        // Try common extensions
        for ext in &["", ".js", ".jsx", ".ts", ".tsx", "/index.js", "/index.ts"] {
            let with_ext = format!("{}{}", path, ext);
            // Return first that might exist
            return with_ext;
        }
        path.to_string()
    }

    /// Extract Python imports
    fn extract_python_imports(content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // from .module import something
            if trimmed.starts_with("from .") && trimmed.contains(" import ") {
                if let Some(module) = trimmed.strip_prefix("from .")
                    .and_then(|s| s.split(" import ").next()) {
                    let module_path = module.trim().replace('.', "/");
                    imports.push(format!("{}.py", module_path));
                }
            }

            // import .module
            if trimmed.starts_with("import .") {
                if let Some(module) = trimmed.strip_prefix("import .") {
                    let module_name = module.split_whitespace().next().unwrap_or("");
                    imports.push(format!("{}.py", module_name));
                }
            }
        }

        imports
    }

    /// Extract Go imports
    fn extract_go_imports(content: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let mut in_import_block = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "import (" {
                in_import_block = true;
                continue;
            }

            if in_import_block && trimmed == ")" {
                in_import_block = false;
                continue;
            }

            if in_import_block || trimmed.starts_with("import ") {
                // Only process local imports (starting with ./ or ../)
                let import_path = trimmed
                    .trim_start_matches("import ")
                    .trim_matches(|c| c == '"' || c == '\'');

                if import_path.starts_with("./") || import_path.starts_with("../") {
                    imports.push(format!("{}.go", import_path));
                }
            }
        }

        imports
    }

    /// Get related files that should be read when editing a file
    pub fn get_related_files(file_path: &Path, project_root: &Path) -> Result<Vec<PathBuf>> {
        let content = fs::read_to_string(file_path)?;
        let dependencies = Self::extract_dependencies(file_path, &content);

        let mut related = HashSet::new();
        let file_dir = file_path.parent().unwrap_or(project_root);

        for dep in dependencies {
            // Resolve relative paths
            let dep_path = if dep.starts_with("./") || dep.starts_with("../") {
                file_dir.join(&dep)
            } else {
                project_root.join(&dep)
            };

            // Normalize and check existence
            if let Ok(canonical) = dep_path.canonicalize() {
                if canonical.exists() && canonical.is_file() {
                    related.insert(canonical);
                }
            }

            // For JS/TS, try multiple extensions
            if !dep_path.exists() {
                for ext in &["", ".js", ".jsx", ".ts", ".tsx", "/index.js", "/index.ts", "/index.tsx"] {
                    let with_ext = PathBuf::from(format!("{}{}", dep_path.display(), ext));
                    if let Ok(canonical) = with_ext.canonicalize() {
                        if canonical.exists() && canonical.is_file() {
                            related.insert(canonical);
                            break;
                        }
                    }
                }
            }
        }

        Ok(related.into_iter().collect())
    }

    /// Generate context message for related files
    pub fn generate_context_message(related_files: &[PathBuf], project_root: &Path) -> String {
        if related_files.is_empty() {
            return String::new();
        }

        let mut msg = String::from("\n\n💡 このファイルは以下のファイルに依存しています。変更時は影響を考慮してください:\n");

        for (i, file) in related_files.iter().enumerate() {
            if let Ok(relative) = file.strip_prefix(project_root) {
                msg.push_str(&format!("  {}. {}\n", i + 1, relative.display()));
            }
        }

        msg.push_str("\n関連ファイルも確認が必要な場合は、read_file()で読み込んでください。\n");

        msg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_imports() {
        let content = r#"
            use crate::berrycode::foo::bar;
            use super::baz;
            use std::collections::HashMap;
            mod submodule;
        "#;

        let imports = DependencyTracker::extract_rust_imports(content);
        assert!(imports.contains(&"src/foo/bar.rs".to_string()));
        assert!(imports.contains(&"../baz.rs".to_string()));
        assert!(imports.contains(&"submodule.rs".to_string()));
    }

    #[test]
    fn test_js_imports() {
        let content = r#"
            import React from 'react';
            import { foo } from './utils/foo';
            import Bar from '../components/Bar';
            const baz = require('./baz');
        "#;

        let imports = DependencyTracker::extract_js_imports(content);
        assert!(imports.iter().any(|i| i.contains("./utils/foo")));
        assert!(imports.iter().any(|i| i.contains("../components/Bar")));
        assert!(imports.iter().any(|i| i.contains("./baz")));
    }
}
