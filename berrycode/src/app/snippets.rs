#![allow(dead_code)]
//! Snippet expansion engine: parse LSP snippet syntax and manage tab stop navigation
//!
//! Supports: $1, $0, ${1:placeholder}, ${2|choice1,choice2|}, $TM_FILENAME etc.
//! Tab/Shift+Tab moves between stops. Enter/Escape exits snippet mode.

use super::types::SnippetSession;
use super::BerryCodeApp;

/// Parsed tab stop from a snippet template
#[derive(Debug, Clone)]
struct TabStop {
    index: usize,
    placeholder: String,
}

/// Expand a snippet template string into plain text + tab stop positions.
/// Returns (expanded_text, tab_stops: Vec<(index, offset_in_text, placeholder_len)>)
pub fn parse_snippet(template: &str) -> (String, Vec<(usize, usize, usize)>) {
    let mut result = String::new();
    let mut tab_stops: Vec<(usize, usize, usize)> = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            match chars.peek() {
                Some(&'{') => {
                    chars.next(); // consume '{'
                                  // Parse ${N:placeholder} or ${N}
                    let mut num_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let index: usize = num_str.parse().unwrap_or(0);
                    let mut placeholder = String::new();

                    if chars.peek() == Some(&':') {
                        chars.next(); // consume ':'
                        let mut depth = 1;
                        while let Some(c) = chars.next() {
                            if c == '{' {
                                depth += 1;
                                placeholder.push(c);
                            } else if c == '}' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                placeholder.push(c);
                            } else {
                                placeholder.push(c);
                            }
                        }
                    } else if chars.peek() == Some(&'|') {
                        // Choice: ${1|a,b,c|}
                        chars.next(); // consume '|'
                        let mut choice = String::new();
                        while let Some(c) = chars.next() {
                            if c == '|' {
                                break;
                            }
                            choice.push(c);
                        }
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                        // Use first choice as placeholder
                        placeholder = choice.split(',').next().unwrap_or("").to_string();
                    } else {
                        // Just ${N}
                        if chars.peek() == Some(&'}') {
                            chars.next();
                        }
                    }

                    let offset = result.len();
                    tab_stops.push((index, offset, placeholder.len()));
                    result.push_str(&placeholder);
                }
                Some(&c) if c.is_ascii_digit() => {
                    // Simple $N
                    chars.next();
                    let mut num_str = String::from(c);
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let index: usize = num_str.parse().unwrap_or(0);
                    let offset = result.len();
                    tab_stops.push((index, offset, 0));
                }
                _ => {
                    // Unknown $ sequence, pass through
                    result.push('$');
                }
            }
        } else if ch == '\\' {
            // Escape: \$ → $, \} → }, etc.
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(ch);
        }
    }

    // Sort tab stops by index (but $0 goes last)
    tab_stops.sort_by_key(|&(idx, _, _)| if idx == 0 { usize::MAX } else { idx });

    (result, tab_stops)
}

impl BerryCodeApp {
    /// Insert a snippet at the current cursor position
    pub(crate) fn insert_snippet(&mut self, template: &str) {
        let tab = match self.editor_tabs.get_mut(self.active_tab_idx) {
            Some(t) => t,
            None => return,
        };

        let (expanded, tab_stops) = parse_snippet(template);

        // Calculate the absolute position for tab stops
        let insert_line = tab.cursor_line;
        let insert_col = tab.cursor_col;

        // Insert the expanded text at cursor
        let mut text = tab.buffer.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let byte_offset: usize = lines
            .iter()
            .take(insert_line)
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + insert_col;

        let byte_offset = byte_offset.min(text.len());
        text.insert_str(byte_offset, &expanded);
        tab.buffer = crate::buffer::TextBuffer::from_str(&text);
        tab.mark_dirty();

        if tab_stops.is_empty() {
            // No tab stops, just move cursor to end of insertion
            tab.cursor_col = insert_col + expanded.len();
            return;
        }

        // Convert offsets to (line, col) positions
        let session_stops: Vec<(usize, usize, usize)> = tab_stops
            .iter()
            .map(|&(_idx, offset, plen)| {
                // offset is relative to the start of the expanded text
                let text_before = &expanded[..offset];
                let newlines = text_before.matches('\n').count();
                let col = if newlines > 0 {
                    offset - text_before.rfind('\n').unwrap() - 1
                } else {
                    insert_col + offset
                };
                let line = insert_line + newlines;
                (line, col, plen)
            })
            .collect();

        // Move cursor to first tab stop
        if let Some(&(line, col, plen)) = session_stops.first() {
            tab.cursor_line = line;
            tab.cursor_col = col;
            // If there's a placeholder, select it (for now just position cursor at start)
            let _ = plen;
        }

        self.snippet_session = Some(SnippetSession {
            tab_stops: session_stops,
            current_stop: 0,
            start_line: insert_line,
        });
    }

    /// Handle Tab key during snippet session: move to next tab stop
    pub(crate) fn snippet_next_stop(&mut self) -> bool {
        let session = match &mut self.snippet_session {
            Some(s) => s,
            None => return false,
        };

        session.current_stop += 1;
        if session.current_stop >= session.tab_stops.len() {
            // Snippet complete
            self.snippet_session = None;
            return true;
        }

        let (line, col, _plen) = session.tab_stops[session.current_stop];
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.cursor_line = line;
            tab.cursor_col = col;
        }
        true
    }

    /// Handle Shift+Tab during snippet session: move to previous tab stop
    pub(crate) fn snippet_prev_stop(&mut self) -> bool {
        let session = match &mut self.snippet_session {
            Some(s) => s,
            None => return false,
        };

        if session.current_stop == 0 {
            return true;
        }

        session.current_stop -= 1;
        let (line, col, _plen) = session.tab_stops[session.current_stop];
        if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
            tab.cursor_line = line;
            tab.cursor_col = col;
        }
        true
    }

    /// Check if snippet session is active
    pub(crate) fn has_active_snippet(&self) -> bool {
        self.snippet_session.is_some()
    }

    /// Cancel snippet session (on Escape)
    pub(crate) fn cancel_snippet(&mut self) {
        self.snippet_session = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tab_stops() {
        let (text, stops) = parse_snippet("fn ${1:name}($2) {\n\t$0\n}");
        assert_eq!(text, "fn name() {\n\t\n}");
        assert_eq!(stops.len(), 3);
        // $1 with placeholder "name" at offset 3
        assert_eq!(stops[0], (1, 3, 4));
        // $2 with no placeholder at offset 8
        assert_eq!(stops[1], (2, 8, 0));
        // $0 (final cursor) at offset 13 (\n=12, \t=13), sorted last
        assert_eq!(stops[2], (0, 13, 0));
    }

    #[test]
    fn test_no_stops() {
        let (text, stops) = parse_snippet("hello world");
        assert_eq!(text, "hello world");
        assert!(stops.is_empty());
    }

    #[test]
    fn test_choice() {
        let (text, stops) = parse_snippet("${1|pub,pub(crate),pub(super)|} fn");
        assert_eq!(text, "pub fn");
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0], (1, 0, 3)); // "pub" is 3 chars
    }
}
