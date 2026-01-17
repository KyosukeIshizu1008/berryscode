//! Syntax Highlighting using Regex
//! 100% Rust - No JavaScript!
//! WASM-compatible without tree-sitter


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Keyword,
    Function,
    Type,
    String,
    Number,
    Comment,
    Operator,
    Identifier,
    Macro,          // NEW: println!, vec!, derive! etc.
    Attribute,      // NEW: #[derive(Debug)], #[test] etc.
    Constant,       // NEW: CONST_VALUE, STATIC_VAR etc.
    Lifetime,       // NEW: 'a, 'static etc.
    Namespace,      // NEW: std::collections etc.
    EscapeSequence, // NEW: \n, \t in strings
}

// TokenType has no methods - colors are defined in egui_app.rs ColorTheme

#[derive(Debug, Clone)]
pub struct SyntaxToken {
    pub token_type: TokenType,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone)]
pub struct SyntaxHighlighter {
    current_language: Option<String>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            current_language: None,
        }
    }

    pub fn set_language(&mut self, lang: &str) -> Result<(), String> {
        self.current_language = Some(lang.to_string());
        Ok(())
    }

    pub fn get_language(&self) -> Option<&str> {
        self.current_language.as_deref()
    }

    pub fn highlight_line(&self, line: &str) -> Vec<SyntaxToken> {
        let lang = self.current_language.as_deref().unwrap_or("");

        match lang {
            "rust" | "rs" => self.highlight_rust(line),
            "javascript" | "js" | "typescript" | "ts" => self.highlight_javascript(line),
            "python" | "py" => self.highlight_python(line),
            "html" | "htm" => self.highlight_html(line),
            "css" => self.highlight_css(line),
            _ => vec![SyntaxToken {
                token_type: TokenType::Identifier,
                text: line.to_string(),
                start: 0,
                end: line.len(),
            }],
        }
    }

    fn highlight_rust(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        let _trimmed = line.trim_start();

        // Comments
        if let Some(pos) = line.find("//") {
            if pos > 0 {
                self.add_basic_tokens(&mut tokens, &line[..pos], 0);
            }
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line[pos..].to_string(),
                start: pos,
                end: line.len(),
            });
            return tokens;
        }

        self.add_basic_tokens(&mut tokens, line, 0);
        tokens
    }

    fn highlight_javascript(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // Comments
        if let Some(pos) = line.find("//") {
            if pos > 0 {
                self.add_basic_tokens(&mut tokens, &line[..pos], 0);
            }
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line[pos..].to_string(),
                start: pos,
                end: line.len(),
            });
            return tokens;
        }

        self.add_basic_tokens(&mut tokens, line, 0);
        tokens
    }

    fn highlight_python(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // Comments
        if let Some(pos) = line.find('#') {
            if pos > 0 {
                self.add_basic_tokens(&mut tokens, &line[..pos], 0);
            }
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line[pos..].to_string(),
                start: pos,
                end: line.len(),
            });
            return tokens;
        }

        self.add_basic_tokens(&mut tokens, line, 0);
        tokens
    }

    fn add_basic_tokens(&self, tokens: &mut Vec<SyntaxToken>, text: &str, offset: usize) {
        let keywords = [
            "fn", "let", "mut", "const", "pub", "use", "mod", "impl", "struct", "enum", "trait",
            "type", "if", "else", "match", "for", "while", "loop", "return", "async", "await",
            "move", "self", "Self", "super", "crate", "where", "unsafe", "extern", "static", "ref",
            "dyn", "as", "in", "function", "var", "class", "import", "export", "from", "def",
            "yield", "lambda", "with", "try", "except", "finally",
        ];

        let types = [
            "String", "str", "Vec", "Option", "Result", "Box", "Rc", "Arc", "i32", "i64", "u32",
            "u64", "f32", "f64", "bool", "char", "usize", "isize", "TextBuffer",
        ];

        let mut pos = 0;
        let bytes = text.as_bytes();

        while pos < bytes.len() {
            let start = pos;

            // Skip whitespace
            if bytes[pos].is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // String literals
            if bytes[pos] == b'"' || bytes[pos] == b'\'' {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() {
                    if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                        pos += 2; // Skip escaped character
                    } else if bytes[pos] == quote {
                        pos += 1;
                        break;
                    } else {
                        pos += 1;
                    }
                }
                tokens.push(SyntaxToken {
                    token_type: TokenType::String,
                    text: text[start..pos].to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Numbers
            if bytes[pos].is_ascii_digit() {
                while pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.') {
                    pos += 1;
                }
                tokens.push(SyntaxToken {
                    token_type: TokenType::Number,
                    text: text[start..pos].to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Lifetime ('a, 'static)
            if bytes[pos] == b'\'' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_alphabetic() {
                pos += 1; // Skip '
                let lifetime_start = pos;
                while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                    pos += 1;
                }
                tokens.push(SyntaxToken {
                    token_type: TokenType::Lifetime,
                    text: text[start..pos].to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Attributes (#[derive(Debug)])
            if bytes[pos] == b'#' && pos + 1 < bytes.len() && bytes[pos + 1] == b'[' {
                pos += 2;
                let mut bracket_depth = 1;
                while pos < bytes.len() && bracket_depth > 0 {
                    if bytes[pos] == b'[' {
                        bracket_depth += 1;
                    } else if bytes[pos] == b']' {
                        bracket_depth -= 1;
                    }
                    pos += 1;
                }
                tokens.push(SyntaxToken {
                    token_type: TokenType::Attribute,
                    text: text[start..pos].to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Identifiers and keywords
            if bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_' {
                while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                    pos += 1;
                }
                let word = &text[start..pos];

                // Check if this is a macro (followed by '!')
                let mut temp_pos = pos;
                while temp_pos < bytes.len() && bytes[temp_pos].is_ascii_whitespace() {
                    temp_pos += 1;
                }
                let is_macro = temp_pos < bytes.len() && bytes[temp_pos] == b'!';

                // Check if this is a function call (followed by '(')
                let is_function_call = temp_pos < bytes.len() && bytes[temp_pos] == b'(';

                // Check if this is a constant (all uppercase with optional underscores)
                let is_constant = word.chars().all(|c| c.is_uppercase() || c == '_') && word.len() > 1;

                if keywords.contains(&word) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Keyword,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                } else if is_macro {
                    // Macro (followed by '!')
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Macro,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                } else if is_constant {
                    // Constant (ALL_UPPERCASE)
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Constant,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                } else if is_function_call {
                    // Function call (identifier followed by '(')
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Function,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                } else if types.contains(&word) || (!word.is_empty() && word.chars().next().unwrap().is_uppercase()) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Type,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                } else {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Identifier,
                        text: word.to_string(),
                        start: offset + start,
                        end: offset + pos,
                    });
                }
                continue;
            }

            // Skip other characters (operators, punctuation)
            pos += 1;
        }
    }

    fn highlight_html(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // HTML/XML comments: <!-- ... -->
        if line.trim_start().starts_with("<!--") {
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line.to_string(),
                start: 0,
                end: line.len(),
            });
            return tokens;
        }

        let mut pos = 0;
        let line_bytes = line.as_bytes();

        while pos < line_bytes.len() {
            // HTML Tags: < tag >
            if line_bytes[pos] == b'<' {
                let tag_start = pos;
                pos += 1;

                // Find tag end
                while pos < line_bytes.len() && line_bytes[pos] != b'>' {
                    pos += 1;
                }

                if pos < line_bytes.len() {
                    pos += 1; // Include '>'
                    let tag_text = &line[tag_start..pos];

                    // Distinguish opening/closing tags
                    if tag_text.starts_with("</") || tag_text.starts_with("<!") {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Keyword,
                            text: tag_text.to_string(),
                            start: tag_start,
                            end: pos,
                        });
                    } else {
                        tokens.push(SyntaxToken {
                            token_type: TokenType::Function, // Opening tags as functions
                            text: tag_text.to_string(),
                            start: tag_start,
                            end: pos,
                        });
                    }
                    continue;
                }
            }

            // Strings: "..." or '...'
            if line_bytes[pos] == b'"' || line_bytes[pos] == b'\'' {
                let quote = line_bytes[pos];
                let str_start = pos;
                pos += 1;

                while pos < line_bytes.len() && line_bytes[pos] != quote {
                    if line_bytes[pos] == b'\\' {
                        pos += 2; // Skip escaped character
                    } else {
                        pos += 1;
                    }
                }

                if pos < line_bytes.len() {
                    pos += 1; // Include closing quote
                }

                tokens.push(SyntaxToken {
                    token_type: TokenType::String,
                    text: line[str_start..pos].to_string(),
                    start: str_start,
                    end: pos,
                });
                continue;
            }

            pos += 1;
        }

        if tokens.is_empty() {
            tokens.push(SyntaxToken {
                token_type: TokenType::Identifier,
                text: line.to_string(),
                start: 0,
                end: line.len(),
            });
        }

        tokens
    }

    fn highlight_css(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // CSS comments: /* ... */
        if line.trim_start().starts_with("/*") {
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line.to_string(),
                start: 0,
                end: line.len(),
            });
            return tokens;
        }

        let mut pos = 0;
        let line_bytes = line.as_bytes();

        // CSS selectors, properties, and values
        let css_keywords = [
            "color", "background", "margin", "padding", "border", "width", "height",
            "display", "position", "top", "left", "right", "bottom", "flex", "grid",
            "font", "text", "line", "letter", "word", "white", "opacity", "transform",
            "transition", "animation", "cursor", "overflow", "z-index", "visibility",
        ];

        while pos < line_bytes.len() {
            // Selectors: . # :: : > + ~
            if line_bytes[pos] == b'.' || line_bytes[pos] == b'#' || line_bytes[pos] == b':' {
                let selector_start = pos;
                pos += 1;

                while pos < line_bytes.len()
                    && (line_bytes[pos].is_ascii_alphanumeric() || line_bytes[pos] == b'-' || line_bytes[pos] == b'_') {
                    pos += 1;
                }

                tokens.push(SyntaxToken {
                    token_type: TokenType::Function, // Selectors as functions (yellow)
                    text: line[selector_start..pos].to_string(),
                    start: selector_start,
                    end: pos,
                });
                continue;
            }

            // Strings: "..." or '...'
            if line_bytes[pos] == b'"' || line_bytes[pos] == b'\'' {
                let quote = line_bytes[pos];
                let str_start = pos;
                pos += 1;

                while pos < line_bytes.len() && line_bytes[pos] != quote {
                    if line_bytes[pos] == b'\\' {
                        pos += 2;
                    } else {
                        pos += 1;
                    }
                }

                if pos < line_bytes.len() {
                    pos += 1;
                }

                tokens.push(SyntaxToken {
                    token_type: TokenType::String,
                    text: line[str_start..pos].to_string(),
                    start: str_start,
                    end: pos,
                });
                continue;
            }

            // Numbers with units: 10px, 1.5em, 50%, #FFF, #FFFFFF
            if line_bytes[pos].is_ascii_digit() || line_bytes[pos] == b'#' {
                let num_start = pos;

                if line_bytes[pos] == b'#' {
                    pos += 1;
                    // Hex color: #RGB or #RRGGBB
                    while pos < line_bytes.len() && line_bytes[pos].is_ascii_hexdigit() {
                        pos += 1;
                    }
                } else {
                    // Regular number
                    while pos < line_bytes.len()
                        && (line_bytes[pos].is_ascii_digit() || line_bytes[pos] == b'.') {
                        pos += 1;
                    }

                    // CSS units: px, em, rem, %, vh, vw, etc.
                    while pos < line_bytes.len()
                        && line_bytes[pos].is_ascii_alphabetic() {
                        pos += 1;
                    }
                }

                tokens.push(SyntaxToken {
                    token_type: TokenType::Number,
                    text: line[num_start..pos].to_string(),
                    start: num_start,
                    end: pos,
                });
                continue;
            }

            // Keywords (property names)
            if line_bytes[pos].is_ascii_alphabetic() || line_bytes[pos] == b'-' {
                let word_start = pos;

                while pos < line_bytes.len()
                    && (line_bytes[pos].is_ascii_alphanumeric() || line_bytes[pos] == b'-') {
                    pos += 1;
                }

                let word = &line[word_start..pos];

                if css_keywords.iter().any(|&kw| word.starts_with(kw)) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Keyword,
                        text: word.to_string(),
                        start: word_start,
                        end: pos,
                    });
                } else {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Identifier,
                        text: word.to_string(),
                        start: word_start,
                        end: pos,
                    });
                }
                continue;
            }

            pos += 1;
        }

        if tokens.is_empty() {
            tokens.push(SyntaxToken {
                token_type: TokenType::Identifier,
                text: line.to_string(),
                start: 0,
                end: line.len(),
            });
        }

        tokens
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_highlight_rust_keyword() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("fn main() {");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Keyword && t.text == "fn"));
    }

    #[test]
    fn test_highlight_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("// This is a comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_highlight_type() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x: String = String::new();");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Type && t.text == "String"));
    }

    #[test]
    fn test_javascript_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("javascript").unwrap();

        let tokens = highlighter.highlight_line("// JavaScript comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_python_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("python").unwrap();

        let tokens = highlighter.highlight_line("# Python comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    // Color tests removed - colors are now defined in egui_app.rs ColorTheme

    #[test]
    fn test_no_language_set() {
        let highlighter = SyntaxHighlighter::new();
        let tokens = highlighter.highlight_line("some text");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
    }

    #[test]
    fn test_get_language() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(highlighter.get_language().is_none());

        highlighter.set_language("rust").unwrap();
        assert_eq!(highlighter.get_language(), Some("rust"));
    }

    #[test]
    fn test_partial_line_with_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x = 5; // inline comment");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Keyword));
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Comment));
    }

    #[test]
    fn test_number_highlighting() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x = 42;");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Number));
    }

    #[test]
    fn test_uppercase_type_detection() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("MyCustomType");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Type));
    }
}
