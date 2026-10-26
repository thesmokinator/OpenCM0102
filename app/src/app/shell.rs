// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Main-menu shell: sidebar navigation with a top info bar, mirroring
//! the original game's main screen layout (sections on the left,
//! content on the right, manager/club/date plus Continue on top).
//! Sections render real data where an engine exists (squad) and an
//! honest placeholder citing the backing spec everywhere else.

use cm0102_core::SavedCareer;

use super::career::{save_career_to, saves_dir};
use super::*;
use crate::i18n::{t, tr};

/// One section of the main-menu sidebar, in sidebar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameSection {
    /// Managed club's squad list.
    #[default]
    Squad,
    /// Formations and instructions.
    Tactics,
    /// Training schedules.
    Training,
    /// Upcoming and past fixtures.
    Fixtures,
    /// League table.
    LeagueTable,
    /// Transfer market.
    Transfers,
    /// Budgets and club finances.
    FinancesInfo,
    /// Board confidence and expectations.
    Board,
    /// Non-playing staff.
    Staff,
    /// Player and staff search.
    Search,
    /// News inbox.
    News,
    /// Club history.
    History,
    /// Club records.
    Records,
    /// Awards roll of honour.
    Awards,
    /// Club information.
    ClubInfo,
}

impl GameSection {
    /// Every section, in the order the sidebar shows them.
    pub const ALL: &'static [GameSection] = &[
        GameSection::Squad,
        GameSection::Tactics,
        GameSection::Training,
        GameSection::Fixtures,
        GameSection::LeagueTable,
        GameSection::Transfers,
        GameSection::FinancesInfo,
        GameSection::Board,
        GameSection::Staff,
        GameSection::Search,
        GameSection::News,
        GameSection::History,
        GameSection::Records,
        GameSection::Awards,
        GameSection::ClubInfo,
    ];

    /// Sidebar label, using the original game's vocabulary in the
    /// current language.
    pub fn label(self) -> String {
        t(match self {
            GameSection::Squad => "section-squad",
            GameSection::Tactics => "section-tactics",
            GameSection::Training => "section-training",
            GameSection::Fixtures => "section-fixtures",
            GameSection::LeagueTable => "section-league-table",
            GameSection::Transfers => "section-transfers",
            GameSection::FinancesInfo => "section-finances-info",
            GameSection::Board => "section-board",
            GameSection::Staff => "section-staff",
            GameSection::Search => "section-search",
            GameSection::News => "section-news",
            GameSection::History => "section-history",
            GameSection::Records => "section-records",
            GameSection::Awards => "section-awards",
            GameSection::ClubInfo => "section-club-info",
        })
    }

    /// Versioned spec backing the section, cited by placeholders.
    pub fn spec(self) -> &'static str {
        match self {
            GameSection::Squad => "docs/players.md",
            GameSection::Tactics => "docs/tactics.md",
            GameSection::Training => "docs/season.md",
            GameSection::Fixtures => "docs/competitions.md",
            GameSection::LeagueTable => "docs/competitions.md",
            GameSection::Transfers => "docs/transfers.md",
            GameSection::FinancesInfo => "docs/season.md",
            GameSection::Board => "docs/season.md",
            GameSection::Staff => "docs/season.md",
            GameSection::Search => "docs/players.md",
            GameSection::News => "docs/season.md",
            GameSection::History => "docs/season.md",
            GameSection::Records => "docs/season.md",
            GameSection::Awards => "docs/season.md",
            GameSection::ClubInfo => "docs/competitions.md",
        }
    }
}

/// Sidebar selection state of the main-menu shell.
#[derive(Default)]
pub struct ShellState {
    /// Currently shown section.
    pub section: GameSection,
}

impl OpenCmApp {
    /// Main-menu screen: top info bar, sidebar, section content.
    pub fn show_game(&mut self, ui: &mut egui::Ui) {
        self.show_game_top_bar(ui);
        ui.separator();
        ui.horizontal(|ui| {
            self.show_sidebar(ui);
            ui.separator();
            ui.vertical(|ui| {
                self.show_section(ui);
            });
        });
    }

    /// Manager, club, date plus Holiday/Save, as on top of the
    /// original main screen. Continue lives in the bottom action bar.
    fn show_game_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let manager = self
                .state
                .manager
                .as_ref()
                .map(|manager| manager.full_name())
                .unwrap_or_else(|| "—".to_string());
            ui.strong(&manager);
            ui.separator();
            ui.label(self.managed_club_name());
            ui.separator();
            ui.label(self.state.current_date.to_string());
            ui.separator();
            if ui.button(t("shell-holiday-7-days")).clicked() {
                for _ in 0..7 {
                    self.state.advance_day();
                }
                self.status = tr(
                    "shell-holidayed-to",
                    &[("date", &self.state.current_date.to_string())],
                );
            }
            if ui.button(t("shell-save-game")).clicked() {
                self.save_active_career();
            }
        });
    }

    /// Section list on the left, original-game order.
    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(140.0);
            for section in GameSection::ALL {
                let selected = self.shell.section == *section;
                if ui.selectable_label(selected, section.label()).clicked() {
                    self.shell.section = *section;
                }
            }
        });
    }

    /// Section content: real data where implemented, an honest
    /// placeholder citing the backing spec everywhere else.
    fn show_section(&mut self, ui: &mut egui::Ui) {
        match self.shell.section {
            GameSection::Squad => self.show_squad_section(ui),
            section => {
                ui.heading(section.label());
                ui.label(tr("shell-not-implemented", &[("spec", section.spec())]));
            }
        }
    }

    /// Managed club's squad, reusing the imported player data.
    fn show_squad_section(&mut self, ui: &mut egui::Ui) {
        let club = self.managed_club_name();
        ui.heading(tr("shell-squad-title", &[("club", &club)]));
        if self.squad.is_empty() {
            ui.label(t("shell-no-players"));
            return;
        }
        egui::Grid::new("shell_squad_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.strong(t("shell-th-name"));
                ui.strong(t("shell-th-role"));
                ui.strong(t("shell-th-ca"));
                ui.strong(t("shell-th-pa"));
                ui.end_row();
                for player in self.squad.iter().take(RENDER_LIMIT) {
                    ui.label(player.full_name());
                    ui.label(role_label(player.job));
                    ui.label(
                        player
                            .current_ability
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "—".to_string()),
                    );
                    ui.label(
                        player
                            .potential_ability
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "—".to_string()),
                    );
                    ui.end_row();
                }
            });
    }

    /// Managed club display name, or `—` when no career is active.
    fn managed_club_name(&self) -> String {
        self.state
            .managed_club_id
            .and_then(|id| self.clubs.iter().find(|club| club.id == id))
            .map(|club| club.name.clone())
            .unwrap_or_else(|| "—".to_string())
    }

    /// Re-save the active career to the file it was started from or
    /// loaded from.
    fn save_active_career(&mut self) {
        let Some(file_name) = self.career.active_save_file.clone() else {
            self.status = t("shell-no-active-save");
            return;
        };
        let path = match saves_dir() {
            Ok(dir) => dir.join(&file_name),
            Err(error) => {
                self.status = error;
                return;
            }
        };
        match save_career_to(&path, &SavedCareer::new(self.state.clone())) {
            Ok(()) => self.status = tr("shell-saved-as", &[("file", &file_name)]),
            Err(error) => self.status = error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sidebar_lists_fifteen_unique_sections() {
        let labels: Vec<String> = GameSection::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), 15);
        assert_eq!(labels.iter().collect::<HashSet<_>>().len(), 15);
    }

    #[test]
    fn shell_opens_on_squad() {
        assert_eq!(ShellState::default().section, GameSection::Squad);
    }

    #[test]
    fn every_section_cites_a_spec() {
        for section in GameSection::ALL {
            assert!(section.spec().starts_with("docs/"), "{section:?}");
        }
    }
}
