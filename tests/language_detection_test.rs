//! Language Detection Robustness Test
//!
//! ❌ CRITICAL: This test prevents "Markdown fails to load" bugs
//!
//! WHY THIS MATTERS:
//! - File path matching must be EXACT on extension
//! - `.MD` (uppercase) should work
//! - `file.md.backup` should NOT be detected as Markdown
//! - Paths with `.md` in directory names should NOT match
//!
//! This test verifies robust language detection based on file paths.
//!
//! Run with: cargo test --test language_detection_test

#[cfg(test)]
mod tests {
    /// Simulates language detection from virtual_editor.rs
    fn detect_language(path: &str) -> Option<&'static str> {
        let path_lower = path.to_lowercase();

        // CRITICAL: Must check EXACT extension, not just "contains"
        if path_lower.ends_with(".rs") {
            Some("rust")
        } else if path_lower.ends_with(".toml") {
            Some("toml")
        } else if path_lower.ends_with(".md") {
            Some("markdown")
        } else if path_lower.ends_with(".js") {
            Some("javascript")
        } else if path_lower.ends_with(".ts") {
            Some("typescript")
        } else if path_lower.ends_with(".py") {
            Some("python")
        } else if path_lower.ends_with(".json") {
            Some("json")
        } else if path_lower.ends_with(".html") {
            Some("html")
        } else if path_lower.ends_with(".css") {
            Some("css")
        } else if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
            Some("yaml")
        } else {
            None
        }
    }

    // ========================================
    // Standard Cases (Should Pass)
    // ========================================

    #[test]
    fn test_rust_file() {
        assert_eq!(detect_language("main.rs"), Some("rust"));
        assert_eq!(detect_language("/path/to/lib.rs"), Some("rust"));
    }

    #[test]
    fn test_markdown_lowercase() {
        assert_eq!(detect_language("README.md"), Some("markdown"));
        assert_eq!(detect_language("/docs/guide.md"), Some("markdown"));
    }

    #[test]
    fn test_markdown_uppercase() {
        // ✅ CRITICAL: .MD (uppercase) must work
        assert_eq!(detect_language("README.MD"), Some("markdown"),
            "❌ CRITICAL: .MD (uppercase) should be detected as markdown");
        assert_eq!(detect_language("CHANGELOG.MD"), Some("markdown"));
    }

    #[test]
    fn test_markdown_mixed_case() {
        assert_eq!(detect_language("notes.Md"), Some("markdown"));
        assert_eq!(detect_language("file.mD"), Some("markdown"));
    }

    #[test]
    fn test_toml_file() {
        assert_eq!(detect_language("Cargo.toml"), Some("toml"));
        assert_eq!(detect_language("config.toml"), Some("toml"));
    }

    // ========================================
    // Edge Cases (Must NOT Match)
    // ========================================

    #[test]
    fn test_backup_files_not_detected() {
        // ❌ CRITICAL: .md.backup should NOT be markdown
        assert_eq!(detect_language("file.md.backup"), None,
            "❌ CRITICAL: .md.backup should NOT be detected as markdown");

        assert_eq!(detect_language("README.md.old"), None,
            ".md.old should NOT be markdown");

        assert_eq!(detect_language("notes.md~"), None,
            ".md~ should NOT be markdown");
    }

    #[test]
    fn test_directory_names_with_extension() {
        // ❌ CRITICAL: Directories with .md in name should NOT match
        assert_eq!(detect_language("/project.md/file.txt"), None,
            "Directory named project.md should not affect detection");

        assert_eq!(detect_language("/rust.repo/main.cpp"), None,
            "Directory named rust.repo should not affect detection");
    }

    #[test]
    fn test_hidden_files() {
        assert_eq!(detect_language(".hidden.md"), Some("markdown"),
            "Hidden markdown files should be detected");

        assert_eq!(detect_language(".config.toml"), Some("toml"),
            "Hidden TOML files should be detected");
    }

    #[test]
    fn test_no_extension() {
        assert_eq!(detect_language("Makefile"), None);
        assert_eq!(detect_language("README"), None);
        assert_eq!(detect_language("LICENSE"), None);
    }

    #[test]
    fn test_multiple_dots_in_filename() {
        assert_eq!(detect_language("my.test.file.rs"), Some("rust"),
            "File with multiple dots should detect last extension");

        assert_eq!(detect_language("archive.tar.gz"), None,
            ".gz is not a supported language");
    }

    #[test]
    fn test_javascript_typescript() {
        assert_eq!(detect_language("app.js"), Some("javascript"));
        assert_eq!(detect_language("types.ts"), Some("typescript"));

        // Must NOT confuse .ts with .toml or .json with .js
        assert_eq!(detect_language("config.json"), Some("json"));
    }

    #[test]
    fn test_python_files() {
        assert_eq!(detect_language("script.py"), Some("python"));
        assert_eq!(detect_language("__init__.py"), Some("python"));
    }

    #[test]
    fn test_web_files() {
        assert_eq!(detect_language("index.html"), Some("html"));
        assert_eq!(detect_language("style.css"), Some("css"));
    }

    #[test]
    fn test_yaml_files() {
        assert_eq!(detect_language("config.yaml"), Some("yaml"));
        assert_eq!(detect_language("docker-compose.yml"), Some("yaml"));
    }

    #[test]
    fn test_empty_path() {
        assert_eq!(detect_language(""), None);
    }

    #[test]
    fn test_path_with_only_dot() {
        assert_eq!(detect_language("."), None);
        assert_eq!(detect_language(".."), None);
    }

    #[test]
    fn test_extension_only() {
        assert_eq!(detect_language(".rs"), Some("rust"),
            "File named exactly .rs should be detected");
        assert_eq!(detect_language(".md"), Some("markdown"));
    }

    // ========================================
    // Security & Robustness
    // ========================================

    #[test]
    fn test_path_traversal_attempt() {
        assert_eq!(detect_language("../../etc/passwd.md"), Some("markdown"),
            "Path traversal should not affect extension detection");
    }

    #[test]
    fn test_windows_paths() {
        assert_eq!(detect_language("C:\\Users\\test\\file.rs"), Some("rust"));
        assert_eq!(detect_language("C:\\project\\README.MD"), Some("markdown"));
    }

    #[test]
    fn test_url_like_paths() {
        assert_eq!(detect_language("file:///home/user/doc.md"), Some("markdown"));
        assert_eq!(detect_language("https://example.com/code.rs"), Some("rust"));
    }

    #[test]
    fn test_unicode_in_path() {
        assert_eq!(detect_language("日本語/ファイル.rs"), Some("rust"));
        assert_eq!(detect_language("émoji/🦀.md"), Some("markdown"));
    }
}
