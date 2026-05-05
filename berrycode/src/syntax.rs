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
    Comment,    // Normal comments: //, /* */
    DocComment, // Doc comments: //!, ///
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
            "gdscript" | "gd" => self.highlight_gdscript(line),
            "tscn" | "tres" | "godot" => self.highlight_godot_resource(line),
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

        // Comments - distinguish between doc comments (//!, ///) and normal comments (//)
        if let Some(pos) = line.find("//") {
            if pos > 0 {
                self.add_basic_tokens(&mut tokens, &line[..pos], 0);
            }

            // Check if this is a doc comment (//! or ///)
            let comment_text = &line[pos..];
            let is_doc_comment = comment_text.starts_with("//!") || comment_text.starts_with("///");

            tokens.push(SyntaxToken {
                token_type: if is_doc_comment {
                    TokenType::DocComment
                } else {
                    TokenType::Comment
                },
                text: comment_text.to_string(),
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

    /// GDScript (`.gd`) — Godot's built-in scripting language. Comments
    /// share Python's `#` syntax, so the comment-stripping block is
    /// identical, but the keyword set differs enough that we use a
    /// dedicated tokeniser instead of falling through to
    /// `add_basic_tokens`.
    fn highlight_gdscript(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        if let Some(pos) = line.find('#') {
            if pos > 0 {
                self.add_gdscript_tokens(&mut tokens, &line[..pos], 0);
            }
            tokens.push(SyntaxToken {
                token_type: TokenType::Comment,
                text: line[pos..].to_string(),
                start: pos,
                end: line.len(),
            });
            return tokens;
        }

        self.add_gdscript_tokens(&mut tokens, line, 0);
        tokens
    }

    /// Godot resource format — covers `.tscn` (scenes), `.tres`
    /// (resources), and `project.godot` (project config). Format is
    /// INI-like with `[section_header attr="val"]` lines, `key = value`
    /// bodies, and `;` line comments. Constructor calls like
    /// `Vector2(0, 0)` and `ExtResource("id")` get the Function colour
    /// so they're easy to spot when scanning a scene file.
    fn highlight_godot_resource(&self, line: &str) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();

        // `;` line comments (used in `project.godot`; rare in `.tscn`).
        if let Some(pos) = line.find(';') {
            // Only treat `;` as a comment if it's not inside a string.
            // For simplicity we check that no unmatched `"` precedes it
            // on this line — Godot files don't put `;` inside strings
            // in practice, so this is safe.
            let before = &line[..pos];
            let quote_count = before.matches('"').count();
            if quote_count.is_multiple_of(2) {
                if pos > 0 {
                    self.add_godot_resource_tokens(&mut tokens, before, 0);
                }
                tokens.push(SyntaxToken {
                    token_type: TokenType::Comment,
                    text: line[pos..].to_string(),
                    start: pos,
                    end: line.len(),
                });
                return tokens;
            }
        }

        self.add_godot_resource_tokens(&mut tokens, line, 0);
        tokens
    }

    fn add_godot_resource_tokens(&self, tokens: &mut Vec<SyntaxToken>, text: &str, offset: usize) {
        let mut pos = 0;
        let bytes = text.as_bytes();
        while pos < bytes.len() {
            let start = pos;

            if bytes[pos].is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // Section header `[...]` — colour the whole bracketed run
            // as Attribute so it stands out as the structural anchor.
            if bytes[pos] == b'[' {
                let mut depth: i32 = 0;
                let mut in_string = false;
                let mut escape = false;
                while pos < bytes.len() {
                    let c = bytes[pos] as char;
                    if escape {
                        escape = false;
                        pos += 1;
                        continue;
                    }
                    match c {
                        '\\' if in_string => escape = true,
                        '"' => in_string = !in_string,
                        '[' if !in_string => depth += 1,
                        ']' if !in_string => {
                            depth -= 1;
                            pos += 1;
                            if depth == 0 {
                                break;
                            }
                            continue;
                        }
                        _ => {}
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

            // String literals.
            if bytes[pos] == b'"' {
                pos += 1;
                while pos < bytes.len() {
                    if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                        pos += 2;
                    } else if bytes[pos] == b'"' {
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

            // Numbers (incl. negatives and floats).
            if bytes[pos].is_ascii_digit()
                || (bytes[pos] == b'-' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit())
            {
                if bytes[pos] == b'-' {
                    pos += 1;
                }
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_digit() || bytes[pos] == b'.' || bytes[pos] == b'e')
                {
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

            // Identifiers / constructor calls.
            if bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_' {
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric()
                        || bytes[pos] == b'_'
                        || bytes[pos] == b'/')
                {
                    pos += 1;
                }
                let word = &text[start..pos];
                // `Vector2(...)`, `ExtResource("...")`, `PackedStringArray(...)`
                // are constructor-style calls. Detect by the next
                // non-whitespace being `(`.
                let mut peek = pos;
                while peek < bytes.len() && bytes[peek].is_ascii_whitespace() {
                    peek += 1;
                }
                let is_call = peek < bytes.len() && bytes[peek] == b'(';
                let token_type = if is_call {
                    TokenType::Function
                } else if word == "true" || word == "false" || word == "null" {
                    TokenType::Keyword
                } else if word.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                    && word.len() >= 2
                {
                    TokenType::Constant
                } else {
                    TokenType::Identifier
                };
                tokens.push(SyntaxToken {
                    token_type,
                    text: word.to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Operators / punctuation.
            pos += 1;
            tokens.push(SyntaxToken {
                token_type: TokenType::Operator,
                text: text[start..pos].to_string(),
                start: offset + start,
                end: offset + pos,
            });
        }
    }

    /// Tokenise a chunk of GDScript with the language-specific keyword
    /// set. Mirrors `add_basic_tokens` in spirit but with GDScript
    /// vocabulary (`extends`, `func`, `signal`, `@onready`, etc.) and
    /// recognises `@`-prefixed annotations as Attribute tokens so they
    /// stand out the way Rust `#[derive(...)]` does.
    fn add_gdscript_tokens(&self, tokens: &mut Vec<SyntaxToken>, text: &str, offset: usize) {
        // Keywords ordered roughly by frequency. `class_name` and
        // `preload` aren't strictly keywords in Godot's grammar but
        // users treat them as such, so colouring them as keywords
        // matches expectations.
        let keywords = [
            "extends",
            "class_name",
            "class",
            "func",
            "var",
            "const",
            "signal",
            "enum",
            "if",
            "elif",
            "else",
            "for",
            "while",
            "match",
            "return",
            "pass",
            "break",
            "continue",
            "and",
            "or",
            "not",
            "in",
            "is",
            "as",
            "self",
            "super",
            "true",
            "false",
            "null",
            "void",
            "yield",
            "await",
            "static",
            "tool",
            "preload",
            "load",
            "assert",
            "breakpoint",
        ];
        // GDScript ships its own type vocabulary plus the Godot scene-graph
        // node types users see most often. Listing every node type would
        // be unmaintainable, so we cover the primitives and let the
        // common-case scene-graph types fall through as identifiers
        // until a future tree-sitter pass replaces this regex layer.
        let types = [
            "int",
            "float",
            "bool",
            "String",
            "StringName",
            "NodePath",
            "Vector2",
            "Vector2i",
            "Vector3",
            "Vector3i",
            "Color",
            "Rect2",
            "Rect2i",
            "Transform2D",
            "Transform3D",
            "Quaternion",
            "Basis",
            "Plane",
            "AABB",
            "Array",
            "Dictionary",
            "PackedByteArray",
            "PackedInt32Array",
            "PackedInt64Array",
            "PackedFloat32Array",
            "PackedFloat64Array",
            "PackedStringArray",
            "PackedVector2Array",
            "PackedVector3Array",
            "PackedColorArray",
            "Callable",
            "Signal",
            "RID",
            "Object",
            "Variant",
        ];

        let mut pos = 0;
        let bytes = text.as_bytes();
        while pos < bytes.len() {
            let start = pos;

            if bytes[pos].is_ascii_whitespace() {
                pos += 1;
                continue;
            }

            // String literals — single or double quoted, handle escapes.
            if bytes[pos] == b'"' || bytes[pos] == b'\'' {
                let quote = bytes[pos];
                pos += 1;
                while pos < bytes.len() {
                    if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                        pos += 2;
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

            // Numbers (integer or float).
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

            // Annotations: `@onready`, `@export`, `@tool`, etc. Treated
            // as Attribute tokens so they read like Rust `#[...]`.
            if bytes[pos] == b'@' {
                pos += 1;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
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

            // Identifiers / keywords / types.
            if bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_' {
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
                    pos += 1;
                }
                let word = &text[start..pos];
                let token_type = if keywords.contains(&word) {
                    TokenType::Keyword
                } else if types.contains(&word) {
                    TokenType::Type
                } else if word.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                    && word.len() >= 2
                {
                    // SCREAMING_SNAKE_CASE → Constant. Matches Rust
                    // highlighter convention so `GRAVITY` reads the same
                    // way across languages.
                    TokenType::Constant
                } else {
                    TokenType::Identifier
                };
                tokens.push(SyntaxToken {
                    token_type,
                    text: word.to_string(),
                    start: offset + start,
                    end: offset + pos,
                });
                continue;
            }

            // Anything else — operators, punctuation, etc.
            pos += 1;
            tokens.push(SyntaxToken {
                token_type: TokenType::Operator,
                text: text[start..pos].to_string(),
                start: offset + start,
                end: offset + pos,
            });
        }
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
            "String",
            "str",
            "Vec",
            "Option",
            "Result",
            "Box",
            "Rc",
            "Arc",
            "i32",
            "i64",
            "u32",
            "u64",
            "f32",
            "f64",
            "bool",
            "char",
            "usize",
            "isize",
            "TextBuffer",
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
            if bytes[pos] == b'\'' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_alphabetic()
            {
                pos += 1; // Skip '
                let _lifetime_start = pos;
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
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
                while pos < bytes.len()
                    && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_')
                {
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
                let is_constant =
                    word.chars().all(|c| c.is_uppercase() || c == '_') && word.len() > 1;

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
                } else if types.contains(&word)
                    || (!word.is_empty() && word.chars().next().unwrap().is_uppercase())
                {
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
            "color",
            "background",
            "margin",
            "padding",
            "border",
            "width",
            "height",
            "display",
            "position",
            "top",
            "left",
            "right",
            "bottom",
            "flex",
            "grid",
            "font",
            "text",
            "line",
            "letter",
            "word",
            "white",
            "opacity",
            "transform",
            "transition",
            "animation",
            "cursor",
            "overflow",
            "z-index",
            "visibility",
        ];

        while pos < line_bytes.len() {
            // Selectors: . # :: : > + ~
            if line_bytes[pos] == b'.' || line_bytes[pos] == b'#' || line_bytes[pos] == b':' {
                let selector_start = pos;
                pos += 1;

                while pos < line_bytes.len()
                    && (line_bytes[pos].is_ascii_alphanumeric()
                        || line_bytes[pos] == b'-'
                        || line_bytes[pos] == b'_')
                {
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
                        && (line_bytes[pos].is_ascii_digit() || line_bytes[pos] == b'.')
                    {
                        pos += 1;
                    }

                    // CSS units: px, em, rem, %, vh, vw, etc.
                    while pos < line_bytes.len() && line_bytes[pos].is_ascii_alphabetic() {
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
                    && (line_bytes[pos].is_ascii_alphanumeric() || line_bytes[pos] == b'-')
                {
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

    #[test]
    fn highlight_gdscript_extends_keyword() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("extends CharacterBody2D");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Keyword && t.text == "extends"));
    }

    #[test]
    fn highlight_gdscript_annotation() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("@export var speed: float = 200.0");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Attribute && t.text == "@export"));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Keyword && t.text == "var"));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Type && t.text == "float"));
    }

    #[test]
    fn highlight_gdscript_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gdscript").unwrap();
        let tokens = highlighter.highlight_line("var x = 5  # set initial speed");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Comment && t.text.starts_with('#')));
    }

    #[test]
    fn highlight_gdscript_screaming_snake_constant() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("const GRAVITY = 980.0");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Constant && t.text == "GRAVITY"));
    }

    #[test]
    fn highlight_tscn_section_header() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("tscn").unwrap();
        let tokens = highlighter.highlight_line(r#"[node name="Main" type="Node2D"]"#);
        // The whole `[...]` run should be one Attribute token.
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Attribute && t.text.starts_with('[')));
    }

    #[test]
    fn highlight_tscn_constructor_call() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("tscn").unwrap();
        let tokens = highlighter.highlight_line("position = Vector2(640, 360)");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Function && t.text == "Vector2"));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Number && t.text == "640"));
    }

    #[test]
    fn highlight_godot_project_comment() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("godot").unwrap();
        let tokens = highlighter.highlight_line("; Engine configuration file");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Comment && t.text.starts_with(';')));
    }

    #[test]
    fn highlight_gdscript_func_definition() {
        // A typical function signature should produce: keyword `func`,
        // identifier `_physics_process`, type `float` for the param.
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("func _physics_process(delta: float) -> void:");
        let kinds: Vec<&str> = tokens
            .iter()
            .map(|t| match t.token_type {
                TokenType::Keyword => "kw",
                TokenType::Type => "ty",
                TokenType::Identifier => "id",
                _ => "other",
            })
            .collect();
        // Must contain at least one keyword (`func` and `void` both
        // qualify), and `float` should land as a type.
        assert!(kinds.contains(&"kw"));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Type && t.text == "float"));
    }

    #[test]
    fn highlight_gdscript_string_literal() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line(r#"var greeting = "hello world""#);
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::String && t.text == "\"hello world\""));
    }

    #[test]
    fn highlight_gdscript_short_uppercase_word_is_not_constant() {
        // SCREAMING_SNAKE detection requires len >= 2 to avoid
        // mistaking single letters (e.g. a generic param `T`) for
        // constants. Verify the guard.
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("T");
        assert!(tokens.iter().all(|t| t.token_type != TokenType::Constant));
    }

    #[test]
    fn highlight_gdscript_number_with_decimal() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        let tokens = highlighter.highlight_line("var g = 9.81");
        // The `9.81` should be a single Number token, not split at
        // the decimal point.
        let nums: Vec<&SyntaxToken> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Number)
            .collect();
        assert_eq!(nums.len(), 1);
        assert_eq!(nums[0].text, "9.81");
    }

    #[test]
    fn highlight_tscn_negative_number() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("tscn").unwrap();
        let tokens = highlighter.highlight_line("position = Vector2(-1.5, -2)");
        // Both `-1.5` and `-2` should land as Number tokens including
        // their leading minus.
        let neg_floats: Vec<&SyntaxToken> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Number && t.text.starts_with('-'))
            .collect();
        assert_eq!(neg_floats.len(), 2);
        assert!(neg_floats.iter().any(|t| t.text == "-1.5"));
        assert!(neg_floats.iter().any(|t| t.text == "-2"));
    }

    #[test]
    fn highlight_tscn_ext_resource_call() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("tscn").unwrap();
        let tokens = highlighter.highlight_line(r#"script = ExtResource("1_player")"#);
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Function && t.text == "ExtResource"));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::String && t.text == "\"1_player\""));
    }

    #[test]
    fn highlight_tscn_boolean_keywords() {
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("tscn").unwrap();
        let tokens = highlighter.highlight_line("visible = true");
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::Keyword && t.text == "true"));
    }

    #[test]
    fn highlight_godot_semicolon_inside_string_is_not_a_comment() {
        // A `;` that appears inside a quoted string shouldn't be
        // mistaken for the start of a comment. The whole `"a;b"`
        // should remain a single String token, and the body should
        // tokenise normally afterward.
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("godot").unwrap();
        let tokens = highlighter.highlight_line(r#"name="a;b""#);
        assert!(tokens.iter().all(|t| t.token_type != TokenType::Comment));
        assert!(tokens
            .iter()
            .any(|t| t.token_type == TokenType::String && t.text == "\"a;b\""));
    }

    #[test]
    fn highlight_unknown_language_falls_back_to_identifier() {
        // Setting an unrecognised language should yield a single
        // Identifier token containing the whole line, not panic.
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("klingon").unwrap();
        let tokens = highlighter.highlight_line("nuqneH");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].text, "nuqneH");
    }

    #[test]
    fn highlight_gdscript_empty_line_produces_no_tokens() {
        // An empty line should not produce any tokens. (A whitespace-
        // only line will also be empty since the tokeniser skips
        // whitespace.)
        let mut highlighter = SyntaxHighlighter::new();
        highlighter.set_language("gd").unwrap();
        assert!(highlighter.highlight_line("").is_empty());
        assert!(highlighter.highlight_line("    ").is_empty());
    }
}
