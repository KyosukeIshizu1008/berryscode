//! Syntax Highlighting using Regex
//! 100% Rust - No JavaScript!
//! WASM-compatible without tree-sitter

use regex::Regex;

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
}

impl TokenType {
    pub fn to_color(&self) -> &'static str {
        match self {
            // IntelliJ IDEA / RustRover Darcula color scheme
            TokenType::Keyword => "#CC7832", // Orange (pub, struct, impl, fn)
            TokenType::Function => "#FFC66D", // Yellow-orange (function names)
            TokenType::Type => "#A9B7C6",    // Light gray (Position, usize, Self)
            TokenType::String => "#6A8759",  // Green (string literals)
            TokenType::Number => "#6897BB",  // Blue (numeric literals)
            TokenType::Comment => "#629755", // Green (comments)
            TokenType::Operator => "#A9B7C6", // Light gray (operators)
            TokenType::Identifier => "#A9B7C6", // Light gray (identifiers)
        }
    }

    pub fn to_class(&self) -> &'static str {
        match self {
            TokenType::Keyword => "syntax-keyword",
            TokenType::Function => "syntax-function",
            TokenType::Type => "syntax-type",
            TokenType::String => "syntax-string",
            TokenType::Number => "syntax-number",
            TokenType::Comment => "syntax-comment",
            TokenType::Operator => "syntax-operator",
            TokenType::Identifier => "syntax-identifier",
        }
    }
}

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
        let trimmed = line.trim_start();

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
            "class", "return", "yield", "lambda", "with", "try", "except", "finally",
        ];

        let types = [
            "String", "str", "Vec", "Option", "Result", "Box", "Rc", "Arc", "i32", "i64", "u32",
            "u64", "f32", "f64", "bool", "char", "usize",
        ];

        let mut current_pos = 0;
        let words: Vec<&str> = text.split_whitespace().collect();

        for word in words {
            if let Some(word_start) = text[current_pos..].find(word) {
                let absolute_start = current_pos + word_start;
                let absolute_end = absolute_start + word.len();

                // Check if it's a keyword
                if keywords.contains(&word) {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Keyword,
                        text: word.to_string(),
                        start: offset + absolute_start,
                        end: offset + absolute_end,
                    });
                }
                // Check if it's a type
                else if types.contains(&word)
                    || (word.len() > 0 && word.chars().next().unwrap().is_uppercase())
                {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Type,
                        text: word.to_string(),
                        start: offset + absolute_start,
                        end: offset + absolute_end,
                    });
                }
                // Check if it's a string
                else if word.starts_with('"') || word.starts_with('\'') {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::String,
                        text: word.to_string(),
                        start: offset + absolute_start,
                        end: offset + absolute_end,
                    });
                }
                // Check if it's a number (strip trailing punctuation)
                else if {
                    let stripped =
                        word.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '.');
                    !stripped.is_empty() && stripped.chars().all(|c| c.is_numeric() || c == '.')
                } {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Number,
                        text: word.to_string(),
                        start: offset + absolute_start,
                        end: offset + absolute_end,
                    });
                }
                // Identifier
                else {
                    tokens.push(SyntaxToken {
                        token_type: TokenType::Identifier,
                        text: word.to_string(),
                        start: offset + absolute_start,
                        end: offset + absolute_end,
                    });
                }

                current_pos = absolute_end;
            }
        }

        // If no tokens were created, add the whole line as identifier
        if tokens.is_empty() && !text.is_empty() {
            tokens.push(SyntaxToken {
                token_type: TokenType::Identifier,
                text: text.to_string(),
                start: offset,
                end: offset + text.len(),
            });
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_highlight_rust_keyword() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("fn main() {");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Keyword && t.text == "fn"));
    }

    #[wasm_bindgen_test]
    fn test_highlight_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("// This is a comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[wasm_bindgen_test]
    fn test_highlight_type() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x: String = String::new();");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Type && t.text == "String"));
    }

    #[wasm_bindgen_test]
    fn test_javascript_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("javascript").unwrap();

        let tokens = highlighter.highlight_line("// JavaScript comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[wasm_bindgen_test]
    fn test_python_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("python").unwrap();

        let tokens = highlighter.highlight_line("# Python comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[wasm_bindgen_test]
    fn test_token_type_colors() {
        // IntelliJ IDEA / RustRover Darcula color scheme
        assert_eq!(TokenType::Keyword.to_color(), "#CC7832");
        assert_eq!(TokenType::Function.to_color(), "#FFC66D");
        assert_eq!(TokenType::Type.to_color(), "#A9B7C6");
        assert_eq!(TokenType::String.to_color(), "#6A8759");
        assert_eq!(TokenType::Number.to_color(), "#6897BB");
        assert_eq!(TokenType::Comment.to_color(), "#629755");
    }

    #[wasm_bindgen_test]
    fn test_token_type_classes() {
        assert_eq!(TokenType::Keyword.to_class(), "syntax-keyword");
        assert_eq!(TokenType::Function.to_class(), "syntax-function");
        assert_eq!(TokenType::Type.to_class(), "syntax-type");
        assert_eq!(TokenType::String.to_class(), "syntax-string");
    }

    #[wasm_bindgen_test]
    fn test_no_language_set() {
        let highlighter = SyntaxHighlighter::new();
        let tokens = highlighter.highlight_line("some text");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
    }

    #[wasm_bindgen_test]
    fn test_get_language() {
        let mut highlighter = SyntaxHighlighter::new();
        assert!(highlighter.get_language().is_none());

        highlighter.set_language("rust").unwrap();
        assert_eq!(highlighter.get_language(), Some("rust"));
    }

    #[wasm_bindgen_test]
    fn test_partial_line_with_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x = 5; // inline comment");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Keyword));
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Comment));
    }

    #[wasm_bindgen_test]
    fn test_number_highlighting() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("let x = 42;");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Number));
    }

    #[wasm_bindgen_test]
    fn test_uppercase_type_detection() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("rust").unwrap();

        let tokens = highlighter.highlight_line("MyCustomType");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Type));
    }
}
