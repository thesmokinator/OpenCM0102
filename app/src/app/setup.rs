// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Setup Game screen and its satellite screens: the original
//! game's entry structure (start, restore/delete, settings,
//! credits) trimmed to one modern mode. Actions that need an
//! import or an engine say so instead of pretending.

use super::*;
use crate::i18n::{available_languages, current_language, display_name, set_language, t};

/// Number of columns in the Setup Game button grid.
const SETUP_GRID_COLUMNS: usize = 2;

/// Stable Setup Game grid actions (never translated).
const ACTION_START: &str = "start";
const ACTION_RESTORE: &str = "restore";
const ACTION_DELETE: &str = "delete";
const ACTION_SETTINGS: &str = "settings";
const ACTION_CREDITS: &str = "credits";

impl OpenCmApp {
    /// Setup Game root screen: ISO picker when nothing is imported,
    /// then the original game's option grid.
    pub fn show_setup_game(&mut self, ui: &mut egui::Ui, importing: bool) {
        if self.provider.is_none() {
            self.show_iso_picker(ui, importing);
            ui.separator();
        }
        ui.heading(t("setup-title"));
        ui.add_space(8.0);
        egui::Grid::new("setup_game_grid")
            .num_columns(SETUP_GRID_COLUMNS)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                self.setup_button(
                    ui,
                    ACTION_START,
                    &t("setup-start-new-game"),
                    &t("setup-start-new-game-hint"),
                );
                self.setup_button(
                    ui,
                    ACTION_RESTORE,
                    &t("setup-restore-saved-game"),
                    &t("setup-restore-saved-game-hint"),
                );
                ui.end_row();
                self.setup_button(
                    ui,
                    ACTION_DELETE,
                    &t("setup-delete-saved-game"),
                    &t("setup-delete-saved-game-hint"),
                );
                self.setup_button(
                    ui,
                    ACTION_SETTINGS,
                    &t("setup-game-settings"),
                    &t("setup-game-settings-hint"),
                );
                ui.end_row();
                self.setup_button(
                    ui,
                    ACTION_CREDITS,
                    &t("setup-game-credits"),
                    &t("setup-game-credits-hint"),
                );
            });
        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
    }

    /// One large Setup Game grid button with its hint underneath.
    fn setup_button(&mut self, ui: &mut egui::Ui, action: &str, label: &str, hint: &str) {
        ui.vertical(|ui| {
            ui.set_min_width(220.0);
            if ui
                .add_sized([220.0, 44.0], egui::Button::new(label))
                .clicked()
            {
                self.on_setup_action(action);
            }
            ui.small(hint);
        });
    }

    /// Route a Setup Game grid action, guarding on import state.
    fn on_setup_action(&mut self, action: &str) {
        self.error = None;
        match action {
            ACTION_START => {
                if self.provider.is_none() {
                    self.status = t("setup-need-iso");
                } else {
                    self.start_new_career();
                }
            }
            ACTION_RESTORE | ACTION_DELETE => {
                if self.provider.is_none() {
                    self.status = t("setup-need-iso");
                } else {
                    self.show_load_career_entry();
                }
            }
            ACTION_SETTINGS => self.screen = Screen::GameSettings,
            ACTION_CREDITS => self.screen = Screen::Credits,
            _ => {}
        }
    }

    /// ISO picker shown on top of Setup Game until a database is
    /// imported. The image is only read, never modified.
    fn show_iso_picker(&mut self, ui: &mut egui::Ui, importing: bool) {
        ui.heading(t("setup-select-iso-title"));
        ui.label(t("setup-select-iso-body"));
        ui.add_space(8.0);
        if importing {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.label(&self.import_stage);
            });
        } else if ui.button(t("setup-select-iso-button")).clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("ISO image", &["iso"])
                .pick_file()
            {
                self.start_import(file);
            }
        }
        if read_session().is_some() && ui.button(t("setup-disconnect")).clicked() {
            self.disconnect();
        }
    }

    /// Game settings: language switch plus knobs that unlock with
    /// their engines (see docs/season.md).
    pub fn show_game_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("settings-title"));
        let mut current = current_language().to_string();
        egui::ComboBox::from_label(t("settings-language"))
            .selected_text(display_name(&current_language()))
            .show_ui(ui, |ui| {
                for language in available_languages() {
                    if ui
                        .selectable_value(
                            &mut current,
                            language.to_string(),
                            display_name(&language),
                        )
                        .clicked()
                    {
                        let _applied = set_language(&current);
                    }
                }
            });
        ui.separator();
        ui.label(t("settings-intro"));
        for key in [
            "settings-currency",
            "settings-background-pictures",
            "settings-commentary-speed",
            "settings-sound",
        ] {
            ui.add_enabled(false, egui::Button::new(t(key)));
        }
    }

    /// Project credits: original game, data source, this program,
    /// plus project links.
    pub fn show_credits(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("credits-title"));
        ui.label(t("credits-body"));
        ui.separator();
        ui.hyperlink_to(
            t("credits-docs-link"),
            "https://github.com/thesmokinator/OpenCM0102/tree/main/docs",
        );
        ui.hyperlink_to(
            t("credits-repo-link"),
            "https://github.com/thesmokinator/OpenCM0102",
        );
    }
}
