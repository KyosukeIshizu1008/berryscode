// Slack-like chat UI components for BerryCodeApp
// This file contains the implementation of Slack-style chat features

use crate::egui_app::{BerryCodeApp, ChatChannel, ChatMessage, ChannelType};

impl BerryCodeApp {
    /// Render channel list (left sidebar)
    pub fn render_channel_list(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("BerryCode").strong().size(18.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+").clicked() {
                        self.show_channel_browser = !self.show_channel_browser;
                    }
                });
            });

            ui.separator();

            // Search
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.chat_search_query)
                        .hint_text("🔍 Search")
                        .desired_width(f32::INFINITY)
                );
            });

            ui.add_space(8.0);

            // Channels section
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Channels").small().color(egui::Color32::GRAY));
                    ui.add_space(4.0);

                    let mut channel_to_select: Option<String> = None;

                    for channel in &self.chat_channels {
                        let is_selected = self.selected_channel_id.as_ref() == Some(&channel.id);

                        let icon = match channel.channel_type {
                            ChannelType::Public => "#",
                            ChannelType::Private => "🔒",
                            ChannelType::DirectMessage => "💬",
                        };

                        let label_text = if channel.unread_count > 0 {
                            format!("{} {} ({})", icon, channel.name, channel.unread_count)
                        } else {
                            format!("{} {}", icon, channel.name)
                        };

                        let mut rich_text = egui::RichText::new(&label_text);
                        if is_selected {
                            rich_text = rich_text.color(egui::Color32::WHITE).strong();
                        } else if channel.unread_count > 0 {
                            rich_text = rich_text.color(egui::Color32::WHITE);
                        } else {
                            rich_text = rich_text.color(egui::Color32::GRAY);
                        }

                        let response = ui.selectable_label(is_selected, rich_text);

                        if response.clicked() {
                            channel_to_select = Some(channel.id.clone());
                        }

                        // Context menu for channels
                        response.context_menu(|ui| {
                            if ui.button("📝 Rename").clicked() {
                                ui.close_menu();
                            }
                            if ui.button("📌 Pin").clicked() {
                                ui.close_menu();
                            }
                            if ui.button("🗑 Delete").clicked() {
                                ui.close_menu();
                            }
                        });
                    }

                    if let Some(channel_id) = channel_to_select {
                        self.selected_channel_id = Some(channel_id);
                        self.show_thread_panel = false;
                    }

                    ui.add_space(16.0);

                    // Add channel button
                    if ui.button("+ Add Channel").clicked() {
                        self.show_channel_browser = true;
                    }
                });

            // Channel browser dialog
            if self.show_channel_browser {
                egui::Window::new("Create Channel")
                    .collapsible(false)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        ui.label("Channel name:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_channel_name)
                                .hint_text("e.g. project-updates")
                        );

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui.button("Create").clicked() && !self.new_channel_name.is_empty() {
                                let channel_id = format!("channel_{}", self.chat_channels.len());
                                self.chat_channels.push(ChatChannel::new(
                                    channel_id.clone(),
                                    self.new_channel_name.clone(),
                                    ChannelType::Public,
                                ));
                                self.selected_channel_id = Some(channel_id);
                                self.new_channel_name.clear();
                                self.show_channel_browser = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_channel_browser = false;
                                self.new_channel_name.clear();
                            }
                        });
                    });
            }
        });
    }

    /// Render message area (center panel)
    pub fn render_message_area(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header with channel name
            if let Some(channel_id) = &self.selected_channel_id {
                if let Some(channel) = self.chat_channels.iter().find(|c| &c.id == channel_id).cloned() {
                    ui.horizontal(|ui| {
                        let icon = match channel.channel_type {
                            ChannelType::Public => "#",
                            ChannelType::Private => "🔒",
                            ChannelType::DirectMessage => "💬",
                        };
                        ui.heading(egui::RichText::new(format!("{} {}", icon, channel.name)).size(16.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("⋮").clicked() {
                                // Channel settings
                            }
                            if ui.button("📌").clicked() {
                                // Show pinned messages
                            }
                            if ui.button("🔍").clicked() {
                                // Search in channel
                            }
                        });
                    });

                    if !channel.description.is_empty() {
                        ui.label(egui::RichText::new(&channel.description).small().color(egui::Color32::GRAY));
                    }

                    ui.separator();

                    // Messages area
                    let available_height = ui.available_height() - 100.0;
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .max_height(available_height)
                        .show(ui, |ui| {
                            let channel_messages = channel.messages.clone();

                            if channel_messages.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(80.0);
                                    ui.label(egui::RichText::new("💬").size(48.0));
                                    ui.add_space(16.0);
                                    ui.label(egui::RichText::new("No messages yet").color(egui::Color32::GRAY));
                                });
                            } else {
                                for msg in &channel_messages {
                                    self.render_message(ui, msg);
                                }
                            }
                        });

                    ui.add_space(8.0);

                    // Input area at bottom
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(35, 36, 38))
                        .inner_margin(egui::Margin::same(12.0))
                        .rounding(8.0)
                        .show(ui, |ui| {
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut self.chat_input)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(2)
                                    .hint_text("Message...")
                                    .frame(false)
                            );

                            // Send on Enter
                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.send_message_to_channel();
                                response.request_focus();
                            }

                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                if ui.button("📎").on_hover_text("Attach file").clicked() {
                                    // TODO: Attach file
                                }
                                if ui.button("😀").on_hover_text("Emoji").clicked() {
                                    // TODO: Emoji picker
                                }
                                if ui.button("@").on_hover_text("Mention").clicked() {
                                    // TODO: Mention user
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Send ▶").clicked() {
                                        self.send_message_to_channel();
                                    }
                                });
                            });
                        });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label(egui::RichText::new("Channel not found").color(egui::Color32::GRAY));
                    });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.label(egui::RichText::new("Select a channel").size(20.0).color(egui::Color32::GRAY));
                });
            }
        });
    }

    /// Render a single message
    fn render_message(&mut self, ui: &mut egui::Ui, msg: &ChatMessage) {
        ui.horizontal(|ui| {
            // User avatar (placeholder)
            ui.label(egui::RichText::new("👤").size(32.0));

            ui.vertical(|ui| {
                // User name and timestamp
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&msg.user_name).strong());
                    ui.label(egui::RichText::new(
                        msg.timestamp.format("%H:%M").to_string()
                    ).small().color(egui::Color32::GRAY));

                    if msg.edited {
                        ui.label(egui::RichText::new("(edited)").small().color(egui::Color32::GRAY));
                    }
                });

                // Message content
                ui.label(&msg.content);

                // Thread replies indicator
                if !msg.thread_replies.is_empty() {
                    let reply_text = format!("{} replies →", msg.thread_replies.len());
                    if ui.link(egui::RichText::new(reply_text).small().color(egui::Color32::from_rgb(100, 150, 255))).clicked() {
                        self.selected_message_for_thread = Some(msg.id.clone());
                        self.show_thread_panel = true;
                    }
                }

                // Reactions
                if !msg.reactions.is_empty() {
                    ui.horizontal(|ui| {
                        for reaction in &msg.reactions {
                            let label = format!("{} {}", reaction.emoji, reaction.user_ids.len());
                            if ui.small_button(label).clicked() {
                                // Toggle reaction
                            }
                        }
                    });
                }

                // Hover actions
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.button_padding = egui::vec2(4.0, 4.0);

                    if ui.small_button("💬").on_hover_text("Reply in thread").clicked() {
                        self.selected_message_for_thread = Some(msg.id.clone());
                        self.show_thread_panel = true;
                    }
                    if ui.small_button("😀").on_hover_text("Add reaction").clicked() {
                        // TODO: Emoji picker
                    }
                    if ui.small_button("📋").on_hover_text("Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = msg.content.clone());
                    }
                    if ui.small_button("⋮").on_hover_text("More actions").clicked() {
                        // TODO: More actions menu
                    }
                });
            });
        });

        ui.add_space(12.0);
    }

    /// Render thread panel (right sidebar)
    pub fn render_thread_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("Thread").size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").clicked() {
                        self.show_thread_panel = false;
                        self.selected_message_for_thread = None;
                    }
                });
            });

            ui.separator();

            // Find the message and its thread
            if let Some(message_id) = &self.selected_message_for_thread {
                if let Some(channel_id) = &self.selected_channel_id {
                    if let Some(channel) = self.chat_channels.iter().find(|c| &c.id == channel_id) {
                        if let Some(parent_msg) = channel.messages.iter().find(|m| &m.id == message_id).cloned() {
                            // Parent message
                            let replies = parent_msg.thread_replies.clone();

                            ui.group(|ui| {
                                self.render_message(ui, &parent_msg);
                            });

                            ui.separator();

                            // Thread replies
                            ui.label(egui::RichText::new(format!("{} replies", parent_msg.thread_replies.len())).strong());
                            ui.add_space(8.0);

                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .max_height(ui.available_height() - 100.0)
                                .show(ui, |ui| {
                                    for reply in &replies {
                                        self.render_message(ui, reply);
                                    }
                                });

                            // Reply input
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.chat_input)
                                        .desired_width(f32::INFINITY)
                                        .hint_text("Reply...")
                                );
                                if ui.button("Send").clicked() {
                                    // TODO: Send thread reply
                                }
                            });
                        }
                    }
                }
            }
        });
    }

    /// Send message to current channel
    fn send_message_to_channel(&mut self) {
        if self.chat_input.trim().is_empty() {
            return;
        }

        if let Some(channel_id) = &self.selected_channel_id {
            let message = ChatMessage::new(
                channel_id.clone(),
                self.current_user_id.clone(),
                self.current_user_name.clone(),
                self.chat_input.clone(),
            );

            if let Some(channel) = self.chat_channels.iter_mut().find(|c| &c.id == channel_id) {
                channel.messages.push(message);
            }

            self.chat_input.clear();
        }
    }
}
