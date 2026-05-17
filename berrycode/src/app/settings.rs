//! Settings panel, color scheme settings, theme editor

use super::syntax_colors;
use super::ui_colors;
use super::BerryCodeApp;
use crate::app::i18n::t;
use chrono::Datelike;

impl BerryCodeApp {
    /// RustRover-style Settings Panel
    pub(crate) fn render_settings_panel(&mut self, ui: &mut egui::Ui) {
        use super::ui_colors;

        // VS Code-style header strip with the settings title and a
        // global search box. Layout below splits into a fixed-width nav
        // column on the left and a scrolled content area on the right.
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Settings")
                    .size(16.0)
                    .color(ui_colors::SETTINGS_HEADER())
                    .strong(),
            );
            ui.add_space(16.0);
            // Search box (placeholder state until v0.4.x search lands).
            let search_frame = egui::Frame::NONE
                .fill(ui_colors::SETTINGS_SEARCH_BG())
                .stroke(egui::Stroke::new(1.0, ui_colors::SETTINGS_CARD_BORDER()))
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 4));
            search_frame.show(ui, |ui| {
                ui.set_min_width(380.0);
                ui.label(
                    egui::RichText::new("Search settings (coming soon)")
                        .small()
                        .color(ui_colors::SETTINGS_HINT()),
                );
            });
        });
        ui.add_space(4.0);
        ui.painter().hline(
            ui.max_rect().x_range(),
            ui.cursor().min.y,
            egui::Stroke::new(1.0, ui_colors::SETTINGS_CARD_BORDER()),
        );
        ui.add_space(8.0);

        ui.horizontal_top(|ui| {
            // --- Left Navigation (220px) — VS Code-style two-pane split.
            // Wrap in `ui.vertical` so items stack top-to-bottom; the
            // outer `horizontal_top` would otherwise lay them on a single
            // row.
            egui::Frame::NONE
                .fill(ui_colors::SETTINGS_NAV_BG())
                .inner_margin(egui::Margin::symmetric(8, 12))
                .show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.set_min_height(ui.available_height());
                    ui.vertical(|ui| {
                        ui.set_width(220.0 - 16.0); // minus inner margin

                        nav_section_header(ui, "Application");
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::Appearance,
                            t(self.ui_language, "Appearance"),
                        );
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::Language,
                            t(self.ui_language, "Language"),
                        );
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::Keybindings,
                            t(self.ui_language, "Keybindings"),
                        );

                        ui.add_space(12.0);
                        nav_section_header(ui, "Editor");
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::EditorColor,
                            "Color Scheme",
                        );

                        ui.add_space(12.0);
                        nav_section_header(ui, "Workbench");
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::Panels,
                            "Activity Bar",
                        );

                        ui.add_space(12.0);
                        nav_section_header(ui, "Features");
                        #[cfg(feature = "ai")]
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::AiProviders,
                            "AI Providers",
                        );
                        #[cfg(feature = "ai")]
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::AiUsage,
                            "AI Usage & Cost",
                        );

                        ui.add_space(12.0);
                        nav_section_header(ui, &self.tr("Plugins"));
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::GitHub,
                            t(self.ui_language, "GitHub Review"),
                        );
                        nav_item(
                            ui,
                            &mut self.active_settings_tab,
                            super::types::SettingsTab::Plugins,
                            t(self.ui_language, "Other Plugins"),
                        );
                    });
                });

            ui.add_space(8.0);

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
                            ui.heading(self.tr("Appearance"));
                            ui.label(self.tr("Window theme, font settings, etc."));
                            ui.add_space(12.0);

                            // Theme preset switcher. Light / High Contrast
                            // are disabled until v0.4.x finishes auditing
                            // the hardcoded `ui_colors::*` constants — only
                            // the Dark preset currently renders all panels
                            // (editor, sidebar, terminal) with consistent
                            // colours.
                            ui.label(egui::RichText::new("Theme preset").strong());
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                use super::types::ThemeMode;
                                let apply = |this: &mut Self, mode: ThemeMode, ctx: &egui::Context| {
                                    this.theme_mode = mode;
                                    ctx.set_visuals(super::visuals_for_theme(mode));
                                    super::ui_colors::set_theme(mode);
                                    super::save_theme_mode(mode);
                                };
                                let dark_sel = self.theme_mode == ThemeMode::Dark;
                                if ui.selectable_label(dark_sel, "Dark").clicked() && !dark_sel {
                                    apply(self, ThemeMode::Dark, ui.ctx());
                                }
                                let light_sel = self.theme_mode == ThemeMode::Light;
                                if ui.selectable_label(light_sel, "Light").clicked() && !light_sel {
                                    apply(self, ThemeMode::Light, ui.ctx());
                                }
                                let hc_sel = self.theme_mode == ThemeMode::HighContrast;
                                if ui.selectable_label(hc_sel, "High Contrast").clicked()
                                    && !hc_sel
                                {
                                    apply(self, ThemeMode::HighContrast, ui.ctx());
                                }
                            });
                            ui.add_space(12.0);

                            // Editor font size — slider with live preview.
                            // Pushed into the global atomic in
                            // `render_editor_area` every frame so the change
                            // shows up immediately without restart.
                            ui.label(egui::RichText::new("Editor font size").strong());
                            ui.add_space(4.0);
                            let mut size = self.settings.font_size as i32;
                            if ui
                                .add(
                                    egui::Slider::new(&mut size, 8..=32)
                                        .text("px")
                                        .clamping(egui::SliderClamping::Always),
                                )
                                .changed()
                            {
                                self.settings.font_size = size as u32;
                                let _ = self.settings.save();
                            }
                            ui.add_space(12.0);

                            // Format-on-save — runs `textDocument/formatting`
                            // through the LSP before writing to disk. Skips
                            // the format if no language server is attached.
                            ui.label(egui::RichText::new("On save").strong());
                            ui.add_space(4.0);
                            let mut fos = self.settings.format_on_save;
                            if ui
                                .checkbox(&mut fos, "Format file before saving")
                                .changed()
                            {
                                self.settings.format_on_save = fos;
                                let _ = self.settings.save();
                            }
                            ui.add_space(12.0);

                            // Advanced theme tools
                            ui.label(egui::RichText::new("Advanced").strong());
                            ui.add_space(4.0);
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
                            ui.heading(self.tr("GitHub Review"));
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
                            ui.heading(self.tr("Other Plugins"));
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
                        super::types::SettingsTab::Panels => {
                            self.render_panel_visibility_settings(ui);
                        }
                        #[cfg(feature = "ai")]
                        super::types::SettingsTab::AiProviders => {
                            self.render_ai_providers_settings(ui);
                        }
                        #[cfg(feature = "ai")]
                        super::types::SettingsTab::AiUsage => {
                            self.render_ai_usage_settings(ui);
                        }
                    });
            });
        });
    }

    /// Activity Bar visibility tab. One checkbox per togglable panel.
    /// Database, Docker, and OracleBerry default to hidden because
    /// they're niche/specialised; the rest default to visible. Edits
    /// persist immediately to `~/.berrycode/panels.json`.
    pub(crate) fn render_panel_visibility_settings(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Activity Bar")
                .size(16.0)
                .color(ui_colors::SETTINGS_HEADER())
                .strong(),
        );
        ui.label(
            egui::RichText::new("Show or hide panels in the left activity bar.")
                .color(ui_colors::SETTINGS_DESC()),
        );
        ui.add_space(12.0);

        let mut v = self.panel_visibility;
        let mut changed = false;

        let row = |ui: &mut egui::Ui,
                   label: &str,
                   state: &mut bool,
                   changed: &mut bool,
                   hint: Option<&str>| {
            let resp = ui.checkbox(state, label);
            if resp.changed() {
                *changed = true;
            }
            if let Some(h) = hint {
                ui.label(
                    egui::RichText::new(format!("    {h}"))
                        .small()
                        .color(ui_colors::SETTINGS_HINT()),
                );
            }
        };

        row(ui, "Explorer", &mut v.explorer, &mut changed, None);
        row(ui, "Search", &mut v.search, &mut changed, None);
        row(ui, "Git", &mut v.git, &mut changed, None);
        row(ui, "Terminal", &mut v.terminal, &mut changed, None);
        row(
            ui,
            "ECS Inspector",
            &mut v.ecs_inspector,
            &mut changed,
            None,
        );
        row(ui, "Scene Editor", &mut v.scene_editor, &mut changed, None);
        row(
            ui,
            "Database",
            &mut v.database,
            &mut changed,
            Some("Off by default."),
        );
        row(
            ui,
            "Docker",
            &mut v.docker,
            &mut changed,
            Some("Off by default."),
        );
        #[cfg(feature = "ai")]
        row(
            ui,
            "OracleBerry",
            &mut v.oracleberry,
            &mut changed,
            Some("Off by default."),
        );

        if changed {
            self.panel_visibility = v;
            super::save_panel_visibility(v);
            // If the user just hid the panel they're currently viewing,
            // fall back to Explorer (or Settings if Explorer is also off)
            // so the central area doesn't go blank.
            if !v.is_visible(self.active_panel) {
                self.active_panel = if v.explorer {
                    super::types::ActivePanel::Explorer
                } else {
                    super::types::ActivePanel::Settings
                };
            }
        }
    }

    #[cfg(feature = "ai")]
    /// AI Providers settings tab — BYOK configuration. Lets the user
    /// paste API keys for Anthropic / OpenAI, point at a local Ollama
    /// instance, and pick which model handles chat vs inline completion.
    /// All edits persist immediately to `~/.berrycode/ai.json`.
    pub(crate) fn render_ai_providers_settings(&mut self, ui: &mut egui::Ui) {
        use crate::ai::settings::AiSettings;
        use crate::ai::ProviderKind;
        use crate::app::ui_colors;

        ui.label(
            egui::RichText::new("AI Providers")
                .size(16.0)
                .color(ui_colors::SETTINGS_HEADER())
                .strong(),
        );
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Bring your own key — BerryCode talks to each provider directly.")
                .small()
                .color(ui_colors::SETTINGS_DESC()),
        );
        ui.add_space(16.0);

        let mut dirty = false;

        // ── Master enable toggle ─────────────────────────────────
        setting_card(
            ui,
            "Enable AI assistant",
            Some(
                "When off, all AI features (chat, inline completion) are disabled even if keys are set.",
            ),
            |ui| {
                if ui
                    .checkbox(&mut self.ai_settings.enabled, "Enabled")
                    .changed()
                {
                    dirty = true;
                }
            },
        );

        // ── Anthropic ────────────────────────────────────────────
        let anthropic_env = std::env::var("ANTHROPIC_API_KEY")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        setting_card(
            ui,
            "Anthropic (Claude)",
            Some("API key from https://console.anthropic.com/settings/keys."),
            |ui| {
                if anthropic_env {
                    // Env always wins at runtime; show that and lock the
                    // field so users don't think they need to paste a key.
                    ui.label(
                        egui::RichText::new("Using ANTHROPIC_API_KEY from environment")
                            .color(egui::Color32::from_rgb(120, 200, 140)),
                    );
                } else {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.anthropic_api_key)
                            .password(true)
                            .desired_width(420.0)
                            .hint_text("sk-ant-…"),
                    );
                    if resp.changed() {
                        dirty = true;
                    }
                }
            },
        );

        // ── OpenAI ───────────────────────────────────────────────
        let openai_env = std::env::var("OPENAI_API_KEY")
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let openai_base = std::env::var("OPENAI_BASE_URL").unwrap_or_default();
        let openai_subtitle = if openai_base.is_empty() {
            "API key from https://platform.openai.com/api-keys.".to_string()
        } else {
            format!("Endpoint override: {}", openai_base)
        };
        setting_card(
            ui,
            "OpenAI (GPT / Codex)",
            Some(openai_subtitle.as_str()),
            |ui| {
                if openai_env {
                    ui.label(
                        egui::RichText::new("Using OPENAI_API_KEY from environment")
                            .color(egui::Color32::from_rgb(120, 200, 140)),
                    );
                } else {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.openai_api_key)
                            .password(true)
                            .desired_width(420.0)
                            .hint_text("sk-…"),
                    );
                    if resp.changed() {
                        dirty = true;
                    }
                }
            },
        );

        // ── Ollama (local) ───────────────────────────────────────
        setting_card(
            ui,
            "Ollama (local)",
            Some("Self-hosted server. No API key required — `ollama serve` and pick a model."),
            |ui| {
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.ollama_endpoint)
                            .desired_width(320.0)
                            .hint_text("http://localhost:11434"),
                    );
                    if resp.changed() {
                        dirty = true;
                    }
                    if ui.button("Test connection").clicked() {
                        let endpoint = self.ai_settings.ollama_endpoint.clone();
                        let status = self.ollama_status.clone();
                        if let Ok(mut s) = status.lock() {
                            *s = crate::app::OllamaProbeStatus::Probing;
                        }
                        self.lsp_runtime.spawn(async move {
                            let result = crate::ai::ollama::probe_version(&endpoint).await;
                            if let Ok(mut s) = status.lock() {
                                *s = match result {
                                    Some(v) => crate::app::OllamaProbeStatus::Connected(v),
                                    None => crate::app::OllamaProbeStatus::Error(
                                        "server not reachable (is `ollama serve` running?)"
                                            .to_string(),
                                    ),
                                };
                            }
                        });
                    }
                    if ui.button("Refresh models").clicked() {
                        let endpoint = self.ai_settings.ollama_endpoint.clone();
                        let cache = self.ollama_installed_models.clone();
                        self.lsp_runtime.spawn(async move {
                            if let Some(list) =
                                crate::ai::ollama::list_installed_models(&endpoint).await
                            {
                                if let Ok(mut c) = cache.lock() {
                                    *c = list;
                                }
                            }
                        });
                    }
                });
                // Status line — surfaced under the row so the buttons
                // don't shift around as the probe completes.
                if let Ok(s) = self.ollama_status.lock() {
                    match &*s {
                        crate::app::OllamaProbeStatus::Unknown => {}
                        crate::app::OllamaProbeStatus::Probing => {
                            ui.label(
                                egui::RichText::new("Probing…")
                                    .small()
                                    .color(egui::Color32::from_rgb(160, 160, 170)),
                            );
                        }
                        crate::app::OllamaProbeStatus::Connected(v) => {
                            ui.label(
                                egui::RichText::new(format!("✓ Connected (v{})", v))
                                    .small()
                                    .color(egui::Color32::from_rgb(120, 200, 140)),
                            );
                        }
                        crate::app::OllamaProbeStatus::Error(e) => {
                            ui.label(
                                egui::RichText::new(format!("✗ {}", e))
                                    .small()
                                    .color(egui::Color32::from_rgb(220, 130, 130)),
                            );
                        }
                    }
                }
                if let Ok(models) = self.ollama_installed_models.lock() {
                    if !models.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("Installed: {}", models.join(", ")))
                                .small()
                                .color(egui::Color32::from_rgb(140, 180, 220)),
                        );
                    }
                }

                // ── One-click "go local with Llama 3" ─────────────
                // Sets up the whole stack (provider, model, agent
                // backend, endpoint) for fully-local Llama inference.
                // Pairs with the Llama-tuned cheatsheet auto-applied
                // by `ai_chat.rs::is_llama_family_model`.
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button("Use Llama 3 (local)")
                        .on_hover_text(
                            "Sets chat provider to Ollama, model to llama3.3, and agent \
                             backend to Ollama. Run `ollama pull llama3.3` first if you \
                             haven't downloaded it yet.",
                        )
                        .clicked()
                    {
                        self.ai_settings.chat_provider = ProviderKind::Ollama;
                        self.ai_settings.chat_model = "llama3.3".to_string();
                        self.ai_settings.completion_provider = ProviderKind::Ollama;
                        self.ai_settings.completion_model = "llama3.3".to_string();
                        self.ai_settings.agent_backend = "ollama".to_string();
                        if self.ai_settings.ollama_endpoint.trim().is_empty() {
                            self.ai_settings.ollama_endpoint = "http://localhost:11434".to_string();
                        }
                        dirty = true;
                    }
                    ui.label(
                        egui::RichText::new(
                            "Local-first AI (no API keys, no data leaves your machine).",
                        )
                        .small()
                        .color(egui::Color32::from_rgb(140, 180, 220)),
                    );
                });
            },
        );

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Model selection")
                .size(13.0)
                .color(ui_colors::SETTINGS_HEADER())
                .strong(),
        );
        ui.add_space(8.0);

        let provider_options = [
            ProviderKind::Anthropic,
            ProviderKind::OpenAi,
            ProviderKind::Ollama,
        ];

        // ── Chat model ───────────────────────────────────────────
        setting_card(
            ui,
            "Chat sidebar",
            Some("Provider and model used when you talk to the assistant in the right-hand chat panel."),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Provider:");
                    for kind in provider_options {
                        if ui
                            .selectable_label(
                                self.ai_settings.chat_provider == kind,
                                kind.label(),
                            )
                            .clicked()
                        {
                            self.ai_settings.chat_provider = kind;
                            let models = AiSettings::chat_models_for(kind);
                            if let Some(first) = models.first() {
                                self.ai_settings.chat_model = first.to_string();
                            }
                            dirty = true;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    // Free-text input: Azure OpenAI deployments and other
                    // proxy setups use arbitrary names that won't appear
                    // in the preset list. The dropdown next to it stays
                    // as a quick-pick of the well-known public models.
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.chat_model)
                            .desired_width(220.0)
                            .hint_text("model / deployment"),
                    );
                    if resp.changed() {
                        dirty = true;
                    }
                    let presets = AiSettings::chat_models_for(self.ai_settings.chat_provider);
                    // Snapshot the dynamic model list under the lock,
                    // then drop the lock before showing the popup —
                    // egui closures can re-render and we don't want a
                    // long-held mutex blocking the async refresh task.
                    let dynamic_models: Vec<String> =
                        if self.ai_settings.chat_provider == ProviderKind::Ollama {
                            self.ollama_installed_models
                                .lock()
                                .map(|m| m.clone())
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                    egui::ComboBox::from_id_salt("ai_chat_model")
                        .selected_text("Presets")
                        .show_ui(ui, |ui| {
                            for model in presets {
                                if ui
                                    .selectable_label(
                                        self.ai_settings.chat_model == *model,
                                        *model,
                                    )
                                    .clicked()
                                {
                                    self.ai_settings.chat_model = model.to_string();
                                    dirty = true;
                                }
                            }
                            if !dynamic_models.is_empty() {
                                ui.separator();
                                ui.label(
                                    egui::RichText::new("Installed locally")
                                        .small()
                                        .color(egui::Color32::from_rgb(140, 180, 220)),
                                );
                                for model in &dynamic_models {
                                    if ui
                                        .selectable_label(
                                            &self.ai_settings.chat_model == model,
                                            model,
                                        )
                                        .clicked()
                                    {
                                        self.ai_settings.chat_model = model.clone();
                                        dirty = true;
                                    }
                                }
                            }
                        });
                });
            },
        );

        // ── Inline completion model ──────────────────────────────
        setting_card(
            ui,
            "Inline / Tab completion",
            Some("Lower-latency model used for ghost-text suggestions. Smaller / faster models recommended."),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Provider:");
                    for kind in provider_options {
                        if ui
                            .selectable_label(
                                self.ai_settings.completion_provider == kind,
                                kind.label(),
                            )
                            .clicked()
                        {
                            self.ai_settings.completion_provider = kind;
                            let models = AiSettings::completion_models_for(kind);
                            if let Some(first) = models.first() {
                                self.ai_settings.completion_model = first.to_string();
                            }
                            dirty = true;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Model:");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.ai_settings.completion_model)
                            .desired_width(220.0)
                            .hint_text("model / deployment"),
                    );
                    if resp.changed() {
                        dirty = true;
                    }
                    let models =
                        AiSettings::completion_models_for(self.ai_settings.completion_provider);
                    egui::ComboBox::from_id_salt("ai_completion_model")
                        .selected_text("Presets")
                        .show_ui(ui, |ui| {
                            for model in models {
                                if ui
                                    .selectable_label(
                                        self.ai_settings.completion_model == *model,
                                        *model,
                                    )
                                    .clicked()
                                {
                                    self.ai_settings.completion_model = model.to_string();
                                    dirty = true;
                                }
                            }
                        });
                });
            },
        );

        // ── Coding agent backend ─────────────────────────────────
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("Coding agent")
                .size(13.0)
                .color(ui_colors::SETTINGS_HEADER())
                .strong(),
        );
        ui.add_space(8.0);

        setting_card(
            ui,
            "Agent backend",
            Some(
                "Which engine drives Autonomous (🤖 Agent) mode. Native runs in-process via the OpenAI Responses API. \
                 Claude Code / Codex spawn the official CLI as a subprocess. Ollama runs against a local server — \
                 use a tool-capable model (llama3.1, qwen2.5-coder, mistral)."
            ),
            |ui| {
                let backends: &[(&str, &str)] = &[
                    ("native", "Native (in-process)"),
                    ("claude", "Claude Code"),
                    ("codex", "Codex"),
                    ("ollama", "Ollama (local)"),
                ];
                ui.horizontal(|ui| {
                    for (id, label) in backends {
                        if ui
                            .selectable_label(
                                self.ai_settings.agent_backend == *id,
                                *label,
                            )
                            .clicked()
                        {
                            self.ai_settings.agent_backend = id.to_string();
                            dirty = true;
                        }
                    }
                });
            },
        );

        if dirty {
            self.ai_settings.save();
        }

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Settings are persisted to ~/.berrycode/ai.json. Keys are stored in plaintext for now — \
                 a future revision will move them to the OS keyring.",
            )
            .small()
            .color(egui::Color32::from_rgb(140, 140, 150)),
        );
    }

    /// Color Scheme Settings (RustRover Darcula)
    pub(crate) fn render_color_scheme_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.tr("Color Scheme: Darcula (Customized)"));
        ui.label(self.tr("Customize syntax highlighting colors:"));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.keyword_color);
            ui.label(self.tr("Keyword (fn, let, match)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.function_color);
            ui.label(self.tr("Function / Macro"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.type_color);
            ui.label(self.tr("Type (struct, enum)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.string_color);
            ui.label(self.tr("String"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.number_color);
            ui.label(self.tr("Number"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.comment_color);
            ui.label(self.tr("Comment"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.macro_color);
            ui.label(self.tr("Macro (println!)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.attribute_color);
            ui.label(self.tr("Attribute (#[derive])"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.constant_color);
            ui.label(self.tr("Constant (STATIC)"));
        });
        ui.horizontal(|ui| {
            ui.color_edit_button_srgba(&mut self.lifetime_color);
            ui.label(self.tr("Lifetime ('a, 'static)"));
        });

        ui.add_space(20.0);
        ui.separator();
        ui.label(egui::RichText::new(self.tr("Live Preview:")).strong());
        ui.add_space(8.0);
        self.render_color_preview(ui);

        ui.add_space(16.0);
        if ui
            .button(format!("🔄 {}", self.tr("Reset to Darcula Defaults")))
            .clicked()
        {
            self.reset_colors_to_darcula();
        }
    }

    /// Live preview of syntax colors
    pub(crate) fn render_color_preview(&self, ui: &mut egui::Ui) {
        let frame = egui::Frame::NONE
            .fill(ui_colors::SIDEBAR_BG())
            .inner_margin(12)
            .corner_radius(4);

        let font = egui::FontId::monospace(13.0);
        let def = egui::Color32::from_rgb(212, 212, 212);

        // Build a LayoutJob for pixel-perfect monospace rendering
        let mut job = egui::text::LayoutJob::default();
        let f = |color: egui::Color32| egui::TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        };

        // fn main() {
        job.append("fn", 0.0, f(self.keyword_color));
        job.append(" main", 0.0, f(self.function_color));
        job.append("() {\n", 0.0, f(def));
        //     let x: u32 = 42;
        job.append("    ", 0.0, f(def));
        job.append("let", 0.0, f(self.keyword_color));
        job.append(" x: ", 0.0, f(def));
        job.append("u32", 0.0, f(self.type_color));
        job.append(" = ", 0.0, f(def));
        job.append("42", 0.0, f(self.number_color));
        job.append(";\n", 0.0, f(def));
        //     // Hello World
        job.append("    ", 0.0, f(def));
        job.append("// Hello World", 0.0, f(self.comment_color));
        job.append("\n", 0.0, f(def));
        //     println!("Ready!");
        job.append("    ", 0.0, f(def));
        job.append("println!", 0.0, f(self.macro_color));
        job.append("(", 0.0, f(def));
        job.append("\"Ready!\"", 0.0, f(self.string_color));
        job.append(");\n", 0.0, f(def));
        //     const MAX: usize = 100;
        job.append("    ", 0.0, f(def));
        job.append("const", 0.0, f(self.keyword_color));
        job.append(" ", 0.0, f(def));
        job.append("MAX", 0.0, f(self.constant_color));
        job.append(": ", 0.0, f(def));
        job.append("usize", 0.0, f(self.type_color));
        job.append(" = ", 0.0, f(def));
        job.append("100", 0.0, f(self.number_color));
        job.append(";\n", 0.0, f(def));
        // }
        job.append("}", 0.0, f(def));

        frame.show(ui, |ui| {
            ui.add(egui::Label::new(job));
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

    /// AI Usage & Cost settings tab — reads `~/.berrycode/ai_usage.json`
    /// and shows today / month rollups plus an estimated USD figure
    /// based on `crate::ai::usage::estimate_cost`. The numbers are
    /// best-effort: the authoritative bill is always whatever the
    /// provider charges, so this tab is for self-monitoring, not
    /// reconciliation.
    #[cfg(feature = "ai")]
    pub(crate) fn render_ai_usage_settings(&mut self, ui: &mut egui::Ui) {
        let records = crate::ai::usage::load();

        ui.heading(self.tr("AI Usage & Cost"));
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Token counts and estimated cost based on the model list \
                 prices baked into BerryCode. Local models (Ollama / \
                 llama.cpp) report tokens but are billed at $0.",
            )
            .color(egui::Color32::from_rgb(140, 145, 160)),
        );
        ui.add_space(12.0);

        // Window boundaries for "today" (from local-midnight UTC) and
        // "this month" (from the 1st at 00:00 UTC). A single timezone
        // for both rollups keeps the math simple; if the user is on the
        // last day of the month at 23:00 local their "today" can briefly
        // exceed "this month" — that's a known acceptable wart.
        let now = chrono::Utc::now();
        let today_start = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc))
            .unwrap_or(now);
        let month_start = now
            .date_naive()
            .with_day(1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|n| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(n, chrono::Utc))
            .unwrap_or(now);
        let far_future = now + chrono::Duration::days(1);

        let today = crate::ai::usage::totals_between(&records, today_start, far_future);
        let month = crate::ai::usage::totals_between(&records, month_start, far_future);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width(220.0);
                Self::render_usage_card(ui, "Today", &today);
            });
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.set_min_width(220.0);
                Self::render_usage_card(ui, "This month", &month);
            });
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Soft cap. Just a number stored alongside the API keys; we
        // surface it here for visibility but don't actively block
        // requests yet — that needs a UX call (toast? hard stop?) we'll
        // settle in v0.4.6.
        ui.horizontal(|ui| {
            ui.label("Monthly soft cap (USD):");
            ui.add(
                egui::DragValue::new(&mut self.ai_settings.monthly_cap_usd)
                    .speed(1.0)
                    .range(0.0..=10_000.0)
                    .prefix("$"),
            );
            if month.cost_usd > self.ai_settings.monthly_cap_usd
                && self.ai_settings.monthly_cap_usd > 0.0
            {
                ui.label(
                    egui::RichText::new("over cap")
                        .color(egui::Color32::from_rgb(220, 120, 120))
                        .strong(),
                );
            }
        });
        ui.label(
            egui::RichText::new(
                "Cap is informational for now — BerryCode warns you here \
                 but doesn't block requests automatically.",
            )
            .size(11.0)
            .color(egui::Color32::from_rgb(140, 145, 160)),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);
        ui.collapsing("Recent requests", |ui| {
            if records.is_empty() {
                ui.label("No requests recorded yet. Send a chat message to populate this list.");
                return;
            }
            egui::Grid::new("ai_usage_recent")
                .striped(true)
                .num_columns(5)
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Time").strong());
                    ui.label(egui::RichText::new("Model").strong());
                    ui.label(egui::RichText::new("In").strong());
                    ui.label(egui::RichText::new("Out").strong());
                    ui.label(egui::RichText::new("USD").strong());
                    ui.end_row();
                    // Show the last 30 records, newest first.
                    for r in records.iter().rev().take(30) {
                        ui.label(r.timestamp.split('T').next().unwrap_or(&r.timestamp));
                        ui.label(&r.model);
                        ui.label(format!("{}", r.prompt_tokens));
                        ui.label(format!("{}", r.completion_tokens));
                        ui.label(format!("${:.4}", r.cost_usd));
                        ui.end_row();
                    }
                });
        });
    }

    /// One stat card used in the AI Usage tab. Mirrors the layout of the
    /// Today / Month columns so they read at a glance.
    #[cfg(feature = "ai")]
    fn render_usage_card(ui: &mut egui::Ui, title: &str, t: &crate::ai::usage::UsageTotals) {
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(34, 36, 42))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 64, 72)))
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .size(13.0)
                        .color(egui::Color32::from_rgb(200, 205, 220)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("${:.2}", t.cost_usd))
                        .size(18.0)
                        .color(egui::Color32::WHITE),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("{} requests", t.requests))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(140, 145, 160)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} in / {} out tokens",
                        t.prompt_tokens, t.completion_tokens
                    ))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(140, 145, 160)),
                );
                if t.cache_read_tokens > 0 || t.cache_write_tokens > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "cache: {} read / {} write",
                            t.cache_read_tokens, t.cache_write_tokens
                        ))
                        .size(11.0)
                        .color(egui::Color32::from_rgb(120, 160, 140)),
                    );
                }
            });
    }

    /// Keyboard Shortcuts settings tab
    pub(crate) fn render_keybindings_settings(&mut self, ui: &mut egui::Ui) {
        use super::keymap::{KeyAction, KeyBinding};

        ui.heading(self.tr("Keybindings"));
        ui.add_space(4.0);
        ui.label("Click a shortcut to rebind it. Press Esc to cancel.");
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Capture the next key press while a recording is active. We do this
        // before rendering the grid so the new binding is reflected
        // immediately in the same frame.
        if let Some(target) = self.keybinding_recording {
            let captured = ui.input(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    return Some(None); // cancel
                }
                for ev in &i.events {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = ev
                    {
                        // Ignore modifier-only key events (Cmd / Shift / Alt
                        // by themselves shouldn't count as a binding).
                        let is_modifier = matches!(key, egui::Key::Backtick | egui::Key::Escape);
                        if is_modifier {
                            continue;
                        }
                        let new_binding = KeyBinding {
                            command: modifiers.command,
                            shift: modifiers.shift,
                            alt: modifiers.alt,
                            key: format!("{:?}", key),
                        };
                        return Some(Some(new_binding));
                    }
                }
                None
            });

            match captured {
                Some(None) => {
                    self.keybinding_recording = None;
                    self.keybinding_message =
                        Some(format!("Cancelled (no change to {})", target.label()));
                }
                Some(Some(new_binding)) => {
                    // Detect conflict — another action already uses this chord.
                    let conflict = self
                        .keymap
                        .bindings
                        .iter()
                        .find(|(act, b)| {
                            **act != target
                                && b.command == new_binding.command
                                && b.shift == new_binding.shift
                                && b.alt == new_binding.alt
                                && b.key == new_binding.key
                        })
                        .map(|(a, _)| *a);

                    if let Some(conflict_action) = conflict {
                        self.keybinding_message = Some(format!(
                            "{} already bound to {}",
                            new_binding.display(),
                            conflict_action.label()
                        ));
                    } else {
                        let display = new_binding.display();
                        self.keymap.bindings.insert(target, new_binding);
                        self.keymap.save();
                        self.keybinding_message = Some(format!("{} → {}", target.label(), display));
                    }
                    self.keybinding_recording = None;
                }
                None => {}
            }
        }

        egui::Grid::new("keybindings_grid")
            .num_columns(3)
            .spacing([16.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Action");
                ui.strong("Shortcut");
                ui.strong("");
                ui.end_row();

                for action in KeyAction::ALL {
                    ui.label(action.label());

                    let is_recording = self.keybinding_recording == Some(*action);
                    let label = if is_recording {
                        "Press a key…".to_string()
                    } else {
                        self.keymap
                            .bindings
                            .get(action)
                            .map(|b| b.display())
                            .unwrap_or_else(|| "(unbound)".to_string())
                    };

                    let button = egui::Button::new(egui::RichText::new(&label).monospace())
                        .min_size(egui::vec2(140.0, 22.0))
                        .fill(if is_recording {
                            egui::Color32::from_rgb(60, 90, 140)
                        } else {
                            egui::Color32::TRANSPARENT
                        });

                    if ui.add(button).clicked() {
                        self.keybinding_recording = Some(*action);
                        self.keybinding_message = None;
                    }

                    if !is_recording && self.keymap.bindings.contains_key(action) {
                        if ui.small_button("Clear").clicked() {
                            self.keymap.bindings.remove(action);
                            self.keymap.save();
                            self.keybinding_message = Some(format!("Cleared {}", action.label()));
                        }
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });

        ui.add_space(8.0);
        if let Some(msg) = &self.keybinding_message {
            ui.label(
                egui::RichText::new(msg)
                    .small()
                    .color(egui::Color32::from_rgb(180, 200, 230)),
            );
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Reset to Defaults").clicked() {
                self.keymap = super::keymap::Keymap::default();
                self.keymap.save();
                self.keybinding_recording = None;
                self.keybinding_message = Some("Reset to defaults".to_string());
                tracing::info!("Keyboard shortcuts reset to defaults");
            }

            if ui.button("Save to File").clicked() {
                self.keymap.save();
                self.keybinding_message = Some("Saved".to_string());
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

// ──────────────────────────────────────────────────────────────────────
// VS Code-style settings UI helpers (free functions, used from inside
// `render_settings_panel` and the per-tab renderers below).
// ──────────────────────────────────────────────────────────────────────

/// Tiny dimmed section label, used as a visual divider between groups
/// of nav entries (e.g. "Application", "Editor", "Features").
fn nav_section_header(ui: &mut egui::Ui, text: &str) {
    use crate::app::ui_colors;
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .small()
            .color(ui_colors::SETTINGS_HINT())
            .strong(),
    );
    ui.add_space(2.0);
}

/// One row in the left navigation column. Looks like a VS Code tree
/// item: full-row click target, subtle hover, accent bar on the
/// selected entry.
fn nav_item(
    ui: &mut egui::Ui,
    current: &mut crate::app::types::SettingsTab,
    target: crate::app::types::SettingsTab,
    label: impl Into<String>,
) {
    use crate::app::ui_colors;
    let label = label.into();
    let is_selected = *current == target;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::click());
    let bg = if is_selected {
        egui::Color32::from_rgb(50, 56, 70)
    } else if response.hovered() {
        egui::Color32::from_rgb(40, 42, 47)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(3), bg);
    if is_selected {
        // Accent bar on the left edge.
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                rect.left_top(),
                egui::pos2(rect.left() + 2.0, rect.bottom()),
            ),
            egui::CornerRadius::ZERO,
            ui_colors::SETTINGS_ACCENT(),
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if is_selected {
            ui_colors::SETTINGS_HEADER()
        } else {
            ui_colors::TEXT_DEFAULT()
        },
    );
    if response.clicked() {
        *current = target;
    }
}

/// VS Code-style settings card: a title (bold), an optional dim
/// description, and a control rendered by `body`. Each card is wrapped
/// in a subtle bordered frame so individual settings are visually
/// separated.
fn setting_card(
    ui: &mut egui::Ui,
    title: &str,
    description: Option<&str>,
    body: impl FnOnce(&mut egui::Ui),
) {
    use crate::app::ui_colors;
    egui::Frame::NONE
        .stroke(egui::Stroke::new(1.0, ui_colors::SETTINGS_CARD_BORDER()))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .fill(ui_colors::SETTINGS_NAV_BG())
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(720.0));
            ui.label(
                egui::RichText::new(title)
                    .strong()
                    .color(ui_colors::SETTINGS_HEADER()),
            );
            if let Some(desc) = description {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(desc)
                        .small()
                        .color(ui_colors::SETTINGS_DESC()),
                );
            }
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(8.0);
}
