//! Utility functions (standalone, not methods on BerryCodeApp)

use super::types::LspLocation;

/// Get filename from a path
pub fn get_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Get file extension (lowercase)
pub fn get_extension(path: &str) -> String {
    path.rsplit('.').next().unwrap_or("").to_lowercase()
}

/// Filter diagnostics to Rust files only
pub fn filter_rust_diagnostics(
    diags: &[super::types::LspDiagnostic],
) -> Vec<&super::types::LspDiagnostic> {
    diags
        .iter()
        .filter(|d| d.source.as_ref().map_or(true, |s| s.ends_with(".rs")))
        .collect()
}

/// Render a VS Code style tab bar with underline indicator
pub fn render_tab_bar<T: PartialEq + Copy>(ui: &mut egui::Ui, tabs: &[(T, &str)], active: &mut T) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (tab, label) in tabs {
            let selected = *active == *tab;
            let color = if selected {
                super::component_colors::TAB_ACTIVE
            } else {
                super::component_colors::TAB_INACTIVE
            };
            let btn = egui::Button::new(egui::RichText::new(*label).size(11.0).color(color))
                .frame(false)
                .min_size(egui::vec2(0.0, 22.0));
            let resp = ui.add(btn);
            if selected {
                let r = resp.rect;
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(r.left(), r.bottom() - 2.0),
                        egui::vec2(r.width(), 2.0),
                    ),
                    0.0,
                    super::component_colors::ACCENT,
                );
            }
            if resp.clicked() {
                *active = *tab;
            }
            ui.add_space(8.0);
        }
    });
}

/// Strip `<thinking>...</thinking>` blocks from LLM responses.
/// CoT blocks are internal reasoning and should not be shown to users.
pub(crate) fn strip_thinking_blocks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let pos_full = rest.find("<thinking>");
        let pos_short = rest.find("<think>");
        let (open_start, open_tag, close_tag) = match (pos_full, pos_short) {
            (Some(a), Some(b)) if a <= b => (a, "<thinking>", "</thinking>"),
            (Some(a), None) => (a, "<thinking>", "</thinking>"),
            (_, Some(b)) => (b, "<think>", "</think>"),
            (None, None) => break,
        };
        result.push_str(&rest[..open_start]);
        let after_open = &rest[open_start + open_tag.len()..];
        let close_candidates = [
            close_tag,
            if close_tag == "</thinking>" {
                "</think>"
            } else {
                "</thinking>"
            },
        ];
        let mut found = false;
        for &close in &close_candidates {
            if let Some(end) = after_open.find(close) {
                rest = &after_open[end + close.len()..];
                found = true;
                break;
            }
        }
        if !found {
            return result.trim().to_string();
        }
    }
    result.push_str(rest);
    result.trim().to_string()
}

/// Parse lsp_types::Location to LspLocation
pub(crate) fn parse_lsp_location(lsp_loc: lsp_types::Location) -> Option<LspLocation> {
    let file_path = lsp_loc.uri.path().to_string();

    Some(LspLocation {
        file_path,
        line: lsp_loc.range.start.line as usize,
        column: lsp_loc.range.start.character as usize,
    })
}

/// Convert UTF-16 character offset to UTF-8 character offset
pub(crate) fn utf16_offset_to_utf8(line_text: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut utf8_count = 0;

    for ch in line_text.chars() {
        if utf16_count >= utf16_offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_count += 1;
    }

    utf8_count
}

/// Convert UTF-8 character offset to UTF-16 code unit offset
pub(crate) fn utf8_offset_to_utf16(line_text: &str, utf8_offset: usize) -> usize {
    let mut utf16_count = 0;

    for (idx, ch) in line_text.chars().enumerate() {
        if idx >= utf8_offset {
            break;
        }
        utf16_count += ch.len_utf16();
    }

    utf16_count
}

/// Calculate line and column from byte offset in text
pub(crate) fn calculate_line_column(text: &str, pos: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;

    for (i, ch) in text.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_filename() {
        assert_eq!(get_filename("/path/to/file.rs"), "file.rs");
        assert_eq!(get_filename("file.rs"), "file.rs");
        assert_eq!(get_filename("/a/b/c/main.rs"), "main.rs");
        assert_eq!(get_filename(""), "");
    }

    #[test]
    fn test_get_extension() {
        assert_eq!(get_extension("file.rs"), "rs");
        assert_eq!(get_extension("file.RS"), "rs");
        assert_eq!(get_extension("file.tar.gz"), "gz");
        assert_eq!(get_extension("noext"), "noext");
        assert_eq!(get_extension(""), "");
    }

    #[test]
    fn test_strip_thinking_blocks() {
        assert_eq!(
            strip_thinking_blocks("before <thinking>hidden</thinking> after"),
            "before  after"
        );
        assert_eq!(
            strip_thinking_blocks("before <think>hidden</think> after"),
            "before  after"
        );
        assert_eq!(strip_thinking_blocks("no blocks here"), "no blocks here");
        assert_eq!(
            strip_thinking_blocks("<thinking>all hidden</thinking>"),
            ""
        );
        // Unclosed tag
        assert_eq!(
            strip_thinking_blocks("before <thinking>no close"),
            "before"
        );
    }

    #[test]
    fn test_utf16_offset_to_utf8() {
        // ASCII only
        assert_eq!(utf16_offset_to_utf8("hello", 3), 3);
        // Japanese (each char is 1 UTF-16 code unit)
        assert_eq!(utf16_offset_to_utf8("日本語", 2), 2);
        // Emoji (surrogate pair = 2 UTF-16 code units)
        assert_eq!(utf16_offset_to_utf8("a😀b", 1), 1); // 'a'
        assert_eq!(utf16_offset_to_utf8("a😀b", 3), 2); // after emoji
    }

    #[test]
    fn test_utf8_offset_to_utf16() {
        assert_eq!(utf8_offset_to_utf16("hello", 3), 3);
        assert_eq!(utf8_offset_to_utf16("日本語", 2), 2);
        assert_eq!(utf8_offset_to_utf16("a😀b", 1), 1); // 'a'
        assert_eq!(utf8_offset_to_utf16("a😀b", 2), 3); // after emoji (2 UTF-16 units)
    }

    #[test]
    fn test_calculate_line_column() {
        let text = "first\nsecond\nthird";
        assert_eq!(calculate_line_column(text, 0), (0, 0));
        assert_eq!(calculate_line_column(text, 3), (0, 3));
        assert_eq!(calculate_line_column(text, 6), (1, 0)); // start of "second"
        assert_eq!(calculate_line_column(text, 13), (2, 0)); // start of "third"
    }

    #[test]
    fn test_calculate_line_column_empty() {
        assert_eq!(calculate_line_column("", 0), (0, 0));
    }
}
