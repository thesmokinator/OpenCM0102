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
        if !self.leagues_ready() {
            self.show_splash(ui, importing);
            return;
        }
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

    /// First-launch splash: pixel-art backdrop, title, and one
    /// vertical button stack driving the ISO import.
    fn show_splash(&mut self, ui: &mut egui::Ui, importing: bool) {
        self.paint_splash_bg(ui);
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            ui.label(
                egui::RichText::new("OpenCM0102")
                    .size(48.0)
                    .color(crate::theme::palette::INK),
            );
            ui.label(
                egui::RichText::new(t("setup-splash-tagline"))
                    .size(15.0)
                    .color(crate::theme::palette::INK_DIM),
            );
            ui.add_space(28.0);
            if importing {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.label(&self.import_stage);
                });
            } else {
                match self.iso_state() {
                    IsoState::Missing => {
                        ui.label(t("setup-select-iso-body"));
                        ui.add_space(8.0);
                        if self.splash_button(ui, &t("setup-select-iso-button")) {
                            self.pick_iso();
                        }
                    }
                    IsoState::Selected { path, present } => {
                        if present {
                            ui.label(t("setup-import-body"));
                            ui.add_space(8.0);
                            if self.splash_button(ui, &t("setup-import-button")) {
                                self.start_import(path);
                            }
                            ui.add_space(4.0);
                            if self.splash_button(ui, &t("setup-select-iso-button")) {
                                self.pick_iso();
                            }
                        } else {
                            ui.label(t("setup-iso-gone-body"));
                            ui.add_space(8.0);
                            if self.splash_button(ui, &t("setup-select-iso-button")) {
                                self.pick_iso();
                            }
                            ui.add_space(4.0);
                            if self.splash_button(ui, &t("setup-disconnect")) {
                                self.disconnect();
                            }
                        }
                    }
                }
            }
            if let Some(error) = self.error.clone() {
                ui.add_space(8.0);
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.small_button(t("setup-game-settings")).clicked() {
                    self.screen = Screen::GameSettings;
                }
                if ui.small_button(t("setup-game-credits")).clicked() {
                    self.screen = Screen::Credits;
                }
                if ui.small_button(t("action-exit-game")).clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });
    }

    /// One full-width splash stack button.
    fn splash_button(&mut self, ui: &mut egui::Ui, label: &str) -> bool {
        ui.add_sized([280.0, 46.0], egui::Button::new(label))
            .clicked()
    }

    /// Pixel-art splash texture, decoded once from the embedded PNG
    /// (nearest filtering keeps the pixels crisp).
    fn splash_texture(&mut self, ctx: &egui::Context) -> Option<(egui::TextureId, egui::Vec2)> {
        if self.splash_tex.is_none() {
            const BYTES: &[u8] = include_bytes!("../../../assets/splash-pixel-stadium.png");
            let decoded =
                image::load_from_memory_with_format(BYTES, image::ImageFormat::Png).ok()?;
            let rgba = decoded.to_rgba8();
            let size = egui::vec2(rgba.width() as f32, rgba.height() as f32);
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [size.x as usize, size.y as usize],
                &rgba,
            );
            self.splash_tex = Some((
                ctx.load_texture("splash", image, egui::TextureOptions::NEAREST),
                size,
            ));
        }
        self.splash_tex
            .as_ref()
            .map(|(handle, size)| (handle.id(), *size))
    }

    /// Paint the splash backdrop edge-to-edge (cover fit) with a
    /// dark overlay for legibility. Must run before any widgets.
    fn paint_splash_bg(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let Some((id, size)) = self.splash_texture(ui.ctx()) else {
            return;
        };
        let img_aspect = size.x / size.y.max(1.0);
        let rect_aspect = rect.width() / rect.height().max(1.0);
        let uv = if rect_aspect > img_aspect {
            let h = (img_aspect / rect_aspect).clamp(0.0, 1.0);
            egui::Rect::from_min_max(
                egui::pos2(0.0, (1.0 - h) / 2.0),
                egui::pos2(1.0, (1.0 + h) / 2.0),
            )
        } else {
            let w = (rect_aspect / img_aspect).clamp(0.0, 1.0);
            egui::Rect::from_min_max(
                egui::pos2((1.0 - w) / 2.0, 0.0),
                egui::pos2((1.0 + w) / 2.0, 1.0),
            )
        };
        ui.painter().image(id, rect, uv, egui::Color32::WHITE);
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(150));
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
            if ui.button(t("tools-data-dir")).clicked() {
                self.pick_data_dir();
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

    /// Point the game at an installed Data directory: its
    /// `events_eng.cfg` and `weather.cfg` override the built-ins.
    fn pick_data_dir(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            super::save_game_data_dir(&dir);
            self.game_data_dir = Some(dir);
            self.reload_commentary_pack();
            self.reload_weather_table();
            let count = self.commentary_pack.len().to_string();
            let zones = self.weather_table.zones.len().to_string();
            self.status = tr(
                "tools-pack-loaded",
                &[("count", &count), ("zones", &zones)],
            );
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
