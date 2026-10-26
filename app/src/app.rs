// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Main application state and screens: the setup wizard (first launch),
//! the data explorer, the new-career wizard, and the generic database
//! editor.
//!
//! Selecting an ISO image starts the import on a background thread; the
//! UI only polls progress events and stays responsive, showing a
//! spinner. After a successful import the session (last ISO position)
//! is saved: on the next launch the imported database is reopened
//! directly, unless the ISO is no longer at the saved position, which
//! is reported as an error. The user can disconnect voluntarily from
//! the Settings menu, which forgets the session and returns to the
//! wizard.

mod career;
mod editor;
mod setup;
mod shell;

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;

use cm0102_core::{Club, DataProvider, GameState, Player};
use cm0102_db::{
    import_game_data, import_inventory, validate_source, CabinetSource, GameDataSource, IsoSource,
    SqliteDataProvider, ValidationReport,
};
use eframe::egui;

/// Which screen the application is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// First-launch ISO picker.
    SetupWizard,
    /// Imported-data club/squad browser.
    DataExplorer,
    /// Generic consultable database editor.
    DatabaseEditor,
    /// New-career wizard (manager, nationality, club, save, start).
    NewCareer,
    /// Load a previously saved native career.
    LoadCareer,
    /// Main-menu shell: sidebar sections for the active career.
    Game,
    /// Game settings (knobs unlock with their engines).
    GameSettings,
    /// Project credits and links.
    Credits,
}

impl Screen {
    /// Banner subtitle naming the current screen.
    fn subtitle(self) -> String {
        crate::i18n::t(match self {
            Screen::SetupWizard => "screen-setup-game",
            Screen::DataExplorer => "screen-data-explorer",
            Screen::DatabaseEditor => "screen-database-editor",
            Screen::NewCareer => "screen-new-career",
            Screen::LoadCareer => "screen-load-career",
            Screen::Game => "screen-main-menu",
            Screen::GameSettings => "screen-game-settings",
            Screen::Credits => "screen-credits",
        })
    }
}

/// Events sent by the background import thread.
enum ImportEvent {
    /// A human-readable stage description.
    Progress(String),
    /// The import finished (successfully or not).
    Finished(Result<ImportOutcome, String>),
}

/// Data the UI needs after a successful import.
struct ImportOutcome {
    source_description: String,
    report: ValidationReport,
    iso_path: PathBuf,
    db_path: PathBuf,
}

/// State of the generic database editor screen.
#[derive(Default)]
struct EditorState {
    tables: Vec<(String, i64)>,
    selected_table: Option<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    total_rows: i64,
    page: usize,
    search: String,
    selected_row: Option<usize>,
    loaded: bool,
}

/// Root application object handed to eframe.
pub struct OpenCmApp {
    screen: Screen,
    state: GameState,
    provider: Option<Arc<SqliteDataProvider>>,
    source_description: String,
    report: Option<ValidationReport>,
    clubs: Vec<Club>,
    selected_club_id: Option<u32>,
    squad: Vec<Player>,
    players_only: bool,
    selected_player: Option<Player>,
    club_filter: String,
    filtered_clubs: Vec<Club>,
    player_filter: String,
    filtered_squad: Vec<Player>,
    player_count: u32,
    status: String,
    error: Option<String>,
    import_receiver: Option<mpsc::Receiver<ImportEvent>>,
    import_stage: String,
    db_path: Option<PathBuf>,
    editor: EditorState,
    career: career::CareerState,
    shell: shell::ShellState,
}

impl OpenCmApp {
    /// Create the application, restoring the saved session if possible.
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&creation_context.egui_ctx);
        let mut app = Self {
            screen: Screen::SetupWizard,
            state: GameState::new_season(),
            provider: None,
            source_description: String::new(),
            report: None,
            clubs: Vec::new(),
            selected_club_id: None,
            squad: Vec::new(),
            players_only: true,
            selected_player: None,
            club_filter: String::new(),
            filtered_clubs: Vec::new(),
            player_filter: String::new(),
            filtered_squad: Vec::new(),
            player_count: 0,
            status: "Choose the ISO image of your CM 01/02 disc to begin.".to_string(),
            error: None,
            import_receiver: None,
            import_stage: String::new(),
            db_path: None,
            editor: EditorState::default(),
            career: career::CareerState::default(),
            shell: shell::ShellState::default(),
        };
        app.restore_session();
        app
    }

    /// Reopen the imported database when a saved session exists.
    ///
    /// The ISO is looked up at its last saved position: if it is gone,
    /// that is an error shown in the wizard instead of silently
    /// forgetting the session.
    fn restore_session(&mut self) {
        let Some(iso_path) = read_session() else {
            return;
        };
        let Ok(db_path) = database_path() else {
            return;
        };
        if !iso_path.is_file() {
            self.error = Some(format!(
                "The last ISO image was not found at {}. Reconnect it or disconnect.",
                iso_path.display()
            ));
            return;
        }
        if !db_path.is_file() {
            self.status = format!(
                "Saved ISO found at {}, but no imported database yet. Import it to continue.",
                iso_path.display()
            );
            return;
        }
        match SqliteDataProvider::open(&db_path) {
            Ok(provider) => {
                let provider = Arc::new(provider);
                self.clubs = provider.list_clubs().unwrap_or_default();
                self.refresh_club_filter();
                self.player_count = provider.count_players().unwrap_or(0);
                self.source_description = format!("ISO image: {}", iso_path.display());
                self.db_path = Some(db_path);
                self.provider = Some(provider);
                self.screen = Screen::DataExplorer;
                self.status = "Session restored from the imported database.".to_string();
            }
            Err(error) => {
                self.error = Some(format!("cannot reopen the imported database: {error}"));
            }
        }
    }

    /// Forget the saved session and return to the setup wizard.
    fn disconnect(&mut self) {
        clear_session();
        self.provider = None;
        self.db_path = None;
        self.clubs.clear();
        self.squad.clear();
        self.selected_club_id = None;
        self.selected_player = None;
        self.club_filter.clear();
        self.filtered_clubs.clear();
        self.player_filter.clear();
        self.filtered_squad.clear();
        self.editor = EditorState::default();
        self.career = career::CareerState::default();
        self.state = GameState::new_season();
        self.screen = Screen::SetupWizard;
        self.error = None;
        self.status = "Disconnected. Choose the ISO image to import again.".to_string();
    }

    /// Start the background import of the ISO at `iso_path`.
    fn start_import(&mut self, iso_path: PathBuf) {
        let (sender, receiver) = mpsc::channel();
        self.import_receiver = Some(receiver);
        self.import_stage = "Starting import…".to_string();
        self.error = None;

        std::thread::spawn(move || {
            let send = |message: &str| {
                let _ = sender.send(ImportEvent::Progress(message.to_string()));
            };
            let outcome = run_import(&iso_path, &send);
            let _ = sender.send(ImportEvent::Finished(outcome));
        });
    }

    /// Poll the background import; returns true while it is running.
    fn poll_import(&mut self, ctx: &egui::Context) -> bool {
        let Some(receiver) = &self.import_receiver else {
            return false;
        };
        let mut finished: Option<Result<ImportOutcome, String>> = None;
        loop {
            match receiver.try_recv() {
                Ok(ImportEvent::Progress(stage)) => self.import_stage = stage,
                Ok(ImportEvent::Finished(result)) => {
                    finished = Some(result);
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    finished = Some(Err("import thread stopped unexpectedly".to_string()));
                    break;
                }
            }
        }
        if let Some(result) = finished {
            self.import_receiver = None;
            match result {
                Ok(outcome) => self.on_import_finished(outcome),
                Err(error) => self.error = Some(error),
            }
            return false;
        }
        ctx.request_repaint();
        true
    }

    /// Apply a successful background import to the UI state.
    fn on_import_finished(&mut self, outcome: ImportOutcome) {
        save_session(&outcome.iso_path);
        match SqliteDataProvider::open(&outcome.db_path) {
            Ok(provider) => {
                let provider = Arc::new(provider);
                self.clubs = provider.list_clubs().unwrap_or_default();
                self.refresh_club_filter();
                self.player_count = provider.count_players().unwrap_or(0);
                self.source_description = outcome.source_description;
                self.report = Some(outcome.report);
                self.db_path = Some(outcome.db_path.clone());
                self.provider = Some(provider);
                self.editor = EditorState::default();
                self.career = career::CareerState::default();
                self.screen = Screen::DataExplorer;
                self.status = format!(
                    "Imported into {}. {} players and staff available.",
                    outcome.db_path.display(),
                    self.player_count
                );
            }
            Err(error) => {
                self.error = Some(format!("cannot reopen database: {error}"));
            }
        }
    }

    /// Load the squad of the currently selected club, if any.
    fn refresh_squad(&mut self) {
        if let (Some(provider), Some(club_id)) = (&self.provider, self.selected_club_id) {
            self.squad = provider.get_players_by_club(club_id).unwrap_or_default();
        } else {
            self.squad.clear();
        }
        self.refresh_player_filter();
    }

    /// Rebuild the cached club list for the current filter text.
    ///
    /// Called only when the club list or the filter text changes, never
    /// once per frame, so typing stays responsive with 10k clubs.
    fn refresh_club_filter(&mut self) {
        let needle = self.club_filter.to_lowercase();
        self.filtered_clubs = self
            .clubs
            .iter()
            .filter(|club| matches_filter(&club.name, &needle))
            .cloned()
            .collect();
    }

    /// Rebuild the cached squad list for the current filter text.
    fn refresh_player_filter(&mut self) {
        let needle = self.player_filter.to_lowercase();
        self.filtered_squad = self
            .squad
            .iter()
            .filter(|player| {
                (!self.players_only || player.is_player)
                    && matches_filter(&player.full_name(), &needle)
            })
            .cloned()
            .collect();
    }

    /// Setup-wizard screen: pick the ISO image of the original disc.
    /// Fixed application sidebar: version, navigation, restart/exit.
    /// Visible on every screen, like the original game's menu bar.
    fn show_app_sidebar(&mut self, ui: &mut egui::Ui) {
        use crate::i18n::{t, tr};
        ui.small(tr(
            "sidebar-version",
            &[("version", env!("CARGO_PKG_VERSION"))],
        ));
        ui.separator();
        if ui.button(t("sidebar-setup-game")).clicked() {
            self.screen = Screen::SetupWizard;
        }
        if ui
            .add_enabled(
                self.state.managed_club_id.is_some(),
                egui::Button::new(t("sidebar-main-menu")),
            )
            .clicked()
        {
            self.screen = Screen::Game;
        }
        if ui
            .add_enabled(
                self.provider.is_some(),
                egui::Button::new(t("sidebar-data-explorer")),
            )
            .clicked()
        {
            self.screen = Screen::DataExplorer;
        }
        if ui
            .add_enabled(
                self.provider.is_some(),
                egui::Button::new(t("sidebar-database-editor")),
            )
            .clicked()
        {
            self.screen = Screen::DatabaseEditor;
            self.editor.loaded = false;
        }
        if read_session().is_some() && ui.button(t("sidebar-disconnect")).clicked() {
            self.disconnect();
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if ui.button(t("sidebar-exit-game")).clicked() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.button(t("sidebar-restart-game")).clicked() {
                self.screen = Screen::SetupWizard;
                self.error = None;
                self.status = t("sidebar-back-to-setup");
            }
        });
    }

    /// Data-explorer screen: clubs on the left, squad on the right.
    fn show_data_explorer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Data Explorer");
            ui.separator();
            ui.label(&self.source_description);
            if let Some(report) = &self.report {
                ui.separator();
                ui.label(format!("{} .dat files", report.dat_files.len()));
            }
            ui.separator();
            ui.label(format!("Season date: {}", self.state.current_date));
            if let Some(manager) = &self.state.manager {
                ui.separator();
                ui.label(format!("Manager: {}", manager.full_name()));
            }
            if ui.button("Continue").clicked() {
                self.state.advance_day();
            }
            if ui.button("New career").clicked() {
                self.start_new_career();
            }
        });
        ui.label(&self.status);
        ui.separator();

        if self.clubs.is_empty() {
            ui.label("No clubs imported yet.");
            return;
        }

        let mut selection_changed = false;
        egui::SidePanel::left("club_list")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Clubs");
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.club_filter)
                            .hint_text("Filter clubs…")
                            .desired_width(f32::INFINITY),
                    )
                    .changed()
                {
                    self.refresh_club_filter();
                }
                ui.label(format!(
                    "{} of {} clubs",
                    self.filtered_clubs.len(),
                    self.clubs.len()
                ));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for club in self.filtered_clubs.iter().take(RENDER_LIMIT) {
                        let selected = self.selected_club_id == Some(club.id);
                        if ui.selectable_label(selected, &club.name).clicked() {
                            self.selected_club_id = Some(club.id);
                            selection_changed = true;
                        }
                    }
                    if self.filtered_clubs.len() > RENDER_LIMIT {
                        ui.label(format!(
                            "…showing the first {RENDER_LIMIT}; refine the filter."
                        ));
                    }
                });
            });
        if selection_changed {
            self.refresh_squad();
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Squad");
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut self.players_only, "Players only")
                    .changed()
                {
                    self.refresh_player_filter();
                }
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut self.player_filter)
                            .hint_text("Filter players…")
                            .desired_width(240.0),
                    )
                    .changed()
                {
                    self.refresh_player_filter();
                }
                ui.label(format!(
                    "{} of {}",
                    self.filtered_squad.len(),
                    self.squad.len()
                ));
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("squad_grid").striped(true).show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Role");
                    ui.strong("Born");
                    ui.strong("CA");
                    ui.strong("PA");
                    ui.end_row();
                    for player in self.filtered_squad.iter().take(RENDER_LIMIT) {
                        let selected = self
                            .selected_player
                            .as_ref()
                            .is_some_and(|other| other.id == player.id);
                        if ui.selectable_label(selected, player.full_name()).clicked() {
                            self.selected_player = Some(player.clone());
                        }
                        ui.label(role_label(player.job));
                        ui.label(
                            player
                                .birth_date
                                .map(|date| date.to_string())
                                .unwrap_or_else(|| "—".to_string()),
                        );
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
            });
        });

        if let Some(player) = self.selected_player.clone() {
            egui::TopBottomPanel::bottom("player_detail")
                .resizable(true)
                .show_inside(ui, |ui| {
                    ui.heading(format!(
                        "{} — {} — CA {:?} / PA {:?}",
                        player.full_name(),
                        role_label(player.job),
                        player.current_ability,
                        player.potential_ability
                    ));
                    if !player.attributes.is_empty() {
                        egui::Grid::new("attribute_grid")
                            .striped(true)
                            .show(ui, |ui| {
                                for (index, (name, value)) in cm0102_core::ATTRIBUTE_NAMES
                                    .iter()
                                    .zip(player.attributes.iter())
                                    .enumerate()
                                {
                                    ui.label(*name);
                                    ui.label(value.to_string());
                                    if index % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    }
                });
        }
    }
}

impl eframe::App for OpenCmApp {
    /// Draw the fixed sidebar, banner, action bar and current screen,
    /// polling the import job. Layout mirrors the original game:
    /// navigation on the left, the game on the right.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let importing = self.poll_import(ctx);

        egui::SidePanel::left("sidebar")
            .exact_width(150.0)
            .show(ctx, |ui| self.show_app_sidebar(ui));
        egui::TopBottomPanel::top("banner").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("OpenCM0102");
                ui.separator();
                ui.label(self.screen.subtitle());
                if importing {
                    ui.separator();
                    ui.add(egui::Spinner::new());
                    ui.label(&self.import_stage);
                }
            });
        });
        if self.screen == Screen::Game {
            egui::TopBottomPanel::bottom("action_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button(crate::i18n::t("shell-continue")).clicked() {
                                self.state.advance_day();
                                self.status = crate::i18n::tr(
                                    "shell-advanced-to",
                                    &[("date", &self.state.current_date.to_string())],
                                );
                            }
                        },
                    );
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.screen {
            Screen::SetupWizard => self.show_setup_game(ui, importing),
            Screen::DataExplorer => self.show_data_explorer(ui),
            Screen::DatabaseEditor => self.show_database_editor(ui),
            Screen::NewCareer => self.show_new_career(ui),
            Screen::LoadCareer => self.show_load_career(ui),
            Screen::Game => self.show_game(ui),
            Screen::GameSettings => self.show_game_settings(ui),
            Screen::Credits => self.show_credits(ui),
        });
    }
}

/// The background import job: validate, import every table, extract
/// the game-data files, and report progress through `progress`.
fn run_import(
    iso_path: &std::path::Path,
    progress: &dyn Fn(&str),
) -> Result<ImportOutcome, String> {
    progress("Opening ISO image…");
    let iso = IsoSource::new(iso_path.to_path_buf()).map_err(|e| e.to_string())?;
    progress("Reading InstallShield cabinet…");
    let source = CabinetSource::from_iso(&iso).map_err(|e| e.to_string())?;

    progress("Validating game data…");
    let report = validate_source(&source).map_err(|e| e.to_string())?;

    let db_path = database_path().map_err(|e| e.to_string())?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create data directory: {e}"))?;
    }
    let mut connection =
        rusqlite::Connection::open(&db_path).map_err(|e| format!("cannot open database: {e}"))?;

    progress("Importing file inventory…");
    import_inventory(&mut connection, &source).map_err(|e| format!("import failed: {e}"))?;

    progress("Importing clubs, players and every database table…");
    import_game_data(&mut connection, &source)
        .map_err(|e| format!("game-data import failed: {e}"))?;
    drop(connection);

    progress("Extracting game-data files…");
    let extracted_dir = db_path
        .parent()
        .map(|parent| parent.join("extracted"))
        .unwrap_or_default();
    for path in &report.dat_files {
        let bytes = source
            .read_entry(path)
            .map_err(|e| format!("extraction failed for {path}: {e}"))?;
        let file_name = path.rsplit('/').next().unwrap_or(path);
        std::fs::create_dir_all(&extracted_dir)
            .and_then(|()| std::fs::write(extracted_dir.join(file_name), &bytes))
            .map_err(|e| format!("cannot write extracted file {file_name}: {e}"))?;
    }

    Ok(ImportOutcome {
        source_description: source.describe(),
        report,
        iso_path: iso_path.to_path_buf(),
        db_path,
    })
}

/// Maximum rows drawn for a filtered list. The full match count is
/// shown next to the filter, but creating more widgets than this in one
/// frame would visibly hitch, so the list asks for a narrower filter.
const RENDER_LIMIT: usize = 500;

/// Case-insensitive substring match of `name` against the lowercased
/// filter text `needle`. An empty filter matches everything.
fn matches_filter(name: &str, needle: &str) -> bool {
    needle.is_empty() || name.to_lowercase().contains(needle)
}

/// Display label for a staff job code. Only the codes verified on
/// known records are labelled; the rest stay generic staff.
fn role_label(job: u8) -> &'static str {
    match job {
        11 => "Player",
        5 => "Manager",
        _ => "Staff",
    }
}

/// Apply the classic CM-inspired dark theme with green `#1e5631` panels.
fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(0x1e, 0x56, 0x31);
    visuals.window_fill = egui::Color32::from_rgb(0x1e, 0x56, 0x31);
    ctx.set_visuals(visuals);
}

/// Directory holding the imported database, extracted files and the
/// saved session, inside the user data directory.
fn data_dir() -> Result<PathBuf, String> {
    let base = if cfg!(target_os = "macos") {
        home_dir()?.join("Library").join("Application Support")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").map_err(|_| "APPDATA is not set".to_string())?)
    } else {
        match std::env::var("XDG_DATA_HOME") {
            Ok(value) if !value.is_empty() => PathBuf::from(value),
            _ => home_dir()?.join(".local").join("share"),
        }
    };
    Ok(base.join("OpenCM0102"))
}

/// Location of the imported SQLite database.
fn database_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("game.db"))
}

/// Location of the saved session file (last ISO position).
fn session_path() -> Result<PathBuf, String> {
    Ok(data_dir()?.join("session.txt"))
}

/// Read the saved ISO position, if a session exists.
fn read_session() -> Option<PathBuf> {
    let path = session_path().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(PathBuf::from(line))
    }
}

/// Save the ISO position for the next launch.
fn save_session(iso_path: &std::path::Path) {
    if let Ok(path) = session_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, format!("{}\n", iso_path.display()));
    }
}

/// Forget the saved session (voluntary disconnect).
fn clear_session() {
    if let Ok(path) = session_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// The current user's home directory.
fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Filtering is case-insensitive, matches substrings, and an empty
    /// filter matches every name.
    #[test]
    fn filter_matches_case_insensitively() {
        assert!(matches_filter("Arsenal", ""));
        assert!(matches_filter("Arsenal", "arse"));
        assert!(matches_filter("Arsenal", "ARS".to_lowercase().as_str()));
        assert!(matches_filter("Thierry Henry", "henry"));
        assert!(!matches_filter("Arsenal", "united"));
        assert!(!matches_filter("Arsenal", "arsenals"));
    }
}
