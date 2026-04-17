//! ANSI escape sequence parser for terminal output colorization

/// A segment of text with associated ANSI color
#[derive(Debug, Clone)]
pub struct AnsiSegment {
    pub text: String,
    pub color: egui::Color32,
    pub bold: bool,
}

/// Parse a string containing ANSI escape codes into colored segments
pub fn parse_ansi(input: &str) -> Vec<AnsiSegment> {
    let mut segments = Vec::new();
    let mut current_color = egui::Color32::from_rgb(204, 204, 204); // default light gray
    let mut current_bold = false;
    let mut current_text = String::new();

    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        // Check for ESC character
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                // Flush current text
                if !current_text.is_empty() {
                    segments.push(AnsiSegment {
                        text: current_text.clone(),
                        color: current_color,
                        bold: current_bold,
                    });
                    current_text.clear();
                }

                chars.next(); // consume '['

                // Parse the CSI parameters
                let mut params = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == 'm'
                        || next_ch == 'K'
                        || next_ch == 'H'
                        || next_ch == 'J'
                        || next_ch == 'A'
                        || next_ch == 'B'
                        || next_ch == 'C'
                        || next_ch == 'D'
                    {
                        break;
                    }
                    params.push(next_ch);
                    chars.next();
                }

                if let Some(&terminator) = chars.peek() {
                    chars.next(); // consume terminator

                    if terminator == 'm' {
                        // SGR - Select Graphic Rendition
                        apply_sgr(&params, &mut current_color, &mut current_bold);
                    }
                    // Other CSI sequences (K, H, J, etc.) are silently ignored
                }
            } else {
                // ESC not followed by '[' - skip the ESC
                continue;
            }
        } else {
            current_text.push(ch);
        }
    }

    // Flush remaining text
    if !current_text.is_empty() {
        segments.push(AnsiSegment {
            text: current_text,
            color: current_color,
            bold: current_bold,
        });
    }

    segments
}

/// Apply SGR (Select Graphic Rendition) parameters
fn apply_sgr(params: &str, color: &mut egui::Color32, bold: &mut bool) {
    if params.is_empty() || params == "0" {
        // Reset
        *color = egui::Color32::from_rgb(204, 204, 204);
        *bold = false;
        return;
    }

    let codes: Vec<u32> = params
        .split(';')
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    let mut i = 0;
    while i < codes.len() {
        let code = codes[i];

        match code {
            0 => {
                *color = egui::Color32::from_rgb(204, 204, 204);
                *bold = false;
            }
            1 => *bold = true,
            22 => *bold = false,

            // Standard foreground colors (30-37)
            30 => *color = egui::Color32::from_rgb(40, 40, 40),    // black
            31 => *color = egui::Color32::from_rgb(220, 80, 80),   // red
            32 => *color = egui::Color32::from_rgb(80, 200, 80),   // green
            33 => *color = egui::Color32::from_rgb(220, 180, 60),  // yellow
            34 => *color = egui::Color32::from_rgb(80, 140, 220),  // blue
            35 => *color = egui::Color32::from_rgb(180, 80, 200),  // magenta
            36 => *color = egui::Color32::from_rgb(80, 200, 200),  // cyan
            37 => *color = egui::Color32::from_rgb(204, 204, 204), // white

            // Bright foreground colors (90-97)
            90 => *color = egui::Color32::from_rgb(100, 100, 100), // bright black (gray)
            91 => *color = egui::Color32::from_rgb(255, 120, 120), // bright red
            92 => *color = egui::Color32::from_rgb(120, 255, 120), // bright green
            93 => *color = egui::Color32::from_rgb(255, 255, 100), // bright yellow
            94 => *color = egui::Color32::from_rgb(100, 180, 255), // bright blue
            95 => *color = egui::Color32::from_rgb(255, 120, 255), // bright magenta
            96 => *color = egui::Color32::from_rgb(120, 255, 255), // bright cyan
            97 => *color = egui::Color32::from_rgb(255, 255, 255), // bright white

            // 256-color mode: 38;5;N
            38 => {
                if i + 2 < codes.len() && codes[i + 1] == 5 {
                    let n = codes[i + 2];
                    *color = ansi_256_color(n);
                    i += 2; // skip the '5' and 'N'
                }
            }

            // Default foreground
            39 => *color = egui::Color32::from_rgb(204, 204, 204),

            // Background colors (40-47, 100-107) - ignored for now (we use dark bg)
            40..=49 | 100..=107 => {}

            _ => {}
        }

        i += 1;
    }
}

/// Convert a 256-color index to an egui Color32
fn ansi_256_color(n: u32) -> egui::Color32 {
    match n {
        // Standard colors 0-7
        0 => egui::Color32::from_rgb(40, 40, 40),
        1 => egui::Color32::from_rgb(220, 80, 80),
        2 => egui::Color32::from_rgb(80, 200, 80),
        3 => egui::Color32::from_rgb(220, 180, 60),
        4 => egui::Color32::from_rgb(80, 140, 220),
        5 => egui::Color32::from_rgb(180, 80, 200),
        6 => egui::Color32::from_rgb(80, 200, 200),
        7 => egui::Color32::from_rgb(204, 204, 204),
        // Bright colors 8-15
        8 => egui::Color32::from_rgb(100, 100, 100),
        9 => egui::Color32::from_rgb(255, 120, 120),
        10 => egui::Color32::from_rgb(120, 255, 120),
        11 => egui::Color32::from_rgb(255, 255, 100),
        12 => egui::Color32::from_rgb(100, 180, 255),
        13 => egui::Color32::from_rgb(255, 120, 255),
        14 => egui::Color32::from_rgb(120, 255, 255),
        15 => egui::Color32::from_rgb(255, 255, 255),
        // 216-color cube (16-231)
        16..=231 => {
            let idx = n - 16;
            let r_idx = idx / 36;
            let g_idx = (idx % 36) / 6;
            let b_idx = idx % 6;
            let r = if r_idx == 0 { 0 } else { (r_idx * 40 + 55) as u8 };
            let g = if g_idx == 0 { 0 } else { (g_idx * 40 + 55) as u8 };
            let b = if b_idx == 0 { 0 } else { (b_idx * 40 + 55) as u8 };
            egui::Color32::from_rgb(r, g, b)
        }
        // Grayscale (232-255)
        232..=255 => {
            let gray = ((n - 232) * 10 + 8) as u8;
            egui::Color32::from_rgb(gray, gray, gray)
        }
        _ => egui::Color32::from_rgb(204, 204, 204),
    }
}

/// Render ANSI-colored text segments using egui
pub fn render_ansi_text(
    ui: &mut egui::Ui,
    text: &str,
    default_color: egui::Color32,
    font_size: f32,
) {
    // Fast path: if no escape sequences, render plain
    if !text.contains('\x1b') {
        ui.label(
            egui::RichText::new(text)
                .color(default_color)
                .font(egui::FontId::monospace(font_size)),
        );
        return;
    }

    let segments = parse_ansi(text);

    if segments.is_empty() {
        return;
    }

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for segment in &segments {
            if segment.text.is_empty() {
                continue;
            }
            let mut rt = egui::RichText::new(&segment.text)
                .color(segment.color)
                .font(egui::FontId::monospace(font_size));
            if segment.bold {
                rt = rt.strong();
            }
            ui.label(rt);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let segments = parse_ansi("hello world");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "hello world");
    }

    #[test]
    fn test_colored_text() {
        let segments = parse_ansi("\x1b[31mred text\x1b[0m normal");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "red text");
        assert_eq!(segments[0].color, egui::Color32::from_rgb(220, 80, 80));
        assert_eq!(segments[1].text, " normal");
    }

    #[test]
    fn test_bold() {
        let segments = parse_ansi("\x1b[1;32mbold green\x1b[0m");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "bold green");
        assert!(segments[0].bold);
        assert_eq!(segments[0].color, egui::Color32::from_rgb(80, 200, 80));
    }

    #[test]
    fn test_reset() {
        let segments = parse_ansi("\x1b[31mred\x1b[0mnormal");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "red");
        assert_eq!(segments[1].color, egui::Color32::from_rgb(204, 204, 204));
    }

    #[test]
    fn test_strip_escape_no_m() {
        // Test CSI sequences that aren't SGR (like erase line)
        let segments = parse_ansi("before\x1b[2Kafter");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "before");
        assert_eq!(segments[1].text, "after");
    }

    #[test]
    fn test_utf8_text() {
        let segments = parse_ansi("\x1b[32m日本語テスト\x1b[0m");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "日本語テスト");
        assert_eq!(segments[0].color, egui::Color32::from_rgb(80, 200, 80));
    }

    #[test]
    fn test_256_color() {
        let segments = parse_ansi("\x1b[38;5;9mcolor\x1b[0m");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "color");
        assert_eq!(segments[0].color, egui::Color32::from_rgb(255, 120, 120));
    }

    #[test]
    fn test_multiple_segments() {
        let segments = parse_ansi("\x1b[31mred\x1b[32mgreen\x1b[34mblue\x1b[0m");
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].text, "red");
        assert_eq!(segments[1].text, "green");
        assert_eq!(segments[2].text, "blue");
    }
}
