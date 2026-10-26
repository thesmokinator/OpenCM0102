// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Generic, consultable database editor: every imported table can be
//! browsed, searched and inspected row by row. This is a read-only
//! viewer over the imported SQLite database; it is opened from the
//! Settings menu and is the first building block of the future full
//! database editor.

use super::*;
use rusqlite::types::Value;

/// Rows shown per page.
const PAGE_SIZE: i64 = 100;

impl OpenCmApp {
    /// Database-editor screen: table list, search, paged rows, detail.
    pub fn show_database_editor(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::{t, tr};
        ui.horizontal(|ui| {
            ui.heading(t("screen-database-editor"));
            ui.separator();
            if ui.button(t("career-back-to-setup")).clicked() {
                self.screen = Screen::SetupWizard;
            }
        });
        ui.separator();

        if !self.editor.loaded {
            self.refresh_editor_tables();
            self.editor.loaded = true;
        }

        egui::SidePanel::left("editor_tables")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading(t("editor-tables"));
                let mut changed = false;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (name, count) in self.editor.tables.clone() {
                        let selected = self.editor.selected_table.as_deref() == Some(&name);
                        if ui
                            .selectable_label(selected, format!("{name} ({count})"))
                            .clicked()
                        {
                            self.editor.selected_table = Some(name);
                            self.editor.page = 0;
                            self.editor.selected_row = None;
                            changed = true;
                        }
                    }
                });
                if changed {
                    self.refresh_editor_rows();
                }
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut self.editor.search)
                    .hint_text(t("editor-search-hint"))
                    .desired_width(320.0),
            );
            if search_response.changed() {
                self.editor.page = 0;
                self.editor.selected_row = None;
                self.refresh_editor_rows();
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.editor.page > 0, egui::Button::new(t("action-prev")))
                    .clicked()
                {
                    self.editor.page -= 1;
                    self.editor.selected_row = None;
                    self.refresh_editor_rows();
                }
                let pages = (self.editor.total_rows + PAGE_SIZE - 1) / PAGE_SIZE.max(1);
                let rows = self.editor.total_rows.to_string();
                let page = (self.editor.page + 1).to_string();
                let page_count = pages.max(1).to_string();
                ui.label(tr(
                    "editor-page",
                    &[("rows", &rows), ("page", &page), ("pages", &page_count)],
                ));
                if ui
                    .add_enabled(
                        ((self.editor.page + 1) as i64) < pages,
                        egui::Button::new(t("action-next")),
                    )
                    .clicked()
                {
                    self.editor.page += 1;
                    self.editor.selected_row = None;
                    self.refresh_editor_rows();
                }
            });
            ui.separator();

            let columns = self.editor.columns.clone();
            let mut clicked_row: Option<usize> = None;
            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("editor_grid").striped(true).show(ui, |ui| {
                    for column in &columns {
                        ui.strong(column);
                    }
                    ui.end_row();
                    for (row_index, row) in self.editor.rows.iter().enumerate() {
                        for (col_index, value) in row.iter().enumerate() {
                            if col_index == 0 {
                                let selected = self.editor.selected_row == Some(row_index);
                                if ui.selectable_label(selected, value).clicked() {
                                    clicked_row = Some(row_index);
                                }
                            } else {
                                ui.label(value);
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            if let Some(row_index) = clicked_row {
                self.editor.selected_row = Some(row_index);
            }

            if let Some(row) = self
                .editor
                .selected_row
                .and_then(|index| self.editor.rows.get(index).cloned())
            {
                ui.separator();
                ui.heading(t("editor-row-detail"));
                egui::Grid::new("editor_detail")
                    .striped(true)
                    .show(ui, |ui| {
                        for (column, value) in columns.iter().zip(row.iter()) {
                            ui.strong(column);
                            ui.label(value);
                            ui.end_row();
                        }
                    });
            }
        });
    }

    /// Reload the table list (names and row counts) from the database.
    fn refresh_editor_tables(&mut self) {
        self.editor.tables.clear();
        let Some(db_path) = &self.db_path else {
            return;
        };
        let Ok(connection) = rusqlite::Connection::open(db_path) else {
            return;
        };
        let Ok(mut statement) = connection.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' ORDER BY name",
        ) else {
            return;
        };
        let names: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .map(|rows| rows.filter_map(|row| row.ok()).collect())
            .unwrap_or_default();
        for name in names {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                    row.get(0)
                })
                .unwrap_or(0);
            self.editor.tables.push((name, count));
        }
        if self.editor.selected_table.is_none() {
            self.editor.selected_table = self.editor.tables.first().map(|(name, _)| name.clone());
        }
        self.refresh_editor_rows();
    }

    /// Reload the current page of rows for the selected table.
    fn refresh_editor_rows(&mut self) {
        self.editor.rows.clear();
        self.editor.columns.clear();
        self.editor.total_rows = 0;
        let (Some(db_path), Some(table)) = (&self.db_path, self.editor.selected_table.clone())
        else {
            return;
        };
        let Ok(connection) = rusqlite::Connection::open(db_path) else {
            return;
        };
        let Ok(mut pragma) = connection.prepare(&format!("PRAGMA table_info(\"{table}\")")) else {
            return;
        };
        let columns: Vec<String> = pragma
            .query_map([], |row| row.get::<_, String>(1))
            .map(|rows| rows.filter_map(|row| row.ok()).collect())
            .unwrap_or_default();
        if columns.is_empty() {
            return;
        }
        self.editor.columns = columns.clone();

        let search = self.editor.search.trim().to_string();
        let (where_clause, pattern) = if search.is_empty() {
            (String::new(), String::new())
        } else {
            let pattern = format!(
                "%{}%",
                search
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            let conditions = columns
                .iter()
                .map(|column| format!("CAST(\"{column}\" AS TEXT) LIKE ?1 ESCAPE '\\'"))
                .collect::<Vec<_>>()
                .join(" OR ");
            (format!("WHERE {conditions}"), pattern)
        };

        self.editor.total_rows = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM \"{table}\" {where_clause}"),
                rusqlite::params![pattern],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let offset = self.editor.page as i64 * PAGE_SIZE;
        let query =
            format!("SELECT * FROM \"{table}\" {where_clause} LIMIT {PAGE_SIZE} OFFSET {offset}");
        let Ok(mut statement) = connection.prepare(&query) else {
            return;
        };
        let rows = statement.query_map(rusqlite::params![pattern], |row| {
            let mut values = Vec::new();
            for index in 0..columns.len() {
                let value: Value = row.get(index)?;
                values.push(match value {
                    Value::Null => crate::i18n::t("missing-value"),
                    Value::Integer(v) => v.to_string(),
                    Value::Real(v) => v.to_string(),
                    Value::Text(v) => v,
                    Value::Blob(v) => {
                        let bytes = v.len().to_string();
                        crate::i18n::tr("editor-blob", &[("bytes", &bytes)])
                    }
                });
            }
            Ok(values)
        });
        if let Ok(rows) = rows {
            self.editor.rows = rows.filter_map(|row| row.ok()).collect();
        }
    }
}
