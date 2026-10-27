// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! SQLite schema and import.
//!
//! Milestone 1 imports the discovered game-data inventory (`source_files`)
//! plus any clubs and players a documented `.dat` parser has produced.
//! Per-file record parsers live under `formats/` and are added only when
//! their layout has been specified in `docs/dat-format.md`; until then the
//! inventory alone proves the source plumbing end to end.

use rusqlite::Connection;

use crate::formats::{
    parse_cities, parse_clubs, parse_colours, parse_competition_history, parse_competitions,
    parse_continents, parse_names, parse_nations, parse_officials, parse_stadiums, parse_staff,
    parse_staff_comp_history, parse_staff_competitions, parse_staff_history,
};
use crate::source::GameDataSource;
use crate::Result;

/// Import clubs and players from the extracted game-data files.
///
/// Reads `club.dat`, `staff.dat` and the three name files from the
/// source (cabinet paths such as `CM3 Data/club.dat`) and replaces the
/// `clubs`/`players` tables in a single transaction. Returns the number
/// of players imported.
pub fn import_game_data(connection: &mut Connection, source: &dyn GameDataSource) -> Result<u32> {
    let clubs = parse_clubs(&source.read_entry("CM3 Data/club.dat")?)?;
    let first_names = parse_names(&source.read_entry("CM3 Data/first_names.dat")?);
    let second_names = parse_names(&source.read_entry("CM3 Data/second_names.dat")?);
    let common_names = parse_names(&source.read_entry("CM3 Data/common_names.dat")?);
    let players = parse_staff(
        &source.read_entry("CM3 Data/staff.dat")?,
        &first_names,
        &second_names,
        &common_names,
    );

    let known_clubs: std::collections::HashSet<u32> = clubs.iter().map(|club| club.id).collect();
    create_schema(connection)?;
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM players", [])?;
    transaction.execute("DELETE FROM clubs", [])?;
    {
        let mut statement =
            transaction.prepare(
                "INSERT OR REPLACE INTO clubs (id, name, league_id) VALUES (?1, ?2, ?3)",
            )?;
        for club in &clubs {
            statement.execute(rusqlite::params![
                i64::from(club.id),
                club.name,
                club.league_id.map(i64::from),
            ])?;
        }
        let mut statement = transaction.prepare(
            "INSERT OR REPLACE INTO players \
             (id, first_name, second_name, club_id, current_ability, potential_ability, \
              birth_day, birth_month, birth_year, job, is_player, squad_number, \
              home_reputation, current_reputation, world_reputation, attributes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        )?;
        for player in &players {
            statement.execute(rusqlite::params![
                i64::from(player.id),
                player.first_name,
                player.second_name,
                player
                    .club_id
                    .filter(|club_id| known_clubs.contains(club_id))
                    .map(i64::from),
                player.current_ability.map(i64::from),
                player.potential_ability.map(i64::from),
                player.birth_date.map(|date| i64::from(date.day)),
                player.birth_date.map(|date| i64::from(date.month)),
                player.birth_date.map(|date| i64::from(date.year)),
                i64::from(player.job),
                i64::from(player.is_player),
                player.squad_number.map(i64::from),
                player.home_reputation.map(i64::from),
                player.current_reputation.map(i64::from),
                player.world_reputation.map(i64::from),
                if player.attributes.is_empty() {
                    None
                } else {
                    Some(player.attributes.clone())
                },
            ])?;
        }
    }
    // Every remaining documented table, replaced in the same transaction.
    let nations = parse_nations(&source.read_entry("CM3 Data/nation.dat")?);
    let continents = parse_continents(&source.read_entry("CM3 Data/continent.dat")?);
    let cities = parse_cities(&source.read_entry("CM3 Data/city.dat")?);
    let stadiums = parse_stadiums(&source.read_entry("CM3 Data/stadium.dat")?);
    let colours = parse_colours(&source.read_entry("CM3 Data/colour.dat")?);
    let mut competitions = parse_competitions(&source.read_entry("CM3 Data/club_comp.dat")?);
    let competition_total = competitions.len();
    competitions.extend(parse_competitions(
        &source.read_entry("CM3 Data/nation_comp.dat")?,
    ));
    let staff_competitions =
        parse_staff_competitions(&source.read_entry("CM3 Data/staff_comp.dat")?);
    let officials = parse_officials(
        &source.read_entry("CM3 Data/officials.dat")?,
        &first_names,
        &second_names,
    );
    let mut comp_history =
        parse_competition_history(&source.read_entry("CM3 Data/club_comp_history.dat")?);
    comp_history.extend(parse_competition_history(
        &source.read_entry("CM3 Data/nation_comp_history.dat")?,
    ));
    let staff_history = parse_staff_history(&source.read_entry("CM3 Data/staff_history.dat")?);
    let staff_comp_history =
        parse_staff_comp_history(&source.read_entry("CM3 Data/staff_comp_history.dat")?);

    for table in [
        "nations",
        "continents",
        "cities",
        "stadiums",
        "colours",
        "competitions",
        "staff_competitions",
        "officials",
        "competition_history",
        "staff_history",
        "staff_comp_history",
    ] {
        transaction.execute(&format!("DELETE FROM {table}"), [])?;
    }
    {
        let mut q =
            transaction.prepare("INSERT OR REPLACE INTO nations VALUES (?1, ?2, ?3, ?4, ?5)")?;
        for n in &nations {
            q.execute(rusqlite::params![
                n.id,
                n.name,
                n.short_name,
                n.three_letter,
                n.continent_id
            ])?;
        }
        let mut q =
            transaction.prepare("INSERT OR REPLACE INTO continents VALUES (?1, ?2, ?3, ?4)")?;
        for c in &continents {
            q.execute(rusqlite::params![
                c.id,
                c.name,
                c.three_letter,
                c.federation_name
            ])?;
        }
        let mut q = transaction.prepare("INSERT OR REPLACE INTO cities VALUES (?1, ?2, ?3)")?;
        for c in &cities {
            q.execute(rusqlite::params![c.id, c.name, c.nation_id])?;
        }
        let mut q = transaction
            .prepare("INSERT OR REPLACE INTO stadiums VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for s in &stadiums {
            q.execute(rusqlite::params![
                s.id,
                s.name,
                s.city_id,
                s.capacity,
                s.seating_capacity,
                s.expansion_capacity
            ])?;
        }
        let mut q =
            transaction.prepare("INSERT OR REPLACE INTO colours VALUES (?1, ?2, ?3, ?4, ?5)")?;
        for c in &colours {
            q.execute(rusqlite::params![c.id, c.name, c.red, c.green, c.blue])?;
        }
        let mut q = transaction.prepare(
            "INSERT OR REPLACE INTO competitions VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (index, c) in competitions.iter().enumerate() {
            let kind = if index < competition_total {
                "club"
            } else {
                "nation"
            };
            q.execute(rusqlite::params![
                c.id,
                c.name,
                c.short_name,
                c.three_letter,
                c.nation_id,
                c.continent_id,
                c.reputation,
                kind
            ])?;
        }
        let mut q = transaction
            .prepare("INSERT OR REPLACE INTO staff_competitions VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for c in &staff_competitions {
            q.execute(rusqlite::params![
                c.id,
                c.name,
                c.short_name,
                c.nation_id,
                c.continent_id,
                c.reputation
            ])?;
        }
        let mut q = transaction.prepare(
            "INSERT OR REPLACE INTO officials VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )?;
        for o in &officials {
            q.execute(rusqlite::params![
                o.id,
                o.first_name,
                o.second_name,
                o.nation_id,
                o.city_id,
                o.current_ability,
                o.potential_ability,
                o.reputation,
                o.strictness,
            ])?;
        }
        let mut q = transaction.prepare(
            "INSERT OR REPLACE INTO competition_history VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        for h in &comp_history {
            q.execute(rusqlite::params![
                h.id,
                h.competition_id,
                h.year,
                h.winners_club_id,
                h.runners_up_club_id,
                h.third_club_id,
                h.hosts_club_id
            ])?;
        }
        let mut q = transaction
            .prepare("INSERT OR REPLACE INTO staff_history VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
        for h in &staff_history {
            q.execute(rusqlite::params![
                h.id, h.staff_id, h.year, h.club_id, h.on_loan, h.apps, h.goals
            ])?;
        }
        let mut q = transaction
            .prepare("INSERT OR REPLACE INTO staff_comp_history VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
        for h in &staff_comp_history {
            q.execute(rusqlite::params![
                h.id,
                h.competition_id,
                h.year,
                h.first_staff_id,
                h.second_staff_id,
                h.third_staff_id
            ])?;
        }
    }

    let imported = players.len() as u32;
    transaction.commit()?;
    Ok(imported)
}

/// Create every table if it does not exist yet. Safe to call repeatedly.
pub fn create_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS source_files (
             path TEXT PRIMARY KEY,
             size INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS clubs (
             id INTEGER PRIMARY KEY,
             name TEXT NOT NULL,
             league_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS nations (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, short_name TEXT NOT NULL,
             three_letter TEXT NOT NULL, continent_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS continents (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, three_letter TEXT NOT NULL,
             federation_name TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS cities (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, nation_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS stadiums (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, city_id INTEGER,
             capacity INTEGER, seating_capacity INTEGER, expansion_capacity INTEGER
         );
         CREATE TABLE IF NOT EXISTS colours (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL,
             red INTEGER NOT NULL, green INTEGER NOT NULL, blue INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS competitions (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, short_name TEXT NOT NULL,
             three_letter TEXT NOT NULL, nation_id INTEGER, continent_id INTEGER,
             reputation INTEGER, kind TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS staff_competitions (
             id INTEGER PRIMARY KEY, name TEXT NOT NULL, short_name TEXT NOT NULL,
             nation_id INTEGER, continent_id INTEGER, reputation INTEGER
         );
         CREATE TABLE IF NOT EXISTS officials (
             id INTEGER PRIMARY KEY, first_name TEXT NOT NULL, second_name TEXT NOT NULL,
             nation_id INTEGER, city_id INTEGER, current_ability INTEGER,
             potential_ability INTEGER, reputation INTEGER, strictness INTEGER
         );
         CREATE TABLE IF NOT EXISTS competition_history (
             id INTEGER PRIMARY KEY, competition_id INTEGER, year INTEGER,
             winners_club_id INTEGER, runners_up_club_id INTEGER,
             third_club_id INTEGER, hosts_club_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS staff_history (
             id INTEGER PRIMARY KEY, staff_id INTEGER, year INTEGER, club_id INTEGER,
             on_loan INTEGER NOT NULL DEFAULT 0, apps INTEGER NOT NULL DEFAULT 0,
             goals INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS staff_comp_history (
             id INTEGER PRIMARY KEY, competition_id INTEGER, year INTEGER,
             first_staff_id INTEGER, second_staff_id INTEGER, third_staff_id INTEGER
         );
         CREATE TABLE IF NOT EXISTS players (
             id INTEGER PRIMARY KEY,
             first_name TEXT NOT NULL,
             second_name TEXT NOT NULL,
             club_id INTEGER REFERENCES clubs(id),
             current_ability INTEGER,
             potential_ability INTEGER,
             birth_day INTEGER,
             birth_month INTEGER,
             birth_year INTEGER,
             squad_number INTEGER,
             home_reputation INTEGER,
             current_reputation INTEGER,
             world_reputation INTEGER,
             attributes BLOB,
             job INTEGER NOT NULL DEFAULT 0,
             is_player INTEGER NOT NULL DEFAULT 0
         );",
    )?;
    // Databases created before these columns existed keep their table;
    // add the columns one by one, ignoring duplicates.
    for column in [
        "birth_day INTEGER",
        "birth_month INTEGER",
        "birth_year INTEGER",
        "squad_number INTEGER",
        "home_reputation INTEGER",
        "current_reputation INTEGER",
        "world_reputation INTEGER",
        "attributes BLOB",
        "job INTEGER NOT NULL DEFAULT 0",
        "is_player INTEGER NOT NULL DEFAULT 0",
    ] {
        let _ = connection.execute(&format!("ALTER TABLE players ADD COLUMN {column}"), []);
    }
    let _ = connection.execute("ALTER TABLE clubs ADD COLUMN league_id INTEGER", []);
    let _ = connection.execute("ALTER TABLE officials ADD COLUMN strictness INTEGER", []);
    Ok(())
}

/// Import the `.dat` inventory of a source, replacing any previous one.
///
/// Runs in a single transaction: a failed import leaves the previous
/// inventory untouched.
pub fn import_inventory(connection: &mut Connection, source: &dyn GameDataSource) -> Result<u32> {
    let entries = source.list_entries()?;
    create_schema(connection)?;
    let transaction = connection.transaction()?;
    transaction.execute("DELETE FROM source_files", [])?;
    let mut imported = 0u32;
    {
        let mut statement =
            transaction.prepare("INSERT INTO source_files (path, size) VALUES (?1, ?2)")?;
        for entry in entries.iter().filter(|entry| entry.is_dat_file()) {
            statement.execute(rusqlite::params![entry.path, entry.size as i64])?;
            imported += 1;
        }
    }
    transaction.commit()?;
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceEntry;

    struct StubSource(Vec<SourceEntry>);

    impl GameDataSource for StubSource {
        fn describe(&self) -> String {
            "stub".to_string()
        }
        fn list_entries(&self) -> Result<Vec<SourceEntry>> {
            Ok(self.0.clone())
        }
        fn read_entry(&self, _path: &str) -> Result<Vec<u8>> {
            Ok(Vec::new())
        }
    }

    /// Inventory import stores only `.dat` files, with their sizes.
    #[test]
    fn imports_only_dat_files() {
        let source = StubSource(vec![
            SourceEntry {
                path: "Data/player.dat".to_string(),
                size: 10,
            },
            SourceEntry {
                path: "Data/CLUB.DAT".to_string(),
                size: 20,
            },
            SourceEntry {
                path: "readme.txt".to_string(),
                size: 5,
            },
        ]);
        let mut connection = Connection::open_in_memory().unwrap();
        let imported = import_inventory(&mut connection, &source).unwrap();
        assert_eq!(imported, 2);
        let stored: i64 = connection
            .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(stored, 2);
    }

    /// Re-importing replaces the previous inventory instead of appending.
    #[test]
    fn reimport_replaces_inventory() {
        let first = StubSource(vec![SourceEntry {
            path: "a.dat".to_string(),
            size: 1,
        }]);
        let second = StubSource(vec![SourceEntry {
            path: "b.dat".to_string(),
            size: 2,
        }]);
        let mut connection = Connection::open_in_memory().unwrap();
        import_inventory(&mut connection, &first).unwrap();
        import_inventory(&mut connection, &second).unwrap();
        let path: String = connection
            .query_row("SELECT path FROM source_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(path, "b.dat");
    }
}
