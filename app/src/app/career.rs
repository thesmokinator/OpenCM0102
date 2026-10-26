// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! New-career wizard following the original game's flow: create manager,
//! pick a nationality, pick a club, save, then start. Saved careers use
//! our native versioned JSON envelope ([`SavedCareer`]); loading the
//! original game's binary saves is a separate, declared goal tracked in
//! `docs/saves.md`.

use std::path::{Path, PathBuf};

use cm0102_core::{GameDate, Manager, Nation, SavedCareer};

use super::*;

/// One step of the new-career wizard, in original-game order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CareerStep {
    /// Enter the manager's first and second name.
    #[default]
    ManagerName,
    /// Pick the manager's nationality.
    Nationality,
    /// Pick the club to manage.
    Club,
    /// Name the save, then start the career.
    Save,
}

impl CareerStep {
    /// One-based position and total step count for the header.
    fn position(self) -> (usize, usize) {
        let index = match self {
            CareerStep::ManagerName => 1,
            CareerStep::Nationality => 2,
            CareerStep::Club => 3,
            CareerStep::Save => 4,
        };
        (index, 4)
    }
}

/// One saved career discovered in the saves directory.
#[derive(Debug, Clone)]
pub struct SavedCareerEntry {
    /// File name (with extension) inside the saves directory.
    pub file_name: String,
    /// Manager display name, or `—` when the save has no career.
    pub manager_name: String,
    /// Managed club display name, or `—` when unknown.
    pub club_name: String,
    /// In-game date of the save.
    pub date: GameDate,
}

/// State of the new-career wizard and the load-career screen.
#[derive(Default)]
pub struct CareerState {
    step: CareerStep,
    first_name: String,
    second_name: String,
    nations: Vec<Nation>,
    nation_filter: String,
    filtered_nations: Vec<Nation>,
    selected_nation_id: Option<u32>,
    club_filter: String,
    filtered_clubs: Vec<Club>,
    selected_club_id: Option<u32>,
    save_name: String,
    saves: Vec<SavedCareerEntry>,
    saves_loaded: bool,
    error: Option<String>,
    /// File the active career was started from or loaded from, so the
    /// main-menu shell can re-save without asking again.
    pub active_save_file: Option<String>,
    /// Save file with delete confirmation armed (two-click delete).
    delete_armed: Option<String>,
}

impl OpenCmApp {
    /// Enter the new-career wizard, loading nations from the provider.
    pub fn start_new_career(&mut self) {
        self.career = CareerState::default();
        if let Some(provider) = &self.provider {
            self.career.nations = provider.list_nations().unwrap_or_default();
        }
        self.refresh_nation_filter();
        self.refresh_career_club_filter();
        self.screen = Screen::NewCareer;
    }

    /// Enter the load-career screen, scanning the saves directory.
    pub fn show_load_career_entry(&mut self) {
        self.career.saves_loaded = false;
        self.career.error = None;
        self.screen = Screen::LoadCareer;
    }

    /// New-career wizard screen: dispatch to the current step.
    pub fn show_new_career(&mut self, ui: &mut egui::Ui) {
        let (index, total) = self.career.step.position();
        ui.horizontal(|ui| {
            ui.heading(format!("New career — step {index} of {total}"));
            ui.separator();
            if ui.button("Cancel").clicked() {
                self.screen = Screen::DataExplorer;
            }
        });
        ui.separator();
        if let Some(error) = self.career.error.clone() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        match self.career.step {
            CareerStep::ManagerName => self.show_manager_name_step(ui),
            CareerStep::Nationality => self.show_nationality_step(ui),
            CareerStep::Club => self.show_club_step(ui),
            CareerStep::Save => self.show_save_step(ui),
        }
    }

    /// Step 1: the manager's first and second name.
    fn show_manager_name_step(&mut self, ui: &mut egui::Ui) {
        ui.heading("Create manager");
        egui::Grid::new("manager_name_grid").show(ui, |ui| {
            ui.label("First name");
            ui.add(
                egui::TextEdit::singleline(&mut self.career.first_name)
                    .hint_text("Arrigo")
                    .desired_width(240.0),
            );
            ui.end_row();
            ui.label("Second name");
            ui.add(
                egui::TextEdit::singleline(&mut self.career.second_name)
                    .hint_text("Sacchi")
                    .desired_width(240.0),
            );
            ui.end_row();
        });
        ui.add_space(8.0);
        let ready = !self.career.first_name.trim().is_empty()
            && !self.career.second_name.trim().is_empty();
        if ui
            .add_enabled(ready, egui::Button::new("Next"))
            .clicked()
        {
            self.career.step = CareerStep::Nationality;
            self.career.error = None;
        }
        if !ready {
            ui.label("Enter both a first and a second name to continue.");
        }
    }

    /// Step 2: the manager's nationality.
    fn show_nationality_step(&mut self, ui: &mut egui::Ui) {
        ui.heading("Select nationality");
        if self.career.nations.is_empty() {
            ui.label("No nations imported yet.");
            return;
        }
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.career.nation_filter)
                    .hint_text("Filter nations…")
                    .desired_width(240.0),
            )
            .changed()
        {
            self.refresh_nation_filter();
        }
        ui.label(format!(
            "{} of {} nations",
            self.career.filtered_nations.len(),
            self.career.nations.len()
        ));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for nation in self.career.filtered_nations.clone() {
                let selected = self.career.selected_nation_id == Some(nation.id);
                if ui.selectable_label(selected, &nation.name).clicked() {
                    self.career.selected_nation_id = Some(nation.id);
                    self.career.error = None;
                }
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.career.step = CareerStep::ManagerName;
                self.career.error = None;
            }
            let ready = self.career.selected_nation_id.is_some();
            if ui
                .add_enabled(ready, egui::Button::new("Next"))
                .clicked()
            {
                self.career.step = CareerStep::Club;
                self.career.error = None;
            }
        });
    }

    /// Step 3: the club to manage.
    fn show_club_step(&mut self, ui: &mut egui::Ui) {
        ui.heading("Select club");
        if self.clubs.is_empty() {
            ui.label("No clubs imported yet.");
            return;
        }
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.career.club_filter)
                    .hint_text("Filter clubs…")
                    .desired_width(240.0),
            )
            .changed()
        {
            self.refresh_career_club_filter();
        }
        ui.label(format!(
            "{} of {} clubs",
            self.career.filtered_clubs.len(),
            self.clubs.len()
        ));
        egui::ScrollArea::vertical().show(ui, |ui| {
            for club in self.career.filtered_clubs.iter().take(RENDER_LIMIT).cloned() {
                let selected = self.career.selected_club_id == Some(club.id);
                if ui.selectable_label(selected, &club.name).clicked() {
                    self.career.selected_club_id = Some(club.id);
                    self.career.error = None;
                }
            }
            if self.career.filtered_clubs.len() > RENDER_LIMIT {
                ui.label(format!(
                    "…showing the first {RENDER_LIMIT}; refine the filter."
                ));
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.career.step = CareerStep::Nationality;
                self.career.error = None;
            }
            let ready = self.career.selected_club_id.is_some();
            if ui
                .add_enabled(ready, egui::Button::new("Next"))
                .clicked()
            {
                if self.career.save_name.trim().is_empty() {
                    self.career.save_name = self.default_save_name();
                }
                self.career.step = CareerStep::Save;
                self.career.error = None;
            }
        });
    }

    /// Step 4: review the choices, name the save, start the career.
    fn show_save_step(&mut self, ui: &mut egui::Ui) {
        ui.heading("Save and start");
        let manager_name = format!(
            "{} {}",
            self.career.first_name.trim(),
            self.career.second_name.trim()
        );
        let nation_name = self.selected_nation_name();
        let club_name = self.selected_career_club_name();
        egui::Grid::new("career_summary_grid").show(ui, |ui| {
            ui.strong("Manager");
            ui.label(&manager_name);
            ui.end_row();
            ui.strong("Nationality");
            ui.label(&nation_name);
            ui.end_row();
            ui.strong("Club");
            ui.label(&club_name);
            ui.end_row();
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Save name");
            ui.add(
                egui::TextEdit::singleline(&mut self.career.save_name)
                    .hint_text("My career")
                    .desired_width(240.0),
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                self.career.step = CareerStep::Club;
                self.career.error = None;
            }
            let ready = !self.career.save_name.trim().is_empty();
            if ui
                .add_enabled(ready, egui::Button::new("Save & start career"))
                .clicked()
            {
                self.finish_career_start();
            }
        });
    }

    /// Load-career screen: every native save with a one-line summary.
    pub fn show_load_career(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Load career");
            ui.separator();
            if ui.button("Back to setup").clicked() {
                self.screen = Screen::SetupWizard;
            }
            if ui.button("Refresh").clicked() {
                self.career.saves_loaded = false;
            }
        });
        ui.separator();
        if let Some(error) = self.career.error.clone() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if !self.career.saves_loaded {
            self.refresh_saves();
            self.career.saves_loaded = true;
        }
        if self.career.saves.is_empty() {
            ui.label("No saved careers yet. Start a new career first.");
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("load_career_grid").striped(true).show(ui, |ui| {
                ui.strong("Save");
                ui.strong("Manager");
                ui.strong("Club");
                ui.strong("Date");
                ui.strong("");
                ui.strong("");
                ui.end_row();
                for entry in self.career.saves.clone() {
                    ui.label(&entry.file_name);
                    ui.label(&entry.manager_name);
                    ui.label(&entry.club_name);
                    ui.label(entry.date.to_string());
                    if ui.button("Load").clicked() {
                        self.load_career_file(&entry.file_name);
                    }
                    let armed = self.career.delete_armed.as_deref() == Some(&entry.file_name);
                    if ui.button(if armed { "Sure?" } else { "Delete" }).clicked() {
                        if armed {
                            self.delete_career_file(&entry.file_name);
                        } else {
                            self.career.delete_armed = Some(entry.file_name.clone());
                        }
                    }
                    ui.end_row();
                }
            });
        });
    }

    /// Rebuild the cached nation list for the current filter text.
    fn refresh_nation_filter(&mut self) {
        let needle = self.career.nation_filter.to_lowercase();
        self.career.filtered_nations = self
            .career
            .nations
            .iter()
            .filter(|nation| matches_filter(&nation.name, &needle))
            .cloned()
            .collect();
    }

    /// Rebuild the cached wizard club list for the current filter text.
    fn refresh_career_club_filter(&mut self) {
        let needle = self.career.club_filter.to_lowercase();
        self.career.filtered_clubs = self
            .clubs
            .iter()
            .filter(|club| matches_filter(&club.name, &needle))
            .cloned()
            .collect();
    }

    /// Display name of the selected nationality, or `—` when none.
    fn selected_nation_name(&self) -> String {
        self.career
            .selected_nation_id
            .and_then(|id| self.career.nations.iter().find(|nation| nation.id == id))
            .map(|nation| nation.name.clone())
            .unwrap_or_else(|| "—".to_string())
    }

    /// Display name of the selected club, or `—` when none.
    fn selected_career_club_name(&self) -> String {
        self.career
            .selected_club_id
            .and_then(|id| self.clubs.iter().find(|club| club.id == id))
            .map(|club| club.name.clone())
            .unwrap_or_else(|| "—".to_string())
    }

    /// Suggested save name from the manager and club choices.
    fn default_save_name(&self) -> String {
        format!(
            "{} {} - {}",
            self.career.first_name.trim(),
            self.career.second_name.trim(),
            self.selected_career_club_name()
        )
    }

    /// Write the save file and start the career on the season start date.
    fn finish_career_start(&mut self) {
        let (Some(nation_id), Some(club_id)) =
            (self.career.selected_nation_id, self.career.selected_club_id)
        else {
            self.career.error = Some("Pick a nationality and a club first.".to_string());
            return;
        };
        let manager = Manager {
            first_name: self.career.first_name.trim().to_string(),
            second_name: self.career.second_name.trim().to_string(),
            nation_id,
        };
        let mut state = self.state.clone();
        state.start_career(manager.clone(), club_id);
        let file_name = format!("{}.json", sanitize_save_name(&self.career.save_name));
        let path = match saves_dir() {
            Ok(dir) => dir.join(&file_name),
            Err(error) => {
                self.career.error = Some(error);
                return;
            }
        };
        if let Err(error) = save_career_to(&path, &SavedCareer::new(state.clone())) {
            self.career.error = Some(error);
            return;
        }
        self.state = state;
        self.selected_club_id = Some(club_id);
        self.refresh_squad();
        self.selected_player = None;
        self.screen = Screen::Game;
        self.career.active_save_file = Some(file_name.clone());
        self.status = format!(
            "Career started: {} manages {}. Saved as {file_name}.",
            manager.full_name(),
            self.selected_career_club_name()
        );
    }

    /// Delete a save file after two-click confirmation.
    fn delete_career_file(&mut self, file_name: &str) {
        self.career.delete_armed = None;
        if !is_plain_file_name(file_name) {
            self.career.error = Some("Refusing to delete an unsafe file name.".to_string());
            return;
        }
        let path = match saves_dir() {
            Ok(dir) => dir.join(file_name),
            Err(error) => {
                self.career.error = Some(error);
                return;
            }
        };
        if let Err(error) = std::fs::remove_file(&path) {
            self.career.error = Some(format!("cannot delete save: {error}"));
            return;
        }
        if self.career.active_save_file.as_deref() == Some(file_name) {
            self.career.active_save_file = None;
        }
        self.career.saves_loaded = false;
    }

    /// Rescan the saves directory for loadable native careers.
    fn refresh_saves(&mut self) {
        self.career.saves.clear();
        self.career.error = None;
        let dir = match saves_dir() {
            Ok(dir) => dir,
            Err(error) => {
                self.career.error = Some(error);
                return;
            }
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                self.career.error = Some(format!("cannot list saves: {error}"));
                return;
            }
        };
        let mut saves = Vec::new();
        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            let is_save = path
                .extension()
                .is_some_and(|extension| extension == "json");
            if !is_save {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            match load_career_from(&path) {
                Ok(saved) => saves.push(SavedCareerEntry {
                    file_name,
                    manager_name: saved
                        .state
                        .manager
                        .as_ref()
                        .map(Manager::full_name)
                        .unwrap_or_else(|| "—".to_string()),
                    club_name: saved
                        .state
                        .managed_club_id
                        .map(|id| self.club_name_or_id(id))
                        .unwrap_or_else(|| "—".to_string()),
                    date: saved.state.current_date,
                }),
                Err(error) => {
                    self.career.error = Some(format!("{file_name}: {error}"));
                }
            }
        }
        saves.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        self.career.saves = saves;
    }

    /// Load one native save and resume it in the data explorer.
    fn load_career_file(&mut self, file_name: &str) {
        let path = match saves_dir() {
            Ok(dir) => dir.join(file_name),
            Err(error) => {
                self.career.error = Some(error);
                return;
            }
        };
        match load_career_from(&path) {
            Ok(saved) => {
                self.state = saved.state;
                self.selected_club_id = self.state.managed_club_id;
                self.refresh_squad();
                self.selected_player = None;
                self.screen = Screen::Game;
                self.career.active_save_file = Some(file_name.to_string());
                let manager = self
                    .state
                    .manager
                    .as_ref()
                    .map(Manager::full_name)
                    .unwrap_or_else(|| "—".to_string());
                self.status = format!("Loaded {file_name}: {manager}.");
            }
            Err(error) => {
                self.career.error = Some(format!("{file_name}: {error}"));
            }
        }
    }

    /// Club display name, falling back to the raw id when unknown.
    fn club_name_or_id(&self, club_id: u32) -> String {
        if let Some(provider) = &self.provider {
            if let Ok(club) = provider.get_club(club_id) {
                return club.name;
            }
        }
        self.clubs
            .iter()
            .find(|club| club.id == club_id)
            .map(|club| club.name.clone())
            .unwrap_or_else(|| format!("Club {club_id}"))
    }
}

/// Directory holding native career saves inside the user data directory.
pub(super) fn saves_dir() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("saves"))
}

/// File-system-safe stem for a user-supplied save name.
/// True when `name` is a bare file name with no parent or separator,
/// so joining it under the saves directory cannot escape it.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['/', '\\'])
        && name != "."
        && name != ".."
        && !name.contains("..")
}

fn sanitize_save_name(name: &str) -> String {
    let mut cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cleaned.truncate(64);
    cleaned = cleaned.trim().trim_matches('_').to_string();
    if cleaned.is_empty() {
        cleaned = "career".to_string();
    }
    cleaned
}

/// Write a native career save to `path`, creating parent directories.
pub(super) fn save_career_to(path: &Path, saved: &SavedCareer) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create saves directory: {error}"))?;
    }
    let json =
        serde_json::to_string_pretty(saved).map_err(|error| format!("cannot encode save: {error}"))?;
    std::fs::write(path, json).map_err(|error| format!("cannot write save: {error}"))
}

/// Read a native career save, rejecting unknown envelope versions.
fn load_career_from(path: &Path) -> Result<SavedCareer, String> {
    let text = std::fs::read_to_string(path).map_err(|error| format!("cannot read save: {error}"))?;
    let saved: SavedCareer =
        serde_json::from_str(&text).map_err(|error| format!("cannot decode save: {error}"))?;
    if saved.version != SavedCareer::CURRENT_VERSION {
        return Err(format!(
            "unsupported save version {} (this build reads version {})",
            saved.version,
            SavedCareer::CURRENT_VERSION
        ));
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique scratch directory for file round-trip tests.
    fn scratch_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("opencm0102-test-{}-{}", std::process::id(), name))
    }

    /// Save names keep letters, digits and separators, never path parts.
    #[test]
    fn sanitize_keeps_safe_characters() {
        assert_eq!(sanitize_save_name("Sacchi - Arsenal"), "Sacchi - Arsenal");
        assert_eq!(sanitize_save_name("../evil.sav"), "evil_sav");
        assert_eq!(sanitize_save_name("  "), "career");
        assert_eq!(sanitize_save_name("a/b\\c:d"), "a_b_c_d");
    }

    #[test]
    fn plain_file_name_rejects_escapes() {
        assert!(is_plain_file_name("Sacchi - Arsenal.json"));
        assert!(!is_plain_file_name(""));
        assert!(!is_plain_file_name("../evil.json"));
        assert!(!is_plain_file_name("sub/dir.json"));
        assert!(!is_plain_file_name(".."));
    }

    /// A career written to disk reads back with manager and club intact.
    #[test]
    fn save_file_round_trip() {
        let dir = scratch_dir("round-trip");
        let _ = std::fs::remove_dir_all(&dir);
        let mut state = GameState::new_season();
        state.start_career(
            Manager {
                first_name: "Arrigo".to_string(),
                second_name: "Sacchi".to_string(),
                nation_id: 1,
            },
            676,
        );
        let path = dir.join("test.json");
        save_career_to(&path, &SavedCareer::new(state)).unwrap();
        let restored = load_career_from(&path).unwrap();
        assert_eq!(restored.version, SavedCareer::CURRENT_VERSION);
        assert_eq!(restored.state.managed_club_id, Some(676));
        assert_eq!(
            restored.state.manager.expect("manager").full_name(),
            "Arrigo Sacchi"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Unknown envelope versions are rejected, not silently accepted.
    #[test]
    fn unknown_version_is_rejected() {
        let dir = scratch_dir("version");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("future.json");
        std::fs::write(
            &path,
            r#"{"version":999,"state":{"current_date":{"year":2001,"month":7,"day":1},"managed_club_id":null,"manager":null}}"#,
        )
        .unwrap();
        let error = load_career_from(&path).unwrap_err();
        assert!(error.contains("unsupported save version"), "{error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Missing files report a readable error.
    #[test]
    fn missing_file_is_an_error() {
        let error = load_career_from(&scratch_dir("missing").join("nope.json")).unwrap_err();
        assert!(error.contains("cannot read save"), "{error}");
    }
}
