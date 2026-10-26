<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Architecture

Technical overview of the OpenCM0102 workspace for contributors.

## Workspace layout

```text
Cargo.toml              workspace (cm0102-core, cm0102-db, cm0102-match, app)
crates/cm0102-core/     domain model, calendar, traits — no I/O
crates/cm0102-db/       ISO/Cabinet readers, .dat parsers, SQLite import + provider
crates/cm0102-match/    match simulation behind the MatchEngine trait
app/                    native egui application (windows and widgets only)
docs/                   specs and guides (this directory)
```

Dependency direction: `app` → `{core, db, match}`;
`db` → `core`; `match` → `core`. `core` depends on nothing but
`serde`. The UI must never depend on a concrete database or engine
implementation — only on the traits.

## Data pipeline

```text
user ISO → ISO 9660 reader → InstallShield cabinet → .dat parsers
    → SQLite (game.db) + extracted/ → DataProvider → UI / engines
```

- `iso9660.rs`: read-only ISO 9660 + Joliet reader, no OS mount.
- `installshield.rs`: InstallShield v5 cabinet reader (chunked raw
  DEFLATE payloads), located via `data1.hdr` / `data1.cab`.
- `formats.rs`: `.dat` record parsers. A parser ships only for a
  layout documented in [dat-format.md](dat-format.md); tests use
  small synthetic fixtures, never original files.
- `importer.rs`: creates the SQLite schema and fills every table in
  one transaction; `provider.rs` implements `DataProvider` over it.

## Core model

- `data.rs`: `Club`, `Player` (54 attributes in `ATTRIBUTE_NAMES`
  order), `Nation`, `Manager`.
- `calendar.rs`: `GameDate`; the season starts 1 July 2001.
- `game_state.rs`: `GameState` (date, managed club, manager,
  season schedule) and the versioned native save envelope
  `SavedCareer`.
- `schedule.rs`: `Schedule`/`Fixture` with round-robin calendar
  generation; the schedule is the first real `DayEventSource`.
- `simulation.rs`: `process_day()` plus event-driven
  `continue_until_event()` against editable `ContinueRules`.
- `traits.rs`: `DataProvider` (read access to the database),
  `MatchEngine` (`simulate` must be deterministic per seed), and
  the shared `CoreError` (`NotFound` vs `Storage`).

## Application screens

`app/src/app.rs` owns a small state machine:

```text
SetupWizard (hub) → NewCareer / LoadCareer → Game → Menu → hub
     ↓ GameSettings / Credits / Tools (DataExplorer, DatabaseEditor)
```

The hub shell (banner plus content) owns every screen except Game;
the game shell (section sidebar, top bar, Continue bar) owns Game
alone, with the Menu submenu as its only way out.

- The ISO import runs on a background thread; the UI polls progress
  events and stays responsive. The remembered ISO position
  (`session.txt`) reopens the database on later launches.
- UI strings live in Fluent locale files under `app/i18n/<lang>/`
  (`app/src/i18n.rs`, `i18n-embed`), embedded in the binary.
  English is the fallback; the OS locale picks the initial
  language and Game Settings switches it live. New UI text goes
  in the `.ftl` files, never inline.
- The career wizard (`app/career.rs`) mirrors the original flow:
  manager → nationality → club → save → start, persisted through
  the native saves documented in [saves.md](saves.md). Starting
  or loading a career opens the main-menu shell (`app/shell.rs`):
  sidebar sections plus a top bar (manager, club, date,
  Continue/Holiday/Save), 800×600 like the original.
- The database editor (`app/editor.rs`) is a generic read-only
  table browser over `game.db`.

## Rules as data (planned)

Competition rules (league formats, points, tiebreakers,
promotion/relegation, cup rounds and draws), transfer rules
(windows, loans, squad and foreign-player limits) and match
constants must live in editable data files shipped with the app,
not hardcoded in Rust. Rationale: real-world rules change across
seasons, and players should be able to update or mod them without
touching code. The default files ship the 01/02 season exactly as
observed and specified in `docs/`, following the v3.9.68 behaviour
target (see [versions.md](versions.md)); the engines only read them.

## Conventions

- Every source file carries the MIT SPDX header; CI enforces it
  (`scripts/check-headers.sh`).
- Code, comments and names are in English; UI strings too.
- New features and fixes ship with tests next to the code they
  cover; the workspace gate is `cargo build`, `cargo test` and
  `cargo clippy -- -D warnings`.
- Behaviour of the original game is specified in `docs/` before it
  is implemented; placeholders (e.g. the current match engine) are
  declared as such in code and docs, never presented as faithful.
