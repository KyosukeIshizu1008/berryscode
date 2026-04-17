//! Debug panel UI and debug session state management

use super::BerryCodeApp;
use crate::native::dap::*;

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
    pub debug_output: Vec<String>,
    pub program_path: String,
    pub program_args: String,
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
            program_path: String::new(),
            program_args: String::new(),
        }
    }
}

impl BerryCodeApp {
    /// Render the debug panel (shown as a bottom panel when debugging)
    pub(crate) fn render_debug_panel(&mut self, ctx: &egui::Context) {
        if !self.debug_state.active { return; }

        egui::TopBottomPanel::bottom("debug_panel")
            .default_height(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    ui.heading("Debug");
                    ui.separator();

                    let paused = self.debug_state.paused;
                    if paused {
                        if ui.button("\u{25b6} Continue").clicked() {
                            self.debug_continue();
                        }
                        if ui.button("\u{2935} Step Over").clicked() {
                            self.debug_step_over();
                        }
                        if ui.button("\u{2193} Step Into").clicked() {
                            self.debug_step_into();
                        }
                        if ui.button("\u{2191} Step Out").clicked() {
                            self.debug_step_out();
                        }
                    }
                    if ui.button("\u{23f9} Stop").clicked() {
                        self.debug_stop();
                    }
                });

                ui.separator();

                // Split into columns: Variables | Call Stack | Output
                ui.columns(3, |cols| {
                    // Variables
                    cols[0].heading("Variables");
                    egui::ScrollArea::vertical().id_salt("debug_vars").show(&mut cols[0], |ui| {
                        for var in &self.debug_state.variables {
                            ui.horizontal(|ui| {
                                ui.label(&var.name);
                                ui.label("=");
                                ui.monospace(&var.value);
                                if let Some(t) = &var.var_type {
                                    ui.colored_label(egui::Color32::GRAY, t);
                                }
                            });
                        }
                    });

                    // Call Stack
                    cols[1].heading("Call Stack");
                    let mut clicked_frame: Option<(u64, Option<String>, usize, usize)> = None;
                    egui::ScrollArea::vertical().id_salt("debug_stack").show(&mut cols[1], |ui| {
                        for frame in &self.debug_state.stack_frames {
                            let selected = self.debug_state.selected_frame == Some(frame.id);
                            let label = format!("{} (line {})", frame.name, frame.line + 1);
                            if ui.selectable_label(selected, &label).clicked() {
                                clicked_frame = Some((frame.id, frame.file_path.clone(), frame.line, frame.column));
                            }
                        }
                    });
                    if let Some((id, path, line, column)) = clicked_frame {
                        self.debug_state.selected_frame = Some(id);
                        if let Some(p) = &path {
                            self.open_file_from_path(p);
                            if let Some(tab) = self.editor_tabs.get_mut(self.active_tab_idx) {
                                tab.pending_cursor_jump = Some((line, column));
                            }
                        }
                    }

                    // Debug Output
                    cols[2].heading("Output");
                    egui::ScrollArea::vertical().id_salt("debug_output").show(&mut cols[2], |ui| {
                        for line in &self.debug_state.debug_output {
                            ui.monospace(line);
                        }
                    });
                });
            });
    }

    /// Toggle breakpoint at cursor line
    pub(crate) fn toggle_breakpoint(&mut self) {
        if self.editor_tabs.is_empty() { return; }
        let tab = &self.editor_tabs[self.active_tab_idx];
        let file_path = tab.file_path.clone();
        let line = tab.cursor_line;

        if let Some(idx) = self.debug_state.breakpoints.iter().position(|bp| bp.file_path == file_path && bp.line == line) {
            self.debug_state.breakpoints.remove(idx);
        } else {
            self.debug_state.breakpoints.push(DapBreakpoint {
                line,
                verified: false,
                file_path,
            });
        }
    }

    /// Start debugging
    pub(crate) fn start_debug(&mut self) {
        self.debug_state.active = true;
        self.debug_state.debug_output.clear();
        self.debug_state.debug_output.push("Starting debug session...".to_string());
        self.status_message = "Debug session starting...".to_string();
        self.status_message_timestamp = Some(std::time::Instant::now());
    }

    pub(crate) fn debug_continue(&mut self) {
        self.debug_state.paused = false;
    }

    fn debug_step_over(&mut self) {
        // Would call dap_client.step_over() asynchronously
    }

    fn debug_step_into(&mut self) {
        // Would call dap_client.step_into() asynchronously
    }

    fn debug_step_out(&mut self) {
        // Would call dap_client.step_out() asynchronously
    }

    fn debug_stop(&mut self) {
        self.debug_state.active = false;
        self.debug_state.paused = false;
        self.debug_state.stack_frames.clear();
        self.debug_state.variables.clear();
        self.debug_state.threads.clear();
        self.debug_state.debug_output.push("Debug session ended.".to_string());
    }
}
