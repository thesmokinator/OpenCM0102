// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Setup Game screen and its satellite screens: the original
//! game's entry structure (start, restore/delete, settings,
//! credits) trimmed to one modern mode. Actions that need an
//! import or an engine say so instead of pretending.

use super::*;
use crate::i18n::{
    available_languages, current_language, display_name, set_language, t, tr,
};

/// Number of columns in the Setup Game button grid.
const SETUP_GRID_COLUMNS: usize = 2;

/// Stable Setup Game grid actions (never translated).
const ACTION_START: &str = "start";
const ACTION_RESTORE: &str = "restore";
const ACTION_DELETE: &str = "delete";
const ACTION_SETTINGS: &str = "settings";
const ACTION_CREDITS: &str = "credits";

impl OpenCmApp {
    /// Setup Game hub: readiness (ISO × database), the ISO/database
    /// flows when something is missing, then the option grid with
    /// database-gated actions. Every launch lands here.
    pub fn show_setup_game(&mut self, ui: &mut egui::Ui, importing: bool) {
        ui.heading(t("setup-title"));
        ui.add_space(4.0);
        self.show_readiness(ui);
        ui.separator();
        if !self.leagues_ready() {
            self.show_iso_flow(ui, importing);
            ui.separator();
        }
        if self.state.managed_club_id.is_some() {
            ui.horizontal(|ui| {
                if ui
                    .add_sized([220.0, 44.0], egui::Button::new(t("setup-return-to-game")))
                    .clicked()
                {
                    self.screen = Screen::Game;
                }
                ui.small(t("setup-return-to-game-hint"));
            });
            ui.add_space(8.0);
        }
        egui::Grid::new("setup_game_grid")
            .num_columns(SETUP_GRID_COLUMNS)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                let leagues_ready = self.leagues_ready();
                self.setup_button(
                    ui,
                    ACTION_START,
                    &t("setup-start-new-game"),
                    &t("setup-start-new-game-hint"),
                    leagues_ready,
                );
                self.setup_button(
                    ui,
                    ACTION_RESTORE,
                    &t("setup-restore-saved-game"),
                    &t("setup-restore-saved-game-hint"),
                    leagues_ready,
                );
                ui.end_row();
                self.setup_button(
                    ui,
                    ACTION_DELETE,
                    &t("setup-delete-saved-game"),
                    &t("setup-delete-saved-game-hint"),
                    leagues_ready,
                );
                self.setup_button(
                    ui,
                    ACTION_SETTINGS,
                    &t("setup-game-settings"),
                    &t("setup-game-settings-hint"),
                    true,
                );
                ui.end_row();
                self.setup_button(
                    ui,
                    ACTION_CREDITS,
                    &t("setup-game-credits"),
                    &t("setup-game-credits-hint"),
                    true,
                );
            });
        if self.db_ready() {
            ui.separator();
            self.show_tools(ui);
        }
        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.small(tr(
                "app-version",
                &[("version", env!("CARGO_PKG_VERSION"))],
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(t("action-exit-game")).clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    /// Developer utilities and session controls, kept out of the game.
    fn show_tools(&mut self, ui: &mut egui::Ui) {
        ui.heading(t("setup-tools-title"));
        ui.small(t("setup-tools-hint"));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(t("tools-data-explorer")).clicked() {
                self.screen = Screen::DataExplorer;
            }
            if ui.button(t("tools-database-editor")).clicked() {
                self.screen = Screen::DatabaseEditor;
                self.editor.loaded = false;
            }
            if read_session().is_some() {
                ui.separator();
                if ui.button(t("setup-disconnect")).clicked() {
                    self.disconnect();
                }
            }
        });
    }

    /// Two-line readiness panel: ISO image state and database state.
    fn show_readiness(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("setup_readiness_grid").show(ui, |ui| {
            ui.strong(t("setup-iso"));
            match self.iso_state() {
                IsoState::Missing => ui.label(t("setup-iso-missing")),
                IsoState::Selected { path, present } => {
                    if present {
                        ui.label(path.display().to_string())
                    } else {
                        let path = path.display().to_string();
                        ui.label(tr("setup-iso-gone", &[("path", &path)]))
                    }
                }
            };
            ui.end_row();
            ui.strong(t("setup-database"));
            if !self.db_ready() {
                ui.label(t("setup-database-missing"));
            } else if self.leagues_ready() {
                let clubs = self.clubs.len().to_string();
                let players = self.player_count.to_string();
                ui.label(tr(
                    "setup-database-ready",
                    &[("clubs", &clubs), ("players", &players)],
                ));
            } else {
                ui.label(t("setup-database-outdated"));
            }
            ui.end_row();
        });
    }

    /// ISO/database flows: select an ISO, import from the known one,
    /// or reconnect when the file moved away.
    fn show_iso_flow(&mut self, ui: &mut egui::Ui, importing: bool) {
        if importing {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new());
                ui.label(&self.import_stage);
            });
            return;
        }
        match self.iso_state() {
            IsoState::Missing => {
                ui.heading(t("setup-select-iso-title"));
                ui.label(t("setup-select-iso-body"));
                ui.add_space(8.0);
                if ui.button(t("setup-select-iso-button")).clicked() {
                    self.pick_iso();
                }
            }
            IsoState::Selected { path, present } => {
                if present {
                    ui.heading(t("setup-import-title"));
                    ui.label(t("setup-import-body"));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t("setup-import-button")).clicked() {
                            self.start_import(path);
                        }
                        if ui.button(t("setup-select-iso-button")).clicked() {
                            self.pick_iso();
                        }
                        if ui.button(t("setup-disconnect")).clicked() {
                            self.disconnect();
                        }
                    });
                } else {
                    ui.heading(t("setup-iso-gone-title"));
                    ui.label(t("setup-iso-gone-body"));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(t("setup-select-iso-button")).clicked() {
                            self.pick_iso();
                        }
                        if ui.button(t("setup-disconnect")).clicked() {
                            self.disconnect();
                        }
                    });
                }
            }
        }
    }

    /// File picker for the original disc image; starts the import.
    fn pick_iso(&mut self) {
        if let Some(file) = rfd::FileDialog::new()
            .add_filter("ISO image", &["iso"])
            .pick_file()
        {
            self.start_import(file);
        }
    }

    /// One large Setup Game grid button with its hint underneath.
    fn setup_button(
        &mut self,
        ui: &mut egui::Ui,
        action: &str,
        label: &str,
        hint: &str,
        enabled: bool,
    ) {
        ui.vertical(|ui| {
            ui.set_min_width(220.0);
            let clicked = ui
                .add_enabled_ui(enabled, |ui| {
                    ui.add_sized([220.0, 44.0], egui::Button::new(label))
                        .clicked()
                })
                .inner;
            if clicked {
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
                if self.leagues_ready() {
                    self.start_new_career();
                } else {
                    self.status = t("setup-need-db");
                }
            }
            ACTION_RESTORE | ACTION_DELETE => {
                if self.leagues_ready() {
                    self.show_load_career_entry();
                } else {
                    self.status = t("setup-need-db");
                }
            }
            ACTION_SETTINGS => self.screen = Screen::GameSettings,
            ACTION_CREDITS => self.screen = Screen::Credits,
            _ => {}
        }
    }

    /// Game settings: language switch plus knobs that unlock with
    /// their engines (see docs/season.md).
    pub fn show_game_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(t("settings-title"));
            ui.separator();
            if ui.button(t("career-back-to-setup")).clicked() {
                self.screen = Screen::SetupWizard;
            }
        });
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
        ui.horizontal(|ui| {
            ui.heading(t("credits-title"));
            ui.separator();
            if ui.button(t("career-back-to-setup")).clicked() {
                self.screen = Screen::SetupWizard;
            }
        });
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
