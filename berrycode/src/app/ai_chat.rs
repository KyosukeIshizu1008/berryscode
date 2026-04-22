//! AI Chat panel rendering and gRPC communication

use super::types::{AIChatMode, GrpcMessage, GrpcResponse};
use super::utils::strip_thinking_blocks;
use super::BerryCodeApp;
use crate::app::i18n::t;

impl BerryCodeApp {
    /// Render AI Chat panel (right side of editor)
    #[allow(dead_code)]
    pub(crate) fn render_ai_chat_panel(&mut self, ctx: &egui::Context) {
        // ── Drag-and-drop image detection ─────────────────────────────
        let dropped: Vec<_> = ctx.input(|i| i.raw.dropped_files.clone());
        for file in &dropped {
            if let Some(path) = &file.path {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ["png", "jpg", "jpeg", "gif", "webp", "bmp"].contains(&ext.as_str()) {
                    self.chat_attachment = Some(path.to_string_lossy().to_string());
                }
            }
        }

        // Accent colors for the chat panel
        const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(25, 26, 28); // match editor bg #191A1C
        const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(25, 26, 28);
        const INPUT_BG: egui::Color32 = egui::Color32::from_rgb(28, 29, 34);
        const USER_BG: egui::Color32 = egui::Color32::from_rgb(45, 55, 95);
        const ACCENT: egui::Color32 = egui::Color32::from_rgb(99, 139, 255);
        const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(110, 115, 130);
        const DIVIDER: egui::Color32 = egui::Color32::from_rgb(35, 37, 45);

        egui::SidePanel::right("ai_chat_panel")
            .default_width(420.0)
            .width_range(200.0..=600.0)
            .resizable(true)
            .show_separator_line(true)
            .frame(egui::Frame::none().fill(PANEL_BG).inner_margin(0.0))
            .show(ctx, |ui| {
                // ── Header (VS Code Copilot style) ────────────────────
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new("Copilot")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(200, 205, 220)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("+")
                                        .size(14.0)
                                        .color(egui::Color32::from_rgb(180, 180, 180)),
                                )
                                .frame(false),
                            )
                            .on_hover_text("New Chat")
                            .clicked()
                        {
                            self.grpc_messages.clear();
                            self.grpc_input.clear();
                        }
                    });
                });
                ui.add_space(2.0);

                // ── Layout: input pinned to bottom, scroll fills rest ──
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // ── Input area ────────────────────────────────────
                    egui::Frame::none()
                        .fill(PANEL_BG)
                        .inner_margin(egui::Margin {
                            left: 12.0,
                            right: 12.0,
                            top: 8.0,
                            bottom: 12.0,
                        })
                        .show(ui, |ui| {
                            let input_id = egui::Id::new("chat_input");
                            let input_focused = ui.memory(|m| m.has_focus(input_id));
                            let border_color = if input_focused {
                                ACCENT
                            } else {
                                egui::Color32::from_rgb(48, 50, 62)
                            };

                            egui::Frame::none()
                                .fill(INPUT_BG)
                                .inner_margin(egui::Margin {
                                    left: 14.0,
                                    right: 10.0,
                                    top: 10.0,
                                    bottom: 8.0,
                                })
                                .rounding(12.0)
                                .stroke(egui::Stroke::new(1.5, border_color))
                                .show(ui, |ui| {
                                    // Attachment preview
                                    if let Some(ref path) = self.chat_attachment.clone() {
                                        let fname = std::path::Path::new(path)
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(path);
                                        egui::Frame::none()
                                            .fill(egui::Color32::from_rgb(30, 35, 50))
                                            .rounding(6.0)
                                            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(fname)
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(
                                                                160, 180, 255,
                                                            )),
                                                    );
                                                    if ui
                                                        .small_button(
                                                            egui::RichText::new("x")
                                                                .size(10.0)
                                                                .color(TEXT_DIM),
                                                        )
                                                        .clicked()
                                                    {
                                                        self.chat_attachment = None;
                                                    }
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }

                                    let hint = if self.chat_attachment.is_some() {
                                        t(self.ui_language, "Ask about image...")
                                    } else {
                                        t(self.ui_language, "Ask anything...")
                                    };
                                    let text_edit = egui::TextEdit::multiline(&mut self.grpc_input)
                                        .id(input_id)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(2)
                                        .hint_text(egui::RichText::new(hint).color(TEXT_DIM))
                                        .font(egui::FontId::proportional(14.0))
                                        .frame(false);
                                    let response = ui.add(text_edit);

                                    ui.add_space(4.0);

                                    // Send button row
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if self.grpc_streaming {
                                                    ui.spinner();
                                                } else {
                                                    let send_enabled =
                                                        !self.grpc_input.trim().is_empty()
                                                            || self.chat_attachment.is_some();
                                                    let send_btn = egui::Button::new(
                                                        egui::RichText::new("↑").size(16.0).color(
                                                            if send_enabled {
                                                                egui::Color32::WHITE
                                                            } else {
                                                                TEXT_DIM
                                                            },
                                                        ),
                                                    )
                                                    .fill(if send_enabled {
                                                        ACCENT
                                                    } else {
                                                        egui::Color32::from_rgb(40, 42, 52)
                                                    })
                                                    .rounding(8.0)
                                                    .min_size(egui::vec2(28.0, 28.0));

                                                    if ui
                                                        .add_enabled(send_enabled, send_btn)
                                                        .clicked()
                                                        || (response.has_focus()
                                                            && ui.input(|i| {
                                                                i.modifiers.command
                                                                    && i.key_pressed(
                                                                        egui::Key::Enter,
                                                                    )
                                                            }))
                                                    {
                                                        // Prepend image path to message if attached
                                                        if let Some(ref img) =
                                                            self.chat_attachment.clone()
                                                        {
                                                            if self.grpc_input.is_empty() {
                                                                self.grpc_input =
                                                                    format!("[image:{}]", img);
                                                            } else {
                                                                self.grpc_input = format!(
                                                                    "[image:{}] {}",
                                                                    img, self.grpc_input
                                                                );
                                                            }
                                                            self.chat_attachment = None;
                                                        }
                                                        self.send_grpc_message();
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                        });

                    // ── Message scroll area (fills remaining height) ──
                    egui::ScrollArea::vertical()
                        .id_salt("chat_messages_scroll")
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Force top-down layout inside the scroll area.
                            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                ui.set_min_width(ui.available_width());

                                if self.grpc_messages.is_empty() && !self.grpc_streaming {
                                    // ── Welcome / empty state (VS Code Copilot style) ──
                                    ui.add_space(40.0);
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(20.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Ask anything or type / for commands",
                                            )
                                            .size(13.0)
                                            .color(egui::Color32::from_rgb(130, 135, 150)),
                                        );
                                        ui.add_space(24.0);

                                        // Simple suggestion buttons (no category tags)
                                        let suggestions = vec![
                                            t(self.ui_language, "Explain the design"),
                                            t(self.ui_language, "Fix compile errors"),
                                            t(self.ui_language, "Commit changes"),
                                            t(self.ui_language, "Security check"),
                                        ];
                                        for text in &suggestions {
                                            let btn = egui::Button::new(
                                                egui::RichText::new(*text)
                                                    .size(12.0)
                                                    .color(egui::Color32::from_rgb(180, 185, 200)),
                                            )
                                            .fill(egui::Color32::from_rgb(35, 37, 42))
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_rgb(55, 57, 63),
                                            ))
                                            .rounding(6.0)
                                            .min_size(egui::vec2(200.0, 28.0));
                                            if ui.add(btn).clicked() {
                                                self.grpc_input = text.to_string();
                                                self.send_grpc_message();
                                            }
                                            ui.add_space(4.0);
                                        }
                                    });
                                } else {
                                    ui.add_space(16.0);
                                    let messages: Vec<(String, bool)> = self
                                        .grpc_messages
                                        .iter()
                                        .map(|m| (m.content.clone(), m.is_user))
                                        .collect();

                                    for (content, is_user) in &messages {
                                        if *is_user {
                                            let avail = ui.available_width();
                                            ui.horizontal(|ui| {
                                                let bubble_max = 300.0_f32;
                                                let right_pad = 12.0_f32;
                                                let spacer =
                                                    (avail - bubble_max - right_pad - 28.0)
                                                        .max(0.0);
                                                ui.add_space(spacer);
                                                egui::Frame::none()
                                                    .fill(USER_BG)
                                                    .inner_margin(egui::Margin {
                                                        left: 14.0,
                                                        right: 14.0,
                                                        top: 10.0,
                                                        bottom: 10.0,
                                                    })
                                                    .rounding(egui::Rounding {
                                                        nw: 16.0,
                                                        ne: 4.0,
                                                        sw: 16.0,
                                                        se: 16.0,
                                                    })
                                                    .show(ui, |ui| {
                                                        ui.set_max_width(bubble_max);
                                                        ui.label(
                                                            egui::RichText::new(content)
                                                                .color(egui::Color32::from_rgb(
                                                                    225, 230, 255,
                                                                ))
                                                                .size(14.0),
                                                        );
                                                    });
                                                ui.add_space(right_pad);
                                            });
                                        } else {
                                            ui.horizontal(|ui| {
                                                ui.add_space(12.0);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("berrycode")
                                                            .size(10.0)
                                                            .color(TEXT_DIM),
                                                    );
                                                    ui.add_space(2.0);
                                                    ui.set_max_width(380.0);
                                                    Self::render_markdown(ui, content);
                                                });
                                            });
                                        }
                                        ui.add_space(18.0);
                                    }
                                }

                                // Streaming response
                                if self.grpc_streaming {
                                    ui.horizontal(|ui| {
                                        ui.add_space(12.0);
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new("berrycode")
                                                    .size(10.0)
                                                    .color(TEXT_DIM),
                                            );
                                            ui.add_space(2.0);
                                            ui.set_max_width(380.0);
                                            let visible =
                                                strip_thinking_blocks(&self.grpc_current_response);
                                            if !visible.is_empty() {
                                                Self::render_markdown(ui, &visible);
                                            }
                                            ui.add_space(6.0);
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(
                                                    egui::RichText::new(" thinking…")
                                                        .size(11.0)
                                                        .color(TEXT_DIM),
                                                );
                                            });
                                        });
                                    });
                                    ui.add_space(18.0);
                                }

                                ui.add_space(8.0);
                            }); // top_down layout
                        });
                });
            });
    }

    #[allow(dead_code)]
    pub(crate) fn render_berrycode_ai_chat(&mut self, ui: &mut egui::Ui) {
        ui.label("AI Chat - Use right panel instead.");
    }

    /// Simple markdown renderer for AI chat responses
    pub(crate) fn render_markdown(ui: &mut egui::Ui, content: &str) {
        let mut in_code_block = false;
        let mut _code_lang = String::new();
        let mut code_lines = Vec::new();

        for line in content.lines() {
            // Code block detection
            if line.trim().starts_with("```") {
                if in_code_block {
                    // End code block - render it
                    let code_text = code_lines.join("\n");
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 35, 35))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&code_text)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                                )
                                .selectable(true),
                            );
                        });
                    code_lines.clear();
                    in_code_block = false;
                } else {
                    // Start code block
                    _code_lang = line.trim().strip_prefix("```").unwrap_or("").to_string();
                    in_code_block = true;
                }
                continue;
            }

            if in_code_block {
                code_lines.push(line);
                continue;
            }

            // Heading detection
            if line.trim().starts_with("# ") {
                ui.heading(
                    egui::RichText::new(line.trim_start_matches("# "))
                        .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                );
                continue;
            }
            if line.trim().starts_with("## ") {
                ui.label(
                    egui::RichText::new(line.trim_start_matches("## "))
                        .size(16.0)
                        .strong()
                        .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                );
                continue;
            }
            if line.trim().starts_with("### ") {
                ui.label(
                    egui::RichText::new(line.trim_start_matches("### "))
                        .size(14.0)
                        .strong()
                        .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                );
                continue;
            }

            // List detection (bullets)
            if line.trim().starts_with("- ") || line.trim().starts_with("* ") {
                ui.horizontal(|ui| {
                    ui.label("•");
                    let text = line.trim_start_matches("- ").trim_start_matches("* ");
                    Self::render_inline_formatting(ui, text);
                });
                continue;
            }

            // List detection (numbered)
            if let Some(rest) = line.trim().strip_prefix(|c: char| c.is_ascii_digit()) {
                if rest.starts_with(". ") {
                    let number = line
                        .trim()
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect::<String>();
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", number));
                        let text = rest.trim_start_matches(". ");
                        Self::render_inline_formatting(ui, text);
                    });
                    continue;
                }
            }

            // Regular text - handle inline formatting
            if !line.trim().is_empty() {
                Self::render_inline_formatting(ui, line);
            } else {
                ui.add_space(4.0);
            }
        }

        // Handle unclosed code block
        if in_code_block && !code_lines.is_empty() {
            let code_text = code_lines.join("\n");
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(35, 35, 35))
                .inner_margin(8.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&code_text)
                                .monospace()
                                .color(egui::Color32::from_rgb(0xAB, 0xB2, 0xBF)),
                        )
                        .selectable(true),
                    );
                });
        }
    }

    /// Render inline markdown formatting (bold, italic, code, links)
    pub(crate) fn render_inline_formatting(ui: &mut egui::Ui, text: &str) {
        let unified_white = egui::Color32::from_rgb(0xAB, 0xB2, 0xBF);
        let code_bg = egui::Color32::from_rgb(45, 45, 45);

        // Parse inline markdown into segments
        #[derive(Debug)]
        enum Segment {
            Text(String),
            Code(String),
            Bold(String),
            Italic(String),
            Link { text: String, url: String },
        }

        let mut segments = Vec::new();
        let mut chars = text.chars().peekable();
        let mut current_text = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut code_text = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '`' {
                            chars.next();
                            break;
                        }
                        code_text.push(chars.next().unwrap());
                    }
                    segments.push(Segment::Code(code_text));
                }
                '*' if chars.peek() == Some(&'*') => {
                    chars.next();
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut bold_text = String::new();
                    let mut found_closing = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '*' {
                            chars.next();
                            if chars.peek() == Some(&'*') {
                                chars.next();
                                found_closing = true;
                                break;
                            } else {
                                bold_text.push('*');
                            }
                        } else {
                            bold_text.push(chars.next().unwrap());
                        }
                    }
                    if found_closing {
                        segments.push(Segment::Bold(bold_text));
                    } else {
                        current_text.push_str("**");
                        current_text.push_str(&bold_text);
                    }
                }
                '*' => {
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut italic_text = String::new();
                    let mut found_closing = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '*' {
                            chars.next();
                            found_closing = true;
                            break;
                        }
                        italic_text.push(chars.next().unwrap());
                    }
                    if found_closing {
                        segments.push(Segment::Italic(italic_text));
                    } else {
                        current_text.push('*');
                        current_text.push_str(&italic_text);
                    }
                }
                '[' => {
                    if !current_text.is_empty() {
                        segments.push(Segment::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let mut link_text = String::new();
                    let mut found_text_end = false;
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == ']' {
                            chars.next();
                            found_text_end = true;
                            break;
                        }
                        link_text.push(chars.next().unwrap());
                    }
                    if found_text_end && chars.peek() == Some(&'(') {
                        chars.next();
                        let mut url = String::new();
                        let mut found_url_end = false;
                        while let Some(&next_ch) = chars.peek() {
                            if next_ch == ')' {
                                chars.next();
                                found_url_end = true;
                                break;
                            }
                            url.push(chars.next().unwrap());
                        }
                        if found_url_end {
                            segments.push(Segment::Link {
                                text: link_text,
                                url,
                            });
                        } else {
                            current_text.push('[');
                            current_text.push_str(&link_text);
                            current_text.push_str("](");
                            current_text.push_str(&url);
                        }
                    } else {
                        current_text.push('[');
                        current_text.push_str(&link_text);
                        if found_text_end {
                            current_text.push(']');
                        }
                    }
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }

        if !current_text.is_empty() {
            segments.push(Segment::Text(current_text));
        }

        // Render segments with word wrapping enabled
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            for segment in segments {
                match segment {
                    Segment::Text(s) => {
                        ui.label(egui::RichText::new(s).color(unified_white));
                    }
                    Segment::Code(s) => {
                        ui.label(
                            egui::RichText::new(s)
                                .monospace()
                                .color(unified_white)
                                .background_color(code_bg),
                        );
                    }
                    Segment::Bold(s) => {
                        ui.label(egui::RichText::new(s).strong().color(unified_white));
                    }
                    Segment::Italic(s) => {
                        ui.label(egui::RichText::new(s).italics().color(unified_white));
                    }
                    Segment::Link { text, url } => {
                        ui.hyperlink_to(text, url);
                    }
                }
            }
        });
    }

    /// Send a message to the AI via gRPC
    pub(crate) fn send_grpc_message(&mut self) {
        let message = self.grpc_input.trim().to_string();
        if message.is_empty() {
            return;
        }

        // Add user message to chat history
        self.grpc_messages.push(GrpcMessage {
            content: message.clone(),
            is_user: true,
        });

        // Clear input
        self.grpc_input.clear();

        // Set streaming state
        self.grpc_streaming = true;
        self.grpc_current_response.clear();
        self.grpc_streaming_message = Some(String::new());

        let tx = self.grpc_response_tx.clone();
        let repo_path = self.root_path.clone();

        // Try gRPC first, fallback to REST (berry-core-api)
        if let Some(session_id) = &self.grpc_session_id {
            // Use gRPC (legacy berry-api-server)
            let grpc_client = self.grpc_client.clone();
            let session_id = session_id.clone();
            let autonomous = self.ai_chat_mode == AIChatMode::Autonomous;

            tracing::info!("📤 Sending via gRPC: {}", message);

            self.lsp_runtime.spawn(async move {
                match grpc_client
                    .chat_stream(session_id, message, autonomous)
                    .await
                {
                    Ok(mut rx) => {
                        while let Some(chunk) = rx.recv().await {
                            if let Some(tx) = &tx {
                                let _ = tx.send(GrpcResponse::ChatChunk(chunk));
                            }
                        }
                        if let Some(tx) = &tx {
                            let _ = tx.send(GrpcResponse::ChatStreamCompleted);
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ gRPC chat failed: {}", e);
                    }
                }
            });
        } else {
            // Use REST (berry-core-api)
            tracing::info!("📤 Sending via REST (berry-core-api): {}", message);

            let rest_client = crate::native::rest_client::get_client().clone();

            self.lsp_runtime.spawn(async move {
                match rest_client.chat(&repo_path, &message, None).await {
                    Ok(response) => {
                        if let Some(tx) = &tx {
                            let _ = tx.send(GrpcResponse::ChatChunk(response));
                            let _ = tx.send(GrpcResponse::ChatStreamCompleted);
                        }
                    }
                    Err(e) => {
                        tracing::error!("❌ REST chat failed: {}", e);
                        if let Some(tx) = &tx {
                            let _ = tx.send(GrpcResponse::ChatChunk(format!(
                                "⚠️ AI Chat error: {}.\n\nMake sure berry-core-api is running:\n```\ncd ../berry-core-api && cargo run\n```",
                                e
                            )));
                            let _ = tx.send(GrpcResponse::ChatStreamCompleted);
                        }
                    }
                }
            });
        }
    }

    pub(crate) fn poll_grpc_responses(&mut self) {
        if let Some(rx) = &mut self.grpc_response_rx {
            while let Ok(response) = rx.try_recv() {
                match response {
                    GrpcResponse::SessionStarted(session_id) => {
                        tracing::info!("🎯 gRPC session ready: {}", session_id);
                        self.grpc_session_id = Some(session_id);
                        self.grpc_connected = true;
                        self.status_message = "✅ AI Chat ready".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    GrpcResponse::ChatChunk(chunk) => {
                        tracing::info!("🎨 UI received chunk: {} chars", chunk.len());

                        self.grpc_current_response.push_str(&chunk);

                        if let Some(streaming_msg) = &mut self.grpc_streaming_message {
                            streaming_msg.push_str(&chunk);
                            tracing::info!(
                                "📝 Accumulated message: {} chars total",
                                streaming_msg.len()
                            );
                        } else {
                            self.grpc_streaming_message = Some(String::new());
                            if let Some(streaming_msg) = &mut self.grpc_streaming_message {
                                streaming_msg.push_str(&chunk);
                            }
                        }
                    }
                    GrpcResponse::ChatStreamCompleted => {
                        tracing::info!("✅ Chat stream completed");

                        if !self.grpc_current_response.is_empty() {
                            let stripped = strip_thinking_blocks(&self.grpc_current_response);
                            let content = if stripped.is_empty() {
                                self.grpc_current_response.trim().to_string()
                            } else {
                                stripped
                            };
                            self.grpc_messages.push(GrpcMessage {
                                content,
                                is_user: false,
                            });
                            self.grpc_current_response.clear();
                        }

                        self.grpc_streaming = false;
                        self.grpc_streaming_message = None;
                    }
                }
            }
        }
    }
}
