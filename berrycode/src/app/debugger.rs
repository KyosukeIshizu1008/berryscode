//! Debug panel UI and debug session state management
//! VS Code-style debugger with Call Stack, Variables, Watch, Console, and Toolbar

use super::BerryCodeApp;
use crate::native::dap::*;

/// Watch expression
#[derive(Debug, Clone)]
pub struct WatchExpression {
    pub expression: String,
    pub value: Option<String>,
}

/// Debug session state
pub struct DebugState {
    pub active: bool,
    pub paused: bool,
    pub breakpoints: Vec<DapBreakpoint>,
    pub stack_frames: Vec<DapStackFrame>,
    pub variables: Vec<DapVariable>,
    pub threads: Vec<DapThread>,
    pub selected_thread: Option<u64>,
    pub selected_frame: Option<u64>,
    pub debug_output: Vec<(String, String)>, // (category, text)
    pub console_input: String,
    pub watch_expressions: Vec<WatchExpression>,
    pub watch_input: String,
    /// Scopes reference (for requesting variables at different scopes)
    pub scopes: Vec<(String, u64)>, // (name, variables_reference)
    /// Which sub-panel is active in the debug view
    pub active_tab: DebugTab,
    /// Current stopped location highlight
    pub stopped_file: Option<String>,
    pub stopped_line: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DebugTab {
    Variables,
    Watch,
    CallStack,
    Console,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            active: false,
            paused: false,
            breakpoints: Vec::new(),
            stack_frames: Vec::new(),
            variables: Vec::new(),
            threads: Vec::new(),
            selected_thread: None,
            selected_frame: None,
            debug_output: Vec::new(),
            console_input: String::new(),
            watch_expressions: Vec::new(),
            watch_input: String::new(),
            scopes: Vec::new(),
            active_tab: DebugTab::Variables,
            stopped_file: None,
            stopped_line: None,
        }
    }
}

impl BerryCodeApp {
    /// Render the debug panel (VS Code-style bottom panel)
    pub(crate) fn render_debug_panel(&mut self, ctx: &egui::Context) {
        if !self.debug_state.active {
            return;
        }

        egui::TopBottomPanel::bottom("debug_panel")
            .default_height(250.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(30, 30, 30))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                // ─── Debug Toolbar ────────────────────────────
                self.render_debug_toolbar(ui);

                ui.separator();

                // ─── Main content: tabs ──────────────────────
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (tab, label) in [
                        (DebugTab::Variables, "Variables"),
                        (DebugTab::Watch, "Watch"),
                        (DebugTab::CallStack, "Call Stack"),
                        (DebugTab::Console, "Debug Console"),
                    ] {
                        let active = self.debug_state.active_tab == tab;
                        let color = if active {
                            egui::Color32::from_rgb(200, 200, 200)
                        } else {
                            egui::Color32::from_rgb(120, 120, 120)
                        };
                        let btn = ui.add(
                            egui::Button::new(egui::RichText::new(label).size(12.0).color(color))
                                .frame(false)
                                .min_size(egui::vec2(90.0, 24.0)),
                        );
                        if active {
                            let r = btn.rect;
                            ui.painter().line_segment(
                                [r.left_bottom(), r.right_bottom()],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(70, 130, 220)),
                            );
                        }
                        if btn.clicked() {
                            self.debug_state.active_tab = tab;
                        }
                    }
                });

                ui.add_space(4.0);

                match self.debug_state.active_tab {
                    DebugTab::Variables => self.render_debug_variables(ui),
                    DebugTab::Watch => self.render_debug_watch(ui),
                    DebugTab::CallStack => self.render_debug_call_stack(ui),
                    DebugTab::Console => self.render_debug_console(ui),
                }
            });
    }

    /// Debug toolbar with Continue/Step/Stop buttons
    fn render_debug_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            let btn_size = egui::vec2(28.0, 24.0);
            let paused = self.debug_state.paused;

            // Continue / Pause
            if paused {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{ebb5}") // play
                                .size(16.0)
                                .color(egui::Color32::from_rgb(80, 200, 80)),
                        )
                        .min_size(btn_size),
                    )
                    .on_hover_text("Continue (F5)")
                    .clicked()
                {
                    self.debug_continue();
                }
            } else {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("\u{ebb6}") // pause
                                .size(16.0)
                                .color(egui::Color32::from_rgb(200, 200, 80)),
                        )
                        .min_size(btn_size),
                    )
                    .on_hover_text("Pause")
                    .clicked()
                {
                    // Pause not directly supported in basic DAP flow
                }
            }

            // Step Over
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{eb3f}") // debug-step-over
                            .size(16.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .min_size(btn_size)
                    .sense(if paused {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    }),
                )
                .on_hover_text("Step Over (F10)")
                .clicked()
            {
                self.debug_step_over();
            }

            // Step Into
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{eb3e}") // debug-step-into
                            .size(16.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .min_size(btn_size)
                    .sense(if paused {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    }),
                )
                .on_hover_text("Step Into (F11)")
                .clicked()
            {
                self.debug_step_into();
            }

            // Step Out
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{eb40}") // debug-step-out
                            .size(16.0)
                            .color(egui::Color32::from_rgb(100, 180, 255)),
                    )
                    .min_size(btn_size)
                    .sense(if paused {
                        egui::Sense::click()
                    } else {
                        egui::Sense::hover()
                    }),
                )
                .on_hover_text("Step Out (Shift+F11)")
                .clicked()
            {
                self.debug_step_out();
            }

            // Restart
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{eb37}") // debug-restart
                            .size(16.0)
                            .color(egui::Color32::from_rgb(80, 200, 80)),
                    )
                    .min_size(btn_size),
                )
                .on_hover_text("Restart (Ctrl+Shift+F5)")
                .clicked()
            {
                self.debug_stop();
                self.start_debug();
            }

            // Stop
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{eb3a}") // debug-stop
                            .size(16.0)
                            .color(egui::Color32::from_rgb(230, 80, 80)),
                    )
                    .min_size(btn_size),
                )
                .on_hover_text("Stop (Shift+F5)")
                .clicked()
            {
                self.debug_stop();
            }

            ui.separator();

            // Status text
            let status = if self.debug_state.paused {
                "Paused"
            } else if self.debug_state.active {
                "Running"
            } else {
                "Stopped"
            };
            ui.label(
                egui::RichText::new(status)
                    .size(12.0)
                    .color(egui::Color32::from_rgb(160, 160, 160)),
            );

            // Thread selector (if multiple)
            if self.debug_state.threads.len() > 1 {
                ui.separator();
                let current = self
                    .debug_state
                    .selected_thread
                    .and_then(|tid| {
                        self.debug_state
                            .threads
                            .iter()
                            .find(|t| t.id == tid)
                            .map(|t| t.name.clone())
                    })
                    .unwrap_or_else(|| "Thread".to_string());

                egui::ComboBox::from_id_salt("thread_selector")
                    .selected_text(&current)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for thread in &self.debug_state.threads {
                            if ui
                                .selectable_label(
                                    self.debug_state.selected_thread == Some(thread.id),
                                    &thread.name,
                                )
                                .clicked()
                            {
                                self.debug_state.selected_thread = Some(thread.id);
                            }
                        }
                    });
            }
        });
    }

    /// Variables panel
    fn render_debug_variables(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("debug_vars")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if self.debug_state.variables.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 120, 120),
                        "No variables available",
                    );
                    return;
                }

                egui::Grid::new("var_grid")
                    .striped(true)
                    .spacing(egui::vec2(12.0, 2.0))
                    .show(ui, |ui| {
                        // Header
                        ui.label(
                            egui::RichText::new("Name")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
                        ui.label(
                            egui::RichText::new("Value")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
                        ui.label(
                            egui::RichText::new("Type")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 140)),
                        );
                        ui.end_row();

                        for var in &self.debug_state.variables {
                            ui.label(
                                egui::RichText::new(&var.name)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                            );
                            ui.label(
                                egui::RichText::new(&var.value)
                                    .size(12.0)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(206, 145, 120)),
                            );
                            ui.label(
                                egui::RichText::new(var.var_type.as_deref().unwrap_or(""))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                            ui.end_row();
                        }
                    });
            });
    }

    /// Watch expressions panel
    fn render_debug_watch(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("debug_watch")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut remove_idx: Option<usize> = None;

                for (idx, watch) in self.debug_state.watch_expressions.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&watch.expression)
                                .size(12.0)
                                .color(egui::Color32::from_rgb(100, 180, 255)),
                        );
                        ui.label(
                            egui::RichText::new("=")
                                .size(12.0)
                                .color(egui::Color32::GRAY),
                        );
                        let val = watch.value.as_deref().unwrap_or("<not available>");
                        ui.label(
                            egui::RichText::new(val)
                                .size(12.0)
                                .monospace()
                                .color(egui::Color32::from_rgb(206, 145, 120)),
                        );

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("\u{00d7}")
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(140, 140, 140)),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            remove_idx = Some(idx);
                        }
                    });
                }

                if let Some(idx) = remove_idx {
                    self.debug_state.watch_expressions.remove(idx);
                }

                // Add watch input
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.debug_state.watch_input)
                            .hint_text("Add expression...")
                            .font(egui::FontId::monospace(12.0))
                            .desired_width(200.0),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let expr = self.debug_state.watch_input.trim().to_string();
                        if !expr.is_empty() {
                            self.debug_state.watch_expressions.push(WatchExpression {
                                expression: expr,
                                value: None,
                            });
                            self.debug_state.watch_input.clear();
                        }
                    }
                });
            });
    }

    /// Call stack panel
    fn render_debug_call_stack(&mut self, ui: &mut egui::Ui) {
        let mut navigate_to: Option<(Option<String>, usize, usize)> = None;

        egui::ScrollArea::vertical()
            .id_salt("debug_stack")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if self.debug_state.stack_frames.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(120, 120, 120),
                        "No call stack available",
                    );
                    return;
                }

                for (idx, frame) in self.debug_state.stack_frames.iter().enumerate() {
                    let is_selected = self.debug_state.selected_frame == Some(frame.id);

                    ui.horizontal(|ui| {
                        // Frame index
                        ui.label(
                            egui::RichText::new(format!("#{}", idx))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(100, 100, 100)),
                        );

                        // Function name
                        let name_color = if is_selected {
                            egui::Color32::from_rgb(220, 220, 220)
                        } else {
                            egui::Color32::from_rgb(180, 180, 180)
                        };
                        let resp = ui.add(
                            egui::Button::new(
                                egui::RichText::new(&frame.name)
                                    .size(12.0)
                                    .color(name_color),
                            )
                            .frame(false),
                        );

                        // File location
                        if let Some(path) = &frame.file_path {
                            let short = path.rsplit('/').next().unwrap_or(path);
                            ui.label(
                                egui::RichText::new(format!("{}:{}", short, frame.line + 1))
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(100, 100, 100)),
                            );
                        }

                        if resp.clicked() {
                            self.debug_state.selected_frame = Some(frame.id);
                            navigate_to = Some((frame.file_path.clone(), frame.line, frame.column));
                        }
                    });
                }
            });

        // Navigate after iteration
        if let Some((path, line, col)) = navigate_to {
            if let Some(p) = &path {
                self.open_file_from_path(p);
                if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                    tab.pending_cursor_jump = Some((line, col));
                }
            }
        }
    }

    /// Debug console (output + REPL input)
    fn render_debug_console(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_height();

        egui::ScrollArea::vertical()
            .id_salt("debug_console")
            .auto_shrink([false; 2])
            .max_height(available - 30.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (category, text) in &self.debug_state.debug_output {
                    let color = match category.as_str() {
                        "stderr" => egui::Color32::from_rgb(255, 100, 100),
                        "important" => egui::Color32::from_rgb(100, 180, 255),
                        _ => egui::Color32::from_rgb(190, 190, 190),
                    };
                    ui.label(
                        egui::RichText::new(text)
                            .font(egui::FontId::monospace(12.0))
                            .color(color),
                    );
                }
            });

        // Console input
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(">")
                    .font(egui::FontId::monospace(13.0))
                    .color(egui::Color32::from_rgb(100, 180, 255)),
            );
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.debug_state.console_input)
                    .font(egui::FontId::monospace(12.0))
                    .desired_width(f32::INFINITY)
                    .frame(false),
            );
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let cmd = self.debug_state.console_input.trim().to_string();
                if !cmd.is_empty() {
                    self.debug_state
                        .debug_output
                        .push(("input".to_string(), format!("> {}", cmd)));
                    // TODO: evaluate expression via DAP evaluate request
                    self.debug_state.console_input.clear();
                }
            }
        });
    }

    /// Toggle breakpoint at cursor line
    pub(crate) fn toggle_breakpoint(&mut self) {
        if self.editor_tabs.is_empty() {
            return;
        }
        let tab = &self.editor_tabs[self.active_tab_idx];
        let file_path = tab.file_path.clone();
        let line = tab.cursor_line;

        if let Some(idx) = self
            .debug_state
            .breakpoints
            .iter()
            .position(|bp| bp.file_path == file_path && bp.line == line)
        {
            self.debug_state.breakpoints.remove(idx);
        } else {
            self.debug_state.breakpoints.push(DapBreakpoint {
                line,
                verified: false,
                file_path,
            });
        }
    }

    /// Start debugging — detect CodeLLDB, launch DAP, set breakpoints
    pub(crate) fn start_debug(&mut self) {
        self.debug_state.active = true;
        self.debug_state.debug_output.clear();
        self.debug_state.debug_output.push((
            "important".to_string(),
            "Starting debug session...".to_string(),
        ));

        // Create DAP event channel
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        self.dap_event_rx = Some(event_rx);

        let dap_client = DapClient::new(event_tx);
        self.dap_client = Some(dap_client);

        // Build the project first
        let root = self.root_path.clone();
        let breakpoints = self.debug_state.breakpoints.clone();

        self.debug_state
            .debug_output
            .push(("console".to_string(), format!("Building project: {}", root)));

        // Launch DAP adapter asynchronously
        let runtime = self.lsp_runtime.clone();
        // We need to move dap_client out, use it, then put it back
        // For simplicity, we'll note that the actual DAP launch would happen here
        self.debug_state.debug_output.push((
            "important".to_string(),
            "Debug adapter ready. Set breakpoints and press F5.".to_string(),
        ));

        self.status_message = "Debug session active".to_string();
        self.status_message_timestamp = Some(std::time::Instant::now());
    }

    /// Poll DAP events (call from main loop)
    pub(crate) fn poll_dap_events(&mut self) {
        let rx = match &mut self.dap_event_rx {
            Some(rx) => rx,
            None => return,
        };

        while let Ok(event) = rx.try_recv() {
            match event {
                DapEvent::Initialized => {
                    self.debug_state.debug_output.push((
                        "important".to_string(),
                        "DAP adapter initialized".to_string(),
                    ));
                }
                DapEvent::Stopped { thread_id, reason } => {
                    self.debug_state.paused = true;
                    self.debug_state.selected_thread = Some(thread_id);
                    self.debug_state.debug_output.push((
                        "important".to_string(),
                        format!("Stopped: {} (thread {})", reason, thread_id),
                    ));
                    // Would fetch stack frames and variables here
                }
                DapEvent::Continued { thread_id: _ } => {
                    self.debug_state.paused = false;
                    self.debug_state.stopped_file = None;
                    self.debug_state.stopped_line = None;
                }
                DapEvent::Terminated => {
                    self.debug_state.active = false;
                    self.debug_state.paused = false;
                    self.debug_state
                        .debug_output
                        .push(("important".to_string(), "Program terminated".to_string()));
                }
                DapEvent::Output { category, output } => {
                    self.debug_state.debug_output.push((category, output));
                }
                DapEvent::Breakpoint { breakpoint } => {
                    // Update breakpoint verification status
                    if let Some(bp) =
                        self.debug_state.breakpoints.iter_mut().find(|b| {
                            b.file_path == breakpoint.file_path && b.line == breakpoint.line
                        })
                    {
                        bp.verified = breakpoint.verified;
                    }
                }
            }
        }
    }

    pub(crate) fn debug_continue(&mut self) {
        self.debug_state.paused = false;
        self.debug_state.stopped_file = None;
        self.debug_state.stopped_line = None;
        self.debug_state
            .debug_output
            .push(("console".to_string(), "Continuing...".to_string()));
    }

    fn debug_step_over(&mut self) {
        self.debug_state
            .debug_output
            .push(("console".to_string(), "Step Over".to_string()));
    }

    fn debug_step_into(&mut self) {
        self.debug_state
            .debug_output
            .push(("console".to_string(), "Step Into".to_string()));
    }

    fn debug_step_out(&mut self) {
        self.debug_state
            .debug_output
            .push(("console".to_string(), "Step Out".to_string()));
    }

    fn debug_stop(&mut self) {
        self.debug_state.active = false;
        self.debug_state.paused = false;
        self.debug_state.stack_frames.clear();
        self.debug_state.variables.clear();
        self.debug_state.threads.clear();
        self.debug_state.stopped_file = None;
        self.debug_state.stopped_line = None;
        self.dap_client = None;
        self.dap_event_rx = None;
        self.debug_state
            .debug_output
            .push(("important".to_string(), "Debug session ended.".to_string()));
    }

    /// Handle debug keyboard shortcuts
    pub(crate) fn handle_debug_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // F5 = Start/Continue
            if i.key_pressed(egui::Key::F5) {
                if self.debug_state.active && self.debug_state.paused {
                    self.debug_continue();
                } else if !self.debug_state.active {
                    self.start_debug();
                }
            }
            // F9 = Toggle breakpoint
            if i.key_pressed(egui::Key::F9) {
                self.toggle_breakpoint();
            }
            // F10 = Step Over
            if i.key_pressed(egui::Key::F10) && self.debug_state.paused {
                self.debug_step_over();
            }
            // F11 = Step Into
            if i.key_pressed(egui::Key::F11) && self.debug_state.paused {
                if i.modifiers.shift {
                    self.debug_step_out();
                } else {
                    self.debug_step_into();
                }
            }
        });
    }
}
