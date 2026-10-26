<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# OpenCM0102 — Pure Open Engine Skeleton for MuseCode

> **Vision:** Full clean-room reimplementation of Championship Manager 01/02. No Wine, no original exe at runtime. User provides freeware .dat files once, we convert to SQLite, engine runs 100% natively on Windows/macOS/Linux. MIT licensed.

## 0. LEGAL - READ FIRST

- NEVER bundle cm0102.exe, .dat, .cfg, graphics. Repo contains ONLY our code.
- App flow: On first launch, wizard asks for path to CM01/02 freeware folder -> validate (check 20 .dat) -> import to user data dir -> never touch originals again.
- Clean-room: Document behavior by playing original game, write spec in docs/, implement from spec. No decompiled code in repo.
- Freeware status: Game was released as freeware in Dec 2008 by Eidos - user must own it, we just read its data.

## 1. Architecture - No Wine

[User Freeware Folder] -> [Importer] -> [SQLite DB] -> [Open Engine] -> Native Binaries

Open Engine Modules:
    - db: DataProvider trait, SQLite impl
    - core: GameState, Calendar, CompetitionEngine, TransferEngine
    - match: MatchEngine trait, Sim v1 (text events), later v2
    - sim: SeasonSimulator
    - ui: egui (immediate mode) replicating classic CM UI but resizable, 4K
    - platform: paths, saves, config

## 2. Tech Stack - Pure Native

- Language: Rust 1.77+ 
- UI: egui 0.27 + eframe (pure Rust, Apple Silicon native)
- DB: rusqlite + serde
- Binary parsing: binrw or nom for .dat files
- RNG: rand + deterministic seed
- Build: Cargo workspace, GitHub Actions

## 3. Repo Structure to Scaffold

/open-cm0102/
  Cargo.toml (workspace)
  /crates/
    /cm0102-core/ -> data/, calendar/, competition/, game_state.rs, traits.rs
    /cm0102-db/ -> importer.rs, dat/reader.rs, validator.rs, formats/
    /cm0102-match/ -> engine.rs, simple_engine.rs, events.rs
    /cm0102-sim/ -> season.rs, day.rs
  /app/ -> main.rs, app.rs, screens/setup_wizard.rs, squad.rs, match_view.rs
  /docs/ -> dat-format.md, match-engine-spec.md
  .github/workflows/build.yml

## 4. Core Traits - Implement These First

pub trait DataProvider: Send + Sync {
    fn get_player(&self, id: u32) -> Result<Player>;
    fn get_players_by_club(&self, club_id: u32) -> Result<Vec<Player>>;
    fn get_club(&self, id: u32) -> Result<Club>;
}

pub trait MatchEngine: Send + Sync {
    fn simulate(&self, ctx: MatchContext) -> MatchResult;
}

## 5. Implementation Order

TASK 1: Scaffold Workspace + App Shell - eframe 1280x800, dark theme #1e5631, SetupWizard file picker
TASK 2: cm0102-db - DAT Importer - binrw parser, SQLite schema players(id, first_name, second_name, club_id, ca, pa)
TASK 3: cm0102-core - GameState, GameDate starts 1 July 2001, scheduler for Premier League
TASK 4: cm0102-match - simple_engine.rs: team_strength = avg CA, goals = Poisson
TASK 5: cm0102-sim - season loop July->May
TASK 6: UI - squad list + league table + Continue button
TASK 7: Saves - JSON + sqlite snapshot

## 6. MVP Definition of Done

- [ ] Picks freeware folder, imports to SQLite
- [ ] Shows 【entity-Arsenal¦canonical_name=Arsenal】 01/02 squad
- [ ] Continue 10 times updates table
- [ ] View Match shows "12' Henry scores!"
- [ ] Runs natively Win/Mac/Linux - no Wine
- [ ] No original files bundled

## 7. Prompt for MuseCode

> You are a senior Rust engineer. Scaffold the pure OpenCM0102 engine as described. Start with TASK 1 and 2. Create a buildable eframe app with setup wizard and DAT importer. Use binrw for parsing, rusqlite for storage. No Wine, no original exe execution. Write clean, documented code.
