//! Common validation utilities
//!
//! Input validation helpers to avoid duplication.

/// Validate file path format
pub fn is_valid_file_path(path: &str) -> bool {
    !path.is_empty() && !path.contains('\0')
}

/// Validate identifier (for variable names, etc.)
pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Validate line and column numbers (must be > 0)
pub fn is_valid_position(line: u32, column: u32) -> bool {
    line > 0 && column > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_file_path() {
        assert!(is_valid_file_path("/foo/bar.rs"));
        assert!(is_valid_file_path("relative/path.txt"));
        assert!(!is_valid_file_path(""));
        assert!(!is_valid_file_path("path\0with\0null"));
    }

    #[test]
    fn test_valid_identifier() {
        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("snake_case"));
        assert!(is_valid_identifier("camelCase"));
        assert!(is_valid_identifier("PascalCase"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123invalid"));
        assert!(!is_valid_identifier("invalid-name"));
        assert!(!is_valid_identifier("invalid name"));
    }

    #[test]
    fn test_valid_position() {
        assert!(is_valid_position(1, 1));
        assert!(is_valid_position(100, 50));
        assert!(!is_valid_position(0, 1));
        assert!(!is_valid_position(1, 0));
        assert!(!is_valid_position(0, 0));
    }
}
