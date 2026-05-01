//! AI Chat panel rendering and REST communication

use super::types::{AiChatMessage, AiChatResponse};
use super::utils::strip_thinking_blocks;
use super::BerryCodeApp;

impl BerryCodeApp {
    /// Render AI Chat panel (right side of editor)
    #[allow(dead_code)]
    pub(crate) fn render_ai_chat_panel(&mut self, ctx: &egui::Context) {
        // Accent colors for the chat panel
        const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(25, 26, 28); // match editor bg #191A1C
        const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(25, 26, 28);
        const INPUT_BG: egui::Color32 = egui::Color32::from_rgb(28, 29, 34);
        const USER_BG: egui::Color32 = egui::Color32::from_rgb(45, 55, 95);
        const ACCENT: egui::Color32 = egui::Color32::from_rgb(99, 139, 255);
        const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(110, 115, 130);
        const DIVIDER: egui::Color32 = egui::Color32::from_rgb(35, 37, 45);

        // ── Collapsed state: render a thin 32 px reopen strip ──
        if self.ai_chat_collapsed {
            egui::SidePanel::right("ai_chat_panel_v2_collapsed")
                .exact_width(32.0)
                .resizable(false)
                .show_separator_line(true)
                .frame(egui::Frame::NONE.fill(PANEL_BG).inner_margin(0))
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("\u{eab5}") // codicon chevron-left
                                        .family(egui::FontFamily::Name("codicon".into()))
                                        .size(16.0)
                                        .color(egui::Color32::from_rgb(180, 180, 180)),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Expand AI Chat")
                            .clicked()
                        {
                            self.ai_chat_collapsed = false;
                        }
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("AI")
                                .size(10.0)
                                .color(egui::Color32::from_rgb(140, 140, 150)),
                        );
                    });
                });
            return;
        }

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

        egui::SidePanel::right("ai_chat_panel_v2")
            .default_width(420.0)
            .width_range(200.0..=600.0)
            .resizable(true)
            .show_separator_line(true)
            .frame(egui::Frame::NONE.fill(PANEL_BG).inner_margin(0))
            .show(ctx, |ui| {
                // ── Header ────────────────────
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    // Collapse toggle (chevron pointing right = collapse)
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("\u{eab6}") // codicon chevron-right
                                    .family(egui::FontFamily::Name("codicon".into()))
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Collapse")
                        .clicked()
                    {
                        self.ai_chat_collapsed = true;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Berry AI")
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
                            self.ai_messages.clear();
                            self.ai_input.clear();
                        }
                    });
                });
                ui.add_space(2.0);

                // ── Layout: input pinned to bottom, scroll fills rest ──
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    // ── Pending agent edits (Phase D) ─────────────────
                    // Sit just above the input so the user can decide
                    // whether to apply each proposal before the next
                    // turn. Drained as the user clicks Approve / Reject.
                    self.render_pending_agent_edits(ui);

                    // ── Input area ────────────────────────────────────
                    egui::Frame::NONE
                        .fill(PANEL_BG)
                        .inner_margin(egui::Margin {
                            left: 12,
                            right: 12,
                            top: 8,
                            bottom: 12,
                        })
                        .show(ui, |ui| {
                            // ── Agent backend picker (above input)
                            // Chat / Agent toggle was retired with the
                            // shift to a single Native-agent flow — all
                            // requests now go through the agent loop,
                            // which makes the model's `read_file` /
                            // tool calls transparent in the chat panel.
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let claude_installed = crate::agent::CodingAgent::check_installed(
                                    &crate::agent::claude::ClaudeCodeAgent::new(),
                                );
                                let codex_installed = crate::agent::CodingAgent::check_installed(
                                    &crate::agent::codex::CodexAgent::new(),
                                );
                                let mut backend = self.ai_settings.agent_backend.clone();
                                ui.label(egui::RichText::new("Agent:").size(11.0).color(TEXT_DIM));
                                egui::ComboBox::from_id_salt("agent_backend_picker")
                                    .selected_text(match backend.as_str() {
                                        "codex" => "Codex",
                                        "claude" => "Claude Code",
                                        _ => "Native",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut backend,
                                            "native".to_string(),
                                            "Native (in-process)",
                                        );
                                        ui.selectable_value(
                                            &mut backend,
                                            "claude".to_string(),
                                            "Claude Code",
                                        );
                                        ui.selectable_value(
                                            &mut backend,
                                            "codex".to_string(),
                                            "Codex",
                                        );
                                    });
                                if backend != self.ai_settings.agent_backend {
                                    self.ai_settings.agent_backend = backend.clone();
                                    self.ai_settings.save();
                                }
                                let (installed, install_hint) = match backend.as_str() {
                                    "native" => (Some("ready".to_string()), ""),
                                    "codex" => (
                                        codex_installed,
                                        "codex not on PATH — `npm i -g @openai/codex`",
                                    ),
                                    _ => (
                                        claude_installed,
                                        "claude not on PATH — `npm i -g @anthropic-ai/claude-code`",
                                    ),
                                };
                                match installed {
                                    Some(v) => {
                                        ui.label(egui::RichText::new(v).size(11.0).color(TEXT_DIM));
                                    }
                                    None => {
                                        ui.label(
                                            egui::RichText::new(install_hint)
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(220, 120, 120)),
                                        );
                                    }
                                }
                            });
                            ui.add_space(6.0);

                            let input_id = egui::Id::new("chat_input");
                            let input_focused = ui.memory(|m| m.has_focus(input_id));
                            let border_color = if input_focused {
                                ACCENT
                            } else {
                                egui::Color32::from_rgb(48, 50, 62)
                            };

                            egui::Frame::NONE
                                .fill(INPUT_BG)
                                .inner_margin(egui::Margin {
                                    left: 14,
                                    right: 10,
                                    top: 10,
                                    bottom: 8,
                                })
                                .corner_radius(12)
                                .stroke(egui::Stroke::new(1.5, border_color))
                                .show(ui, |ui| {
                                    // Attachment preview
                                    if let Some(ref path) = self.chat_attachment.clone() {
                                        let fname = std::path::Path::new(path)
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(path);
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgb(30, 35, 50))
                                            .corner_radius(6)
                                            .inner_margin(egui::Margin::symmetric(8, 4))
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
                                        self.tr("Ask about image...")
                                    } else {
                                        self.tr("Ask anything...")
                                    };
                                    let text_edit = egui::TextEdit::multiline(&mut self.ai_input)
                                        .id(input_id)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(2)
                                        .hint_text(egui::RichText::new(hint).color(TEXT_DIM))
                                        .font(egui::FontId::proportional(14.0))
                                        .frame(false);
                                    let response = ui.add(text_edit);

                                    // Cmd+L from anywhere drops focus into
                                    // the AI chat input. The flag is raised
                                    // by the global shortcut handler in
                                    // `mod.rs`; we consume it here so the
                                    // very next frame has the cursor in the
                                    // ai_input box even if another widget
                                    // currently owns focus.
                                    if self.ai_chat_focus_pending {
                                        self.ai_chat_focus_pending = false;
                                        response.request_focus();
                                    }

                                    ui.add_space(2.0);

                                    // Send button row
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if self.ai_streaming {
                                                    ui.spinner();
                                                } else {
                                                    let send_enabled =
                                                        !self.ai_input.trim().is_empty()
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
                                                    .corner_radius(8)
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
                                                            if self.ai_input.is_empty() {
                                                                self.ai_input =
                                                                    format!("[image:{}]", img);
                                                            } else {
                                                                self.ai_input = format!(
                                                                    "[image:{}] {}",
                                                                    img, self.ai_input
                                                                );
                                                            }
                                                            self.chat_attachment = None;
                                                        }
                                                        self.send_ai_message();
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

                                if self.ai_messages.is_empty() && !self.ai_streaming {
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
                                            self.tr("Explain the design"),
                                            self.tr("Fix compile errors"),
                                            self.tr("Commit changes"),
                                            self.tr("Security check"),
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
                                            .corner_radius(6)
                                            .min_size(egui::vec2(200.0, 28.0));
                                            if ui.add(btn).clicked() {
                                                self.ai_input = text.to_string();
                                                self.send_ai_message();
                                            }
                                            ui.add_space(4.0);
                                        }
                                    });
                                } else {
                                    ui.add_space(16.0);
                                    // Iterate by reference; each message
                                    // can be 1-10 KB and cloning the
                                    // whole list every frame (60+ fps)
                                    // showed up as visible stutter once
                                    // a few long agent responses had
                                    // landed.
                                    for msg in &self.ai_messages {
                                        let content = &msg.content;
                                        let is_user = msg.is_user;
                                        if is_user {
                                            let avail = ui.available_width();
                                            ui.horizontal(|ui| {
                                                let bubble_max = 300.0_f32;
                                                let right_pad = 12.0_f32;
                                                let spacer =
                                                    (avail - bubble_max - right_pad - 28.0)
                                                        .max(0.0);
                                                ui.add_space(spacer);
                                                egui::Frame::NONE
                                                    .fill(USER_BG)
                                                    .inner_margin(egui::Margin {
                                                        left: 14,
                                                        right: 14,
                                                        top: 10,
                                                        bottom: 10,
                                                    })
                                                    .corner_radius(egui::CornerRadius {
                                                        nw: 16,
                                                        ne: 4,
                                                        sw: 16,
                                                        se: 16,
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
                                                    Self::render_message_body(ui, content);
                                                });
                                            });
                                        }
                                        ui.add_space(18.0);
                                    }
                                }

                                // Streaming response
                                if self.ai_streaming {
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
                                                strip_thinking_blocks(&self.ai_current_response);
                                            if !visible.is_empty() {
                                                Self::render_message_body(ui, &visible);
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

    /// Render a chat message body that may interleave assistant text
    /// with native-agent tool calls. Tool calls arrive as
    /// `<<BERRY_TOOL>>name|args<<END>>` sentinels (see
    /// [`Self::send_agent_message`]); we split on those, draw a styled
    /// pill in their place, and run the surrounding text through the
    /// normal markdown renderer so code fences / links / bold still
    /// work.
    pub(crate) fn render_message_body(ui: &mut egui::Ui, content: &str) {
        const OPEN: &str = "<<BERRY_TOOL>>";
        const CLOSE: &str = "<<END>>";
        let mut rest = content;
        // Group consecutive same-tool calls into a single pill — the
        // model often fans out 5+ `list_files` / `read_file` calls in
        // a row, and stacking those vertically swamps the panel. Args
        // are kept as a list so the pill can show all paths inline,
        // separated by `·`.
        let mut pending: Option<(String, Vec<String>)> = None;
        let flush = |ui: &mut egui::Ui, pending: &mut Option<(String, Vec<String>)>| {
            if let Some((tool, args)) = pending.take() {
                Self::render_tool_pill(ui, &tool, &args);
            }
        };
        while let Some(start) = rest.find(OPEN) {
            let head = &rest[..start];
            if !head.trim().is_empty() {
                flush(ui, &mut pending);
                Self::render_markdown(ui, head);
            }
            let after_open = &rest[start + OPEN.len()..];
            let Some(end) = after_open.find(CLOSE) else {
                flush(ui, &mut pending);
                Self::render_markdown(ui, &rest[start..]);
                return;
            };
            let payload = &after_open[..end];
            let (tool, args) = payload.split_once('|').unwrap_or((payload, ""));
            match pending.as_mut() {
                Some((prev_tool, list)) if prev_tool == tool => {
                    list.push(args.to_string());
                }
                _ => {
                    flush(ui, &mut pending);
                    pending = Some((tool.to_string(), vec![args.to_string()]));
                }
            }
            rest = &after_open[end + CLOSE.len()..];
        }
        flush(ui, &mut pending);
        if !rest.is_empty() {
            Self::render_markdown(ui, rest);
        }
    }

    /// One Claude-Code-style tool-call pill. Color-coded by tool
    /// type (read=blue / write=amber / list=teal / bash=green); when
    /// a single pill carries multiple args (because consecutive same-
    /// tool calls were grouped) they're joined inline with `·`.
    fn render_tool_pill(ui: &mut egui::Ui, tool: &str, args: &[String]) {
        let (icon, accent) = match tool {
            "read_file" => ("📄", egui::Color32::from_rgb(126, 168, 255)), // blue
            "write_file" => ("✏", egui::Color32::from_rgb(248, 196, 96)),  // amber
            "list_files" => ("📁", egui::Color32::from_rgb(101, 214, 204)), // teal
            "run_bash" => ("▸", egui::Color32::from_rgb(144, 212, 160)),   // green
            _ => ("🔧", egui::Color32::from_rgb(170, 175, 190)),
        };
        let bg = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 22);
        let stroke = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 60);

        let joined_args = args
            .iter()
            .filter(|a| !a.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");

        ui.horizontal_wrapped(|ui| {
            egui::Frame::NONE
                .fill(bg)
                .stroke(egui::Stroke::new(1.0, stroke))
                .corner_radius(egui::CornerRadius::same(5))
                .inner_margin(egui::Margin::symmetric(8, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon).size(11.0).color(accent));
                        ui.label(
                            egui::RichText::new(tool)
                                .size(11.0)
                                .monospace()
                                .color(accent),
                        );
                        if !joined_args.is_empty() {
                            ui.label(
                                egui::RichText::new(joined_args)
                                    .size(11.0)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(170, 175, 190)),
                            );
                        }
                    });
                });
        });
        ui.add_space(2.0);
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
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(35, 35, 35))
                        .inner_margin(8)
                        .corner_radius(4)
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
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(35, 35, 35))
                .inner_margin(8)
                .corner_radius(4)
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

    /// Entry point from the chat panel's send button / Enter key. The
    /// dual Chat / Agent surface was retired — every prompt now flows
    /// through the agent loop ([`Self::send_agent_message`]), which
    /// covers chat-style Q&A through `read_file` plus actual editing
    /// through `write_file`. Kept as a thin wrapper so the input UI
    /// doesn't need to know which underlying path runs.
    pub(crate) fn send_ai_message(&mut self) {
        let message = self.ai_input.trim().to_string();
        if message.is_empty() {
            return;
        }
        self.send_agent_message(message);
    }

    /// Autonomous-mode counterpart to [`send_ai_message`]. Spawns an
    /// external coding agent (Claude Code today; Codex once we can
    /// verify the CLI surface) inside the project root and pumps the
    /// event stream into the chat panel as `AiChatResponse::ChatChunk`
    /// values, so the rendering side renders agent output identically
    /// to a normal chat response.
    ///
    /// `Edit` events are formatted as a small markdown-style header
    /// block so the user can see which file the agent intends to
    /// touch; the actual diff-viewer integration comes in Phase D.
    pub(crate) fn send_agent_message(&mut self, message: String) {
        self.ai_messages.push(AiChatMessage {
            content: message.clone(),
            is_user: true,
        });
        self.ai_input.clear();
        self.ai_streaming = true;
        self.ai_current_response.clear();
        self.ai_streaming_message = Some(String::new());

        let tx = self.ai_response_tx.clone();
        let cwd = std::path::PathBuf::from(self.root_path.clone());

        // Pull model + budget from BYOK settings; pick the agent
        // backend off `agent_backend` ("native" / "claude" / "codex").
        let model = Some(self.ai_settings.chat_model.clone());
        let max_budget = if self.ai_settings.monthly_cap_usd > 0.0 {
            Some(self.ai_settings.monthly_cap_usd)
        } else {
            None
        };
        let backend = self.ai_settings.agent_backend.clone();
        let provider_kind_for_usage = match backend.as_str() {
            "codex" | "native" => crate::ai::ProviderKind::OpenAi,
            _ => crate::ai::ProviderKind::Anthropic,
        };
        let ai_settings_for_native = self.ai_settings.clone();

        // Resolve `@<path>` references on the main thread (cheap fs
        // reads). Inlining these via `append_system_prompt` saves the
        // model a tool round-trip per attached file. Bundled with the
        // Bevy 0.18 cheatsheet (#14 MVP).
        let attached = collect_attached_files(&message, &self.root_path);
        let mut extra_system = String::from(
            "You are running inside BerryCode, a native Bevy IDE. \
             Prefer Bevy 0.18 idioms, propose small focused edits, \
             and explain your plan before applying changes.",
        );
        extra_system.push_str(BEVY_018_CHEATSHEET);
        if !attached.is_empty() {
            extra_system.push_str("\n\n## Attached files (referenced via @ in the user message)\n");
            for (path, content) in &attached {
                extra_system.push_str(&format!("\n### {}\n```\n{}\n```\n", path, content));
            }
        }

        self.lsp_runtime.spawn(async move {
            let agent: Box<dyn crate::agent::CodingAgent> = match backend.as_str() {
                "native" => Box::new(crate::agent::native::NativeAgent::new(
                    ai_settings_for_native,
                )),
                "codex" => Box::new(crate::agent::codex::CodexAgent::new()),
                _ => Box::new(crate::agent::claude::ClaudeCodeAgent::new()),
            };
            let opts = crate::agent::AgentRunOpts {
                model,
                max_budget_usd: max_budget,
                additional_dirs: Vec::new(),
                append_system_prompt: Some(extra_system),
            };

            let mut session = match agent.run(&message, &cwd, opts).await {
                Ok(s) => s,
                Err(e) => {
                    if let Some(tx) = &tx {
                        let _ = tx.send(AiChatResponse::ChatChunk(format!(
                            "!Agent failed to start: {}\n\
                                 Install Claude Code with `brew install anthropic/claude/claude` \
                                 or `npm install -g @anthropic-ai/claude-code` and re-run.",
                            e
                        )));
                        let _ = tx.send(AiChatResponse::ChatStreamCompleted);
                    }
                    return;
                }
            };

            // Keep the whole session alive across the recv loop so its
            // `Drop` impl can kill the child if we abort early.
            while let Some(ev) = session.events.recv().await {
                let Some(tx) = tx.as_ref() else {
                    continue;
                };
                match ev {
                    crate::agent::AgentEvent::AssistantMessage(text)
                    | crate::agent::AgentEvent::Output(text) => {
                        let _ = tx.send(AiChatResponse::ChatChunk(text));
                    }
                    crate::agent::AgentEvent::ToolUse { tool, input } => {
                        // Encode tool calls as a sentinel so the chat
                        // renderer can substitute a styled pill instead
                        // of dropping the call into the markdown stream
                        // as `[tool]` text. Format: leading + trailing
                        // newlines keep the pill on its own line; the
                        // payload is `<tool>|<arg-summary>` so the
                        // renderer doesn't need to re-parse JSON.
                        let summary = match tool.as_str() {
                            "read_file" | "write_file" | "list_files" => input
                                .get("path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            "run_bash" => {
                                let cmd =
                                    input.get("command").and_then(|v| v.as_str()).unwrap_or("");
                                if cmd.len() > 60 {
                                    format!("{}…", &cmd[..60])
                                } else {
                                    cmd.to_string()
                                }
                            }
                            _ => String::new(),
                        };
                        let _ = tx.send(AiChatResponse::ChatChunk(format!(
                            "\n<<BERRY_TOOL>>{}|{}<<END>>\n",
                            tool, summary
                        )));
                    }
                    crate::agent::AgentEvent::Edit {
                        path,
                        before,
                        after,
                    } => {
                        // Echo a short header in the chat stream so the
                        // user knows an edit was proposed even if they've
                        // scrolled away from the pending-edits cards…
                        let _ = tx.send(AiChatResponse::ChatChunk(format!(
                            "\n**[Edit]Edit proposed:** `{}`\n",
                            path.display()
                        )));
                        // …and queue the structured payload for the
                        // diff card. `poll_ai_responses` pushes it into
                        // `app.pending_agent_edits` on the main thread.
                        let _ = tx.send(AiChatResponse::PendingEdit {
                            path,
                            before,
                            after,
                        });
                    }
                    crate::agent::AgentEvent::Error(msg) => {
                        let _ = tx.send(AiChatResponse::ChatChunk(format!("\n!{}\n", msg)));
                    }
                    crate::agent::AgentEvent::Done { success, usage } => {
                        if let Some(usage) = usage {
                            let model_label = match provider_kind_for_usage {
                                crate::ai::ProviderKind::OpenAi => "codex-agent",
                                _ => "claude-code-agent",
                            };
                            crate::ai::usage::record(provider_kind_for_usage, model_label, &usage);
                        }
                        if !success {
                            let _ = tx.send(AiChatResponse::ChatChunk(
                                "\n!Agent run finished with errors.\n".to_string(),
                            ));
                        }
                        let _ = tx.send(AiChatResponse::ChatStreamCompleted);
                        break;
                    }
                }
            }
        });
    }

    /// Render the queue of edits proposed by the active coding agent,
    /// each as a small card with file path, a unified-style colourised
    /// diff, and Approve / Reject buttons. Rendered just above the
    /// chat input so the human stays in the loop on every disk write.
    /// v0.4.5 / Phase D.
    pub(crate) fn render_pending_agent_edits(&mut self, ui: &mut egui::Ui) {
        if self.pending_agent_edits.is_empty() {
            return;
        }

        // We may mutate `self.editor_tabs` while resolving an Approve;
        // drain decisions into local lists then act after the loop.
        let mut approve: Option<usize> = None;
        let mut reject: Option<usize> = None;

        // Take ownership for the render pass so we don't double-borrow
        // `self`. Push everything back in the rejected-only case.
        let edits = std::mem::take(&mut self.pending_agent_edits);

        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(28, 30, 36))
            .inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 8,
                bottom: 4,
            })
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "[Edit]{} pending edit{} from agent",
                        edits.len(),
                        if edits.len() == 1 { "" } else { "s" }
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(180, 200, 255))
                    .strong(),
                );
                ui.add_space(4.0);

                for (idx, edit) in edits.iter().enumerate() {
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(34, 36, 44))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 64, 72)))
                        .corner_radius(egui::CornerRadius::same(4))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{}", edit.path.display()))
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(220, 220, 220))
                                        .family(egui::FontFamily::Monospace),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(
                                                egui::RichText::new("Reject")
                                                    .color(egui::Color32::from_rgb(220, 120, 120)),
                                            )
                                            .clicked()
                                        {
                                            reject = Some(idx);
                                        }
                                        if ui
                                            .button(
                                                egui::RichText::new("Approve")
                                                    .color(egui::Color32::from_rgb(120, 220, 140))
                                                    .strong(),
                                            )
                                            .clicked()
                                        {
                                            approve = Some(idx);
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            // Compact unified diff. Caps at 200 lines
                            // total so the card stays scannable for big
                            // edits — full diff lives on disk after
                            // approve.
                            render_simple_unified_diff(
                                ui,
                                edit.before.as_deref().unwrap_or(""),
                                &edit.after,
                                200,
                            );
                        });
                    ui.add_space(4.0);
                }
            });

        // Restore the queue so any non-clicked entries persist.
        self.pending_agent_edits = edits;

        if let Some(idx) = approve {
            // Take the edit out and apply it. `try_apply_edit` writes
            // the file and reloads any open buffer pointing at it.
            let edit = self.pending_agent_edits.remove(idx);
            self.try_apply_edit(&edit);
        } else if let Some(idx) = reject {
            self.pending_agent_edits.remove(idx);
        }
    }

    /// Apply one approved agent edit to disk + reload any matching
    /// editor tab. Status messages report success / failure.
    ///
    /// 3-way safety check (#13): when the agent recorded a `before`
    /// snapshot, refuse to overwrite if the on-disk content has
    /// drifted from that snapshot — the user has edited the file
    /// since the agent read it, and a blind write would silently
    /// destroy their changes. We push the edit back into the pending
    /// queue so they can re-review or Reject. This is a guard, not a
    /// real merge — the proper line-level merge lives behind issue
    /// #13 once we wire in `diffy` / `similar`'s patch primitives.
    pub(crate) fn try_apply_edit(&mut self, edit: &super::types::PendingAgentEdit) {
        if let Some(parent) = edit.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Some(base) = edit.before.as_deref() {
            let current = std::fs::read_to_string(&edit.path).unwrap_or_default();
            if current != base {
                self.status_message = format!(
                    "!Skipped agent edit ({}): file changed since the agent read it. \
                     Reject and re-ask, or apply manually.",
                    edit.path.display()
                );
                self.status_message_timestamp = Some(std::time::Instant::now());
                self.pending_agent_edits.push(edit.clone());
                return;
            }
        }

        match std::fs::write(&edit.path, &edit.after) {
            Ok(()) => {
                self.status_message = format!("Applied agent edit: {}", edit.path.display());
                self.status_message_timestamp = Some(std::time::Instant::now());
                // Reload the buffer for any open tab pointing at this
                // file so the editor shows the new contents instead of
                // a stale copy.
                let path_str = edit.path.to_string_lossy().to_string();
                for tab in self.editor_tabs.iter_mut() {
                    if tab.file_path == path_str {
                        if let Ok(content) = std::fs::read_to_string(&edit.path) {
                            tab.buffer = crate::buffer::TextBuffer::from_str(&content);
                            tab.text_cache = content;
                        }
                    }
                }
            }
            Err(e) => {
                self.status_message =
                    format!("!Failed to apply edit ({}): {}", edit.path.display(), e);
                self.status_message_timestamp = Some(std::time::Instant::now());
            }
        }
    }

    pub(crate) fn poll_ai_responses(&mut self) {
        if let Some(rx) = &mut self.ai_response_rx {
            while let Ok(response) = rx.try_recv() {
                match response {
                    AiChatResponse::SessionStarted(_) => {
                        tracing::info!("✅ AI Chat ready (berry-core-api)");
                        self.ai_connected = true;
                        self.status_message = "✅ AI Chat ready".to_string();
                        self.status_message_timestamp = Some(std::time::Instant::now());
                    }
                    AiChatResponse::ChatChunk(chunk) => {
                        // Streaming chunks arrive 1-token at a time during
                        // a Native-agent response — info-level logging
                        // here used to spam ~100 lines per answer and
                        // showed up as visible stutter in the chat panel.
                        // Demoted to trace; flip with `RUST_LOG=trace`
                        // if you need to debug streaming.
                        tracing::trace!("UI chunk: {} chars", chunk.len());

                        self.ai_current_response.push_str(&chunk);

                        if let Some(streaming_msg) = &mut self.ai_streaming_message {
                            streaming_msg.push_str(&chunk);
                        } else {
                            self.ai_streaming_message = Some(String::new());
                            if let Some(streaming_msg) = &mut self.ai_streaming_message {
                                streaming_msg.push_str(&chunk);
                            }
                        }
                    }
                    AiChatResponse::ChatStreamCompleted => {
                        tracing::info!("✅ Chat stream completed");

                        if !self.ai_current_response.is_empty() {
                            let stripped = strip_thinking_blocks(&self.ai_current_response);
                            let content = if stripped.is_empty() {
                                self.ai_current_response.trim().to_string()
                            } else {
                                stripped
                            };
                            self.ai_messages.push(AiChatMessage {
                                content,
                                is_user: false,
                            });
                            self.ai_current_response.clear();
                        }

                        self.ai_streaming = false;
                        self.ai_streaming_message = None;
                    }
                    AiChatResponse::PendingEdit {
                        path,
                        before,
                        after,
                    } => {
                        // Coalesce repeat proposals on the same path:
                        // if the agent revises a still-pending edit,
                        // we'd rather show the latest single card than
                        // stack duplicates.
                        if let Some(existing) =
                            self.pending_agent_edits.iter_mut().find(|e| e.path == path)
                        {
                            existing.before = before;
                            existing.after = after;
                        } else {
                            self.pending_agent_edits
                                .push(super::types::PendingAgentEdit {
                                    path,
                                    before,
                                    after,
                                });
                        }
                    }
                }
            }
        }
    }
}

/// Compact unified-diff renderer for the pending-edit cards. Computes
/// a line-level LCS-style diff between `before` and `after` and emits
/// Bevy 0.18 cheatsheet injected into the chat system prompt (#14
/// MVP). Pre-RAG fallback: a static set of API reminders covering the
/// pieces the model gets wrong most often (0.18 renamed
/// `ImagePlugin::default_nearest()`, dropped `App::add_state`, moved
/// `bevy::input::mouse` → `bevy::input::mouse`, etc.). Build-time
/// indexing of the real Bevy docs is the v1 of issue #14.
const BEVY_018_CHEATSHEET: &str = "

## Bevy 0.18 reminders
- App entry: `App::new().add_plugins(DefaultPlugins).run();`
- Spawn 3D: `commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_xyz(...)))`. `PbrBundle` is gone — use the component tuple form.
- Camera 3D: `commands.spawn((Camera3d::default(), Transform::from_xyz(...).looking_at(Vec3::ZERO, Vec3::Y)))`.
- States: `app.init_state::<S>()` + `OnEnter(S::Variant)` schedules; the older `app.add_state::<S>()` is removed.
- Events: `MessageReader<E>` / `MessageWriter<E>` (renamed from `EventReader` / `EventWriter` in 0.18).
- Asset load: `asset_server.load::<Mesh>(\"path\")`; loaded handles are `Handle<T>`, attached as components.
- Camera 2D + UI: `Camera2d` (no bundle) + `Node` for UI roots; flexbox layout via `Node` fields.
- ECS queries: `Query<&T>` for read, `Query<&mut T>` for write, `Query<Entity, With<T>>` for filtering. `Single<...>` resolves to one entity (replaces the old `single_mut()` ergonomics in some places).
- Time delta: `time.delta_secs()` (was `delta_seconds()`).
- Color literals: `Color::srgb(r, g, b)` / `Color::srgba(r, g, b, a)`; `Color::rgb` is gone.
";

/// Resolve `@<path>` references in a chat message — read each file
/// against `root_path` and return `(rel_path, content)` pairs to
/// inject into the system prompt. Skips files larger than 200 KB
/// individually or 1 MB in aggregate; capped at 10 files.
fn collect_attached_files(message: &str, root_path: &str) -> Vec<(String, String)> {
    use std::path::PathBuf;
    const MAX_PER_FILE: usize = 200 * 1024;
    const MAX_TOTAL: usize = 1024 * 1024;
    const MAX_COUNT: usize = 10;

    let root = PathBuf::from(root_path);
    let mut files: Vec<(String, String)> = Vec::new();
    let mut total = 0usize;

    for tok in message.split_whitespace() {
        if files.len() >= MAX_COUNT {
            break;
        }
        let Some(rest) = tok.strip_prefix('@') else {
            continue;
        };
        let path_str = rest.trim_end_matches(|c: char| ".,;:!?)".contains(c));
        if path_str.is_empty() {
            continue;
        }
        let path = root.join(path_str);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.len() > MAX_PER_FILE || total + bytes.len() > MAX_TOTAL {
            continue;
        }
        let content = String::from_utf8_lossy(&bytes).into_owned();
        total += content.len();
        files.push((path_str.to_string(), content));
    }
    files
}

/// monospace coloured rows so additions / removals stand out at a
/// glance. Capped at `max_lines` total to keep large edits readable;
/// the full text is still applied on Approve.
fn render_simple_unified_diff(ui: &mut egui::Ui, before: &str, after: &str, max_lines: usize) {
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();

    // Greedy line-pair walk: if lines match, emit a context row;
    // otherwise emit "removed" for `before` then "added" for `after`.
    // Good enough for the common single-block-of-changes case the
    // agents emit; a real LCS can come later.
    let mut bi = 0;
    let mut ai = 0;
    let mut emitted = 0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Monospace);

    while (bi < before_lines.len() || ai < after_lines.len()) && emitted < max_lines {
        match (before_lines.get(bi), after_lines.get(ai)) {
            (Some(b), Some(a)) if b == a => {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(" {}", b))
                            .font(font.clone())
                            .color(egui::Color32::from_rgb(150, 155, 165)),
                    );
                });
                bi += 1;
                ai += 1;
            }
            (Some(b), _) if !after_lines.contains(b) => {
                ui.horizontal(|ui| {
                    let line = ui.painter().clone();
                    let rect = ui
                        .allocate_response(
                            egui::vec2(ui.available_width(), 18.0),
                            egui::Sense::hover(),
                        )
                        .rect;
                    line.rect_filled(rect, 0.0, egui::Color32::from_rgb(60, 28, 28));
                    line.text(
                        rect.left_top() + egui::vec2(4.0, 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("-{}", b),
                        font.clone(),
                        egui::Color32::from_rgb(255, 180, 180),
                    );
                });
                bi += 1;
            }
            (_, Some(a)) => {
                ui.horizontal(|ui| {
                    let line = ui.painter().clone();
                    let rect = ui
                        .allocate_response(
                            egui::vec2(ui.available_width(), 18.0),
                            egui::Sense::hover(),
                        )
                        .rect;
                    line.rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 60, 32));
                    line.text(
                        rect.left_top() + egui::vec2(4.0, 2.0),
                        egui::Align2::LEFT_TOP,
                        format!("+{}", a),
                        font.clone(),
                        egui::Color32::from_rgb(180, 255, 180),
                    );
                });
                ai += 1;
            }
            _ => break,
        }
        emitted += 1;
    }

    let remaining = before_lines.len().saturating_sub(bi) + after_lines.len().saturating_sub(ai);
    if remaining > 0 {
        ui.label(
            egui::RichText::new(format!("… {} more line(s) elided", remaining))
                .size(11.0)
                .color(egui::Color32::from_rgb(140, 145, 160))
                .italics(),
        );
    }
}
