//! SQLite database panel — Docker-Desktop / DBeaver-style layout:
//! sidebar shows the open file + table list, central panel shows
//! row preview and a query runner.

use std::path::PathBuf;

use rusqlite::Connection;

use super::button_style::{button_with_icon, glyph, primary_button, primary_button_with_icon};
use super::ui_colors;
use super::BerryCodeApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbTab {
    Preview,
    Query,
}

pub struct DatabaseState {
    pub conn: Option<Connection>,
    pub file_path: Option<PathBuf>,
    pub recent_files: Vec<PathBuf>,
    pub tables: Vec<String>,
    pub selected_table: Option<String>,
    pub preview_columns: Vec<String>,
    pub preview_rows: Vec<Vec<String>>,
    pub preview_limit: usize,
    pub query_input: String,
    pub query_columns: Vec<String>,
    pub query_rows: Vec<Vec<String>>,
    pub query_error: Option<String>,
    pub active_tab: DbTab,
    pub error_message: Option<String>,
}

impl Default for DatabaseState {
    fn default() -> Self {
        Self {
            conn: None,
            file_path: None,
            recent_files: Vec::new(),
            tables: Vec::new(),
            selected_table: None,
            preview_columns: Vec::new(),
            preview_rows: Vec::new(),
            preview_limit: 100,
            query_input: String::new(),
            query_columns: Vec::new(),
            query_rows: Vec::new(),
            query_error: None,
            active_tab: DbTab::Preview,
            error_message: None,
        }
    }
}

impl DatabaseState {
    pub fn open(&mut self, path: PathBuf) {
        match Connection::open(&path) {
            Ok(conn) => {
                self.conn = Some(conn);
                self.file_path = Some(path.clone());
                if !self.recent_files.iter().any(|p| p == &path) {
                    self.recent_files.insert(0, path);
                    self.recent_files.truncate(8);
                }
                self.error_message = None;
                self.refresh_tables();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to open: {e}"));
            }
        }
    }

    pub fn close(&mut self) {
        self.conn = None;
        self.file_path = None;
        self.tables.clear();
        self.selected_table = None;
        self.preview_columns.clear();
        self.preview_rows.clear();
        self.query_columns.clear();
        self.query_rows.clear();
        self.query_error = None;
    }

    pub fn refresh_tables(&mut self) {
        let Some(conn) = &self.conn else { return };
        let mut stmt = match conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        ) {
            Ok(s) => s,
            Err(e) => {
                self.error_message = Some(format!("List tables: {e}"));
                return;
            }
        };
        let names: rusqlite::Result<Vec<String>> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect());
        match names {
            Ok(v) => self.tables = v,
            Err(e) => self.error_message = Some(format!("List tables: {e}")),
        }
    }

    pub fn load_preview(&mut self, table: &str) {
        self.selected_table = Some(table.to_string());
        self.preview_columns.clear();
        self.preview_rows.clear();
        let Some(conn) = &self.conn else { return };
        // Quote identifier with double-quotes; escape any embedded quote.
        let safe_ident = format!("\"{}\"", table.replace('"', "\"\""));
        let sql = format!("SELECT * FROM {safe_ident} LIMIT {}", self.preview_limit);
        match run_select(conn, &sql) {
            Ok((cols, rows)) => {
                self.preview_columns = cols;
                self.preview_rows = rows;
                self.error_message = None;
            }
            Err(e) => self.error_message = Some(format!("Preview: {e}")),
        }
    }

    pub fn run_query(&mut self) {
        self.query_columns.clear();
        self.query_rows.clear();
        self.query_error = None;
        let Some(conn) = &self.conn else {
            self.query_error = Some("No database open".into());
            return;
        };
        let sql = self.query_input.trim();
        if sql.is_empty() {
            return;
        }
        match run_select(conn, sql) {
            Ok((cols, rows)) => {
                self.query_columns = cols;
                self.query_rows = rows;
            }
            Err(e) => self.query_error = Some(e.to_string()),
        }
    }
}

fn run_select(conn: &Connection, sql: &str) -> rusqlite::Result<(Vec<String>, Vec<Vec<String>>)> {
    let mut stmt = conn.prepare(sql)?;
    let cols: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let col_count = cols.len();
    let rows_iter = stmt.query_map([], |row| {
        let mut out = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let v: rusqlite::types::Value = row.get(i)?;
            out.push(value_to_string(&v));
        }
        Ok(out)
    })?;
    let mut rows = Vec::new();
    for r in rows_iter {
        rows.push(r?);
    }
    Ok((cols, rows))
}

fn value_to_string(v: &rusqlite::types::Value) -> String {
    use rusqlite::types::Value;
    match v {
        Value::Null => "NULL".to_string(),
        Value::Integer(n) => n.to_string(),
        Value::Real(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
    }
}

impl BerryCodeApp {
    /// Sidebar content: open-file controls + table list.
    pub(crate) fn render_database_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Database");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            if primary_button_with_icon(ui, glyph::FOLDER_OPENED, "Open .db…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("SQLite", &["db", "sqlite", "sqlite3"])
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.database.open(path);
                }
            }
            if self.database.conn.is_some() && ui.button("Close").clicked() {
                self.database.close();
            }
        });

        if let Some(path) = &self.database.file_path {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    path.file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string()),
                )
                .strong(),
            );
            ui.label(
                egui::RichText::new(path.display().to_string())
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
        }

        if let Some(err) = &self.database.error_message {
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        if self.database.conn.is_none() {
            if !self.database.recent_files.is_empty() {
                ui.label(egui::RichText::new("Recent").small());
                ui.add_space(2.0);
                let recents: Vec<_> = self.database.recent_files.clone();
                for path in recents {
                    let name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    if ui.link(name).clicked() {
                        self.database.open(path);
                    }
                }
            } else {
                ui.label(
                    egui::RichText::new("No database open. Click \"Open .db…\".")
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
            }
            return;
        }

        ui.label(egui::RichText::new("Tables").small());
        ui.add_space(2.0);
        if button_with_icon(ui, glyph::REFRESH, "Refresh").clicked() {
            self.database.refresh_tables();
        }
        ui.add_space(2.0);

        let tables = self.database.tables.clone();
        if tables.is_empty() {
            ui.label(
                egui::RichText::new("(no tables)")
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for name in tables {
                    let selected = self.database.selected_table.as_deref() == Some(name.as_str());
                    if ui.selectable_label(selected, &name).clicked() {
                        self.database.active_tab = DbTab::Preview;
                        self.database.load_preview(&name);
                    }
                }
            });
        }
    }

    /// Central panel content: tabs (Preview / Query) + result grid.
    pub(crate) fn render_database_central(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.database.active_tab, DbTab::Preview, "Preview");
            ui.selectable_value(&mut self.database.active_tab, DbTab::Query, "Query");
        });
        ui.separator();

        match self.database.active_tab {
            DbTab::Preview => self.render_database_preview(ui),
            DbTab::Query => self.render_database_query(ui),
        }
    }

    fn render_database_preview(&mut self, ui: &mut egui::Ui) {
        let Some(table) = self.database.selected_table.clone() else {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Select a table from the sidebar.")
                    .color(egui::Color32::from_gray(150)),
            );
            return;
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(&table).strong());
            ui.label(
                egui::RichText::new(format!("(first {} rows)", self.database.preview_limit))
                    .small()
                    .color(egui::Color32::from_gray(150)),
            );
            if button_with_icon(ui, glyph::REFRESH, "Reload").clicked() {
                self.database.load_preview(&table);
            }
        });
        ui.add_space(4.0);
        render_result_grid(
            ui,
            "db_preview_grid",
            &self.database.preview_columns,
            &self.database.preview_rows,
        );
    }

    fn render_database_query(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let response = ui.add(
            egui::TextEdit::multiline(&mut self.database.query_input)
                .hint_text("SELECT * FROM …")
                .code_editor()
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        let cmd_enter = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            if primary_button(ui, "Run (⌘↩)").clicked() || (response.has_focus() && cmd_enter) {
                self.database.run_query();
            }
            if button_with_icon(ui, glyph::CLEAR_ALL, "Clear").clicked() {
                self.database.query_input.clear();
                self.database.query_columns.clear();
                self.database.query_rows.clear();
                self.database.query_error = None;
            }
        });
        ui.separator();
        if let Some(err) = &self.database.query_error {
            ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err);
            return;
        }
        render_result_grid(
            ui,
            "db_query_grid",
            &self.database.query_columns,
            &self.database.query_rows,
        );
    }
}

fn render_result_grid(ui: &mut egui::Ui, id: &str, cols: &[String], rows: &[Vec<String>]) {
    if cols.is_empty() {
        ui.label(egui::RichText::new("(no result)").color(egui::Color32::from_gray(150)));
        return;
    }
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new(id)
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    for c in cols {
                        ui.label(
                            egui::RichText::new(c)
                                .strong()
                                .color(ui_colors::TEXT_DEFAULT),
                        );
                    }
                    ui.end_row();
                    for row in rows {
                        for cell in row {
                            ui.label(cell);
                        }
                        ui.end_row();
                    }
                });
        });
}
