//! Settings panel, color scheme settings, theme editor

use super::syntax_colors;
use super::ui_colors;
use super::BerryCodeApp;
use crate::app::i18n::t;

impl BerryCodeApp {
    /// RustRover-style Settings Panel
    pub(crate) fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("⚙️ {}", t(self.ui_language, "Settings")));
        ui.separator();

        ui.horizontal_top(|ui| {
            // --- Left Navigation (150px width) ---
            ui.vertical(|ui| {
                ui.set_width(150.0);
                ui.add_space(8.0);

                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::Appearance,
                    t(self.ui_language, "Appearance"),
                );
                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::EditorColor,
                    t(self.ui_language, "Editor > Color Scheme"),
                );
                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::Keybindings,
                    t(self.ui_language, "Keybindings"),
                );
                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::Language,
                    t(self.ui_language, "Language"),
                );

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new(t(self.ui_language, "Plugins"))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::GitHub,
                    t(self.ui_language, "GitHub Review"),
                );
                ui.selectable_value(
                    &mut self.active_settings_tab,
                    super::types::SettingsTab::Plugins,
                    t(self.ui_language, "Other Plugins"),
                );
            });

            ui.separator();

            // --- Right Content Area ---
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_settings_tab {
                        super::types::SettingsTab::EditorColor => {
                            self.render_color_scheme_settings(ui);
                        }
                        super::types::SettingsTab::Keybindings => {
                            self.render_keybindings_settings(ui);
                        }
                        super::types::SettingsTab::Language => {
                            use super::types::UiLanguage;
                            let heading = match self.ui_language {
                                UiLanguage::English => "Language",
                                UiLanguage::Japanese => "言語設定",
                            };
                            ui.heading(heading);
                            ui.add_space(8.0);

                            let label = match self.ui_language {
                                UiLanguage::English => "UI Language",
                                UiLanguage::Japanese => "表示言語",
                            };
                            ui.label(label);
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        self.ui_language == UiLanguage::English,
                                        "English",
                                    )
                                    .clicked()
                                {
                                    self.ui_language = UiLanguage::English;
                                }
                                if ui
                                    .selectable_label(
                                        self.ui_language == UiLanguage::Japanese,
                                        "日本語",
                                    )
                                    .clicked()
                                {
                                    self.ui_language = UiLanguage::Japanese;
                                }
                            });
                        }
                        super::types::SettingsTab::Appearance => {
                            ui.heading(t(self.ui_language, "Appearance"));
                            ui.label(t(self.ui_language, "Window theme, font settings, etc."));
                            ui.add_space(12.0);

                            // Font size info
                            ui.label("Editor font: monospace 13.0px (default)");
                            ui.add_space(8.0);

                            // Theme selector
                            ui.label("Theme:");
                            ui.horizontal(|ui| {
                                if ui.button("Open Theme Editor").clicked() {
                                    self.show_theme_editor = true;
                                }
                                if ui.button("Open Color Scheme").clicked() {
                                    self.active_settings_tab =
                                        super::types::SettingsTab::EditorColor;
                                }
                            });
                        }
                        super::types::SettingsTab::GitHub => {
                            ui.heading(t(self.ui_language, "GitHub Review"));
                            ui.add_space(8.0);
                            ui.label("Review pull requests directly in the editor.");
                            ui.label("Features planned:");
                            ui.indent("gh_features", |ui| {
                                ui.label("- Browse open pull requests from within BerryCode");
                                ui.label("- Inline diff view with comment threads");
                                ui.label("- Submit reviews (approve / request changes)");
                                ui.label("- Resolve conversations and merge PRs");
                            });
                            ui.add_space(8.0);
                            ui.colored_label(
                                egui::Color32::from_rgb(140, 140, 140),
                                "Requires GitHub CLI (gh) authentication.",
                            );
                        }
                        super::types::SettingsTab::Plugins => {
                            ui.heading(t(self.ui_language, "Other Plugins"));
                            ui.add_space(8.0);
                            let count = self.plugin_manager.plugins.len();
                            ui.label(format!("Installed plugins: {}", count));
                            ui.add_space(4.0);
                            if count == 0 {
                                ui.label(
                                    "No plugins installed. Place plugins in ~/.berrycode/plugins/",
                                );
                            } else {
                                for plugin in &self.plugin_manager.plugins {
                                    ui.horizontal(|ui| {
                                        let status = if plugin.enabled {
                                            "enabled"
                                        } else {
                                            "disabled"
                                        };
                                        ui.label(format!(
                                            "  {} v{} ({})",
                                            plugin.manifest.name, plugin.manifest.version, status
                                        ));
                                    });
                                }
                            }
                        }
                    });
            });
        });
    }

    /// Color Scheme Settings (RustRover Darcula)
    pub(crate) fn render_color_scheme_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(t(self.ui_language, "Color Scheme: Darcula (Customized)"));
        ui.label(t(self.ui_language, "Customize syntax highlighting colors:"));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.keyword_color);
            ui.label(t(self.ui_language, "Keyword (fn, let, match)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.function_color);
            ui.label(t(self.ui_language, "Function / Macro"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.type_color);
            ui.label(t(self.ui_language, "Type (struct, enum)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.string_color);
            ui.label(t(self.ui_language, "String"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.number_color);
            ui.label(t(self.ui_language, "Number"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.comment_color);
            ui.label(t(self.ui_language, "Comment"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.macro_color);
            ui.label(t(self.ui_language, "Macro (println!)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.attribute_color);
            ui.label(t(self.ui_language, "Attribute (#[derive])"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.constant_color);
            ui.label(t(self.ui_language, "Constant (STATIC)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.lifetime_color);
            ui.label(t(self.ui_language, "Lifetime ('a, 'static)"));
        });

        ui.add_space(20.0);
        ui.separator();
        ui.label(egui::RichText::new(t(self.ui_language, "Live Preview:")).strong());
        ui.add_space(8.0);
        self.render_color_preview(ui);

        ui.add_space(16.0);
        if ui
            .button(format!(
                "🔄 {}",
                t(self.ui_language, "Reset to Darcula Defaults")
            ))
            .clicked()
        {
            self.reset_colors_to_darcula();
        }
    }

    /// Live preview of syntax colors
    pub(crate) fn render_color_preview(&self, ui: &mut egui::Ui) {
        let frame = egui::Frame::none()
            .fill(ui_colors::SIDEBAR_BG)
            .inner_margin(12.0)
            .rounding(4.0);

        frame.show(ui, |ui| {
            ui.style_mut().override_font_id = Some(egui::FontId::monospace(13.0));

            ui.horizontal(|ui| {
                ui.colored_label(self.keyword_color, "fn");
                ui.label(" ");
                ui.colored_label(self.function_color, "main");
                ui.label("() {");
            });

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.keyword_color, "let");
                ui.label(" x: ");
                ui.colored_label(self.type_color, "u32");
                ui.label(" = ");
                ui.colored_label(self.number_color, "42");
                ui.label(";");
            });

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.comment_color, "// Hello World");
            });

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.macro_color, "println!");
                ui.label("(");
                ui.colored_label(self.string_color, "\"Ready!\"");
                ui.label(");");
            });

            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.colored_label(self.keyword_color, "const");
                ui.label(" ");
                ui.colored_label(self.constant_color, "MAX");
                ui.label(": ");
                ui.colored_label(self.type_color, "usize");
                ui.label(" = ");
                ui.colored_label(self.number_color, "100");
                ui.label(";");
            });

            ui.label("}");
        });
    }

    /// Reset colors to VS Code Dark+ defaults
    pub(crate) fn reset_colors_to_darcula(&mut self) {
        self.keyword_color = syntax_colors::KEYWORD;
        self.function_color = syntax_colors::FUNCTION;
        self.type_color = syntax_colors::TYPE;
        self.string_color = syntax_colors::STRING;
        self.number_color = syntax_colors::NUMBER;
        self.comment_color = syntax_colors::COMMENT;
        self.doc_comment_color = syntax_colors::DOC_COMMENT;
        self.macro_color = syntax_colors::MACRO;
        self.attribute_color = syntax_colors::ATTRIBUTE;
        self.constant_color = syntax_colors::CONSTANT;
        self.lifetime_color = syntax_colors::LIFETIME;
        self.namespace_color = syntax_colors::NAMESPACE;
        self.variable_color = syntax_colors::VARIABLE;
        self.operator_color = syntax_colors::OPERATOR;
        tracing::info!("🎨 Reset colors to VS Code Dark+ defaults");
    }

    /// Render Settings dialog
    pub(crate) fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("⚙️ Settings")
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Editor Settings");
                    ui.separator();

                    ui.label("Font size:");
                    ui.label("  13.0px (monospace, fixed)");
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 120, 120),
                        "Font size customization will be available in a future release.",
                    );
                    ui.add_space(8.0);

                    ui.label("Tab size:");
                    ui.label("  4 spaces (fixed)");
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 120, 120),
                        "Tab size customization will be available in a future release.",
                    );
                    ui.add_space(8.0);

                    ui.label("Theme:");
                    if ui.button("Open Theme Editor").clicked() {
                        self.show_theme_editor = true;
                    }
                });
                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
    }

    /// Render Theme editor
    pub(crate) fn render_theme_editor(&mut self, ctx: &egui::Context) {
        egui::Window::new("🎨 Theme Editor")
            .collapsible(false)
            .resizable(true)
            .default_size([600.0, 500.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Color Scheme");
                    ui.separator();

                    let mut visuals = ctx.style().visuals.clone();

                    ui.label("Widget colors:");
                    egui::Grid::new("theme_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Background:");
                            ui.color_edit_button_srgba(&mut visuals.panel_fill);
                            ui.end_row();

                            ui.label("Text:");
                            let mut text_color = visuals.text_color();
                            ui.color_edit_button_srgba(&mut text_color);
                            ui.end_row();

                            ui.label("Selection:");
                            ui.color_edit_button_srgba(&mut visuals.selection.bg_fill);
                            ui.end_row();

                            ui.label("Window fill:");
                            ui.color_edit_button_srgba(&mut visuals.window_fill);
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    if ui.button("Apply Theme").clicked() {
                        ctx.set_visuals(visuals);
                        tracing::info!("🎨 Theme updated");
                    }
                });

                ui.separator();

                if ui.button("Close").clicked() {
                    self.show_theme_editor = false;
                }
            });
    }

    /// Keyboard Shortcuts settings tab
    pub(crate) fn render_keybindings_settings(&mut self, ui: &mut egui::Ui) {
        use super::keymap::KeyAction;

        ui.heading(t(self.ui_language, "Keybindings"));
        ui.add_space(4.0);
        ui.label(
            "Current keybinding assignments. Edit keybindings.ron for advanced customization.",
        );
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("keybindings_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Action");
                ui.strong("Shortcut");
                ui.end_row();

                for action in KeyAction::ALL {
                    ui.label(action.label());
                    if let Some(binding) = self.keymap.bindings.get(action) {
                        ui.monospace(binding.display());
                    } else {
                        ui.label("(unbound)");
                    }
                    ui.end_row();
                }
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Reset to Defaults").clicked() {
                self.keymap = super::keymap::Keymap::default();
                self.keymap.save();
                tracing::info!("Keyboard shortcuts reset to defaults");
            }

            if ui.button("Save to File").clicked() {
                self.keymap.save();
                tracing::info!("Keyboard shortcuts saved");
            }
        });

        ui.add_space(8.0);
        let path = if let Some(home) = dirs::home_dir() {
            format!("{}/.berrycode/keybindings.ron", home.display())
        } else {
            "~/.berrycode/keybindings.ron".to_string()
        };
        ui.label(
            egui::RichText::new(format!("Config file: {}", path))
                .small()
                .color(egui::Color32::GRAY),
        );
    }

    /// Handle keyboard shortcuts for Settings and Theme
    pub(crate) fn handle_settings_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Comma) {
                tracing::info!("⚙️ Opening settings");
                self.show_settings = !self.show_settings;
            }

            if i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                tracing::info!("🎨 Opening theme editor");
                self.show_theme_editor = !self.show_theme_editor;
            }

            if i.key_pressed(egui::Key::Escape) {
                if self.show_settings {
                    self.show_settings = false;
                }
                if self.show_theme_editor {
                    self.show_theme_editor = false;
                }
            }
        });
    }
}
