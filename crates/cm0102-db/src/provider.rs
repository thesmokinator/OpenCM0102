// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! `DataProvider` implementation backed by the imported SQLite database.

use std::sync::Mutex;

use cm0102_core::traits::{CoreError, Result};
use cm0102_core::{Club, DataProvider, Nation, Player};
use rusqlite::Connection;

/// SQLite-backed read access to the imported game database.
pub struct SqliteDataProvider {
    connection: Mutex<Connection>,
}

impl SqliteDataProvider {
    /// Wrap an open connection (schema must already exist).
    pub fn new(connection: Connection) -> Self {
        Self {
            connection: Mutex::new(connection),
        }
    }

    /// Open a database file, creating the schema when needed.
    pub fn open(path: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        let connection = Connection::open(path)?;
        crate::importer::create_schema(&connection)?;
        Ok(Self::new(connection))
    }
}

fn to_core(error: rusqlite::Error) -> CoreError {
    CoreError::Storage(error.to_string())
}

/// Map one `players` row (the column order used by every query here).
fn row_to_player(row: &rusqlite::Row<'_>) -> rusqlite::Result<Player> {
    let birth = match (
        row.get::<_, Option<i64>>(6)?,
        row.get::<_, Option<i64>>(7)?,
        row.get::<_, Option<i64>>(8)?,
    ) {
        (Some(day), Some(month), Some(year)) => Some(cm0102_core::GameDate::new(
            year as i32,
            month as u32,
            day as u32,
        )),
        _ => None,
    };
    Ok(Player {
        id: row.get::<_, i64>(0)? as u32,
        first_name: row.get(1)?,
        second_name: row.get(2)?,
        club_id: row.get::<_, Option<i64>>(3)?.map(|v| v as u32),
        current_ability: row.get::<_, Option<i64>>(4)?.map(|v| v as i16),
        potential_ability: row.get::<_, Option<i64>>(5)?.map(|v| v as i16),
        birth_date: birth,
        job: row.get::<_, i64>(9).unwrap_or(0) as u8,
        is_player: row.get::<_, i64>(10).unwrap_or(0) != 0,
        squad_number: row
            .get::<_, Option<i64>>(11)
            .unwrap_or(None)
            .map(|v| v as u8),
        home_reputation: row
            .get::<_, Option<i64>>(12)
            .unwrap_or(None)
            .map(|v| v as i16),
        current_reputation: row
            .get::<_, Option<i64>>(13)
            .unwrap_or(None)
            .map(|v| v as i16),
        world_reputation: row
            .get::<_, Option<i64>>(14)
            .unwrap_or(None)
            .map(|v| v as i16),
        attributes: row
            .get::<_, Option<Vec<u8>>>(15)
            .unwrap_or(None)
            .unwrap_or_default(),
    })
}

impl DataProvider for SqliteDataProvider {
    fn get_player(&self, id: u32) -> Result<Player> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        connection
            .query_row(
                "SELECT id, first_name, second_name, club_id, current_ability, \
                 potential_ability, birth_day, birth_month, birth_year, job, is_player, \
                 squad_number, home_reputation, current_reputation, world_reputation, attributes \
                 FROM players WHERE id = ?1",
                [id],
                row_to_player,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound(format!("player {id}")),
                other => to_core(other),
            })
    }

    fn get_players_by_club(&self, club_id: u32) -> Result<Vec<Player>> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        let mut statement = connection
            .prepare(
                "SELECT id, first_name, second_name, club_id, current_ability, \
                 potential_ability, birth_day, birth_month, birth_year, job, is_player, \
                 squad_number, home_reputation, current_reputation, world_reputation, attributes \
                 FROM players WHERE club_id = ?1 ORDER BY second_name",
            )
            .map_err(to_core)?;
        let rows = statement
            .query_map([club_id], row_to_player)
            .map_err(to_core)?;
        rows.collect::<std::result::Result<Vec<Player>, rusqlite::Error>>()
            .map_err(to_core)
    }

    fn get_club(&self, id: u32) -> Result<Club> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        connection
            .query_row(
                "SELECT id, name, league_id FROM clubs WHERE id = ?1",
                [id],
                |row| {
                    Ok(Club {
                        id: row.get::<_, i64>(0)? as u32,
                        name: row.get(1)?,
                        league_id: row.get::<_, Option<i64>>(2)?.map(|id| id as u32),
                    })
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound(format!("club {id}")),
                other => to_core(other),
            })
    }

    fn list_clubs(&self) -> Result<Vec<Club>> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        let mut statement = connection
            .prepare("SELECT id, name, league_id FROM clubs ORDER BY name")
            .map_err(to_core)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Club {
                    id: row.get::<_, i64>(0)? as u32,
                    name: row.get(1)?,
                    league_id: row.get::<_, Option<i64>>(2)?.map(|id| id as u32),
                })
            })
            .map_err(to_core)?;
        rows.collect::<std::result::Result<Vec<Club>, rusqlite::Error>>()
            .map_err(to_core)
    }

    fn count_players(&self) -> Result<u32> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM players", [], |row| row.get(0))
            .map_err(to_core)?;
        Ok(count as u32)
    }

    fn list_nations(&self) -> Result<Vec<Nation>> {
        let connection = self.connection.lock().expect("provider lock poisoned");
        let mut statement = connection
            .prepare("SELECT id, name FROM nations ORDER BY name")
            .map_err(to_core)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Nation {
                    id: row.get::<_, i64>(0)? as u32,
                    name: row.get(1)?,
                })
            })
            .map_err(to_core)?;
        rows.collect::<std::result::Result<Vec<Nation>, rusqlite::Error>>()
            .map_err(to_core)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed an in-memory database with one club and two players.
    fn seeded_provider() -> SqliteDataProvider {
        let connection = Connection::open_in_memory().unwrap();
        crate::importer::create_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO clubs (id, name, league_id) VALUES (1, 'Arsenal', 7)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO players (id, first_name, second_name, club_id, \
                 current_ability, potential_ability) \
                 VALUES (1, 'Thierry', 'Henry', 1, 185, 190), \
                        (2, 'Patrick', 'Vieira', 1, 180, 182)",
                [],
            )
            .unwrap();
        SqliteDataProvider::new(connection)
    }

    /// Club squads come back ordered by second name with abilities intact.
    #[test]
    fn lists_squad_by_club() {
        let provider = seeded_provider();
        let squad = provider.get_players_by_club(1).unwrap();
        assert_eq!(squad.len(), 2);
        assert_eq!(squad[0].full_name(), "Thierry Henry");
        assert_eq!(squad[0].current_ability, Some(185));
        assert_eq!(provider.count_players().unwrap(), 2);
    }

    /// Unknown ids report `NotFound`, not a storage failure.
    #[test]
    fn missing_player_is_not_found() {
        let provider = seeded_provider();
        let error = provider.get_player(999).unwrap_err();
        assert!(matches!(error, CoreError::NotFound(_)), "{error:?}");
    }

    /// Clubs list alphabetically.
    #[test]
    fn lists_clubs() {
        let provider = seeded_provider();
        let clubs = provider.list_clubs().unwrap();
        assert_eq!(clubs.len(), 1);
        assert_eq!(clubs[0].name, "Arsenal");
        assert_eq!(clubs[0].league_id, Some(7));
        assert_eq!(provider.get_club(1).unwrap().name, "Arsenal");
    }

    /// Databases created before league membership existed gain the
    /// column on open, with NULL (unknown) leagues.
    #[test]
    fn old_databases_migrate_clubs() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute(
                "CREATE TABLE clubs (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        connection
            .execute("INSERT INTO clubs (id, name) VALUES (1, 'Arsenal')", [])
            .unwrap();
        crate::importer::create_schema(&connection).unwrap();
        let provider = SqliteDataProvider::new(connection);
        assert_eq!(provider.get_club(1).unwrap().league_id, None);
    }

    /// Nations list alphabetically.
    #[test]
    fn lists_nations() {
        let connection = Connection::open_in_memory().unwrap();
        crate::importer::create_schema(&connection).unwrap();
        connection
            .execute(
                "INSERT INTO nations (id, name, short_name, three_letter) VALUES \
                 (1, 'Italy', 'Italy', 'ITA'), (2, 'England', 'England', 'ENG')",
                [],
            )
            .unwrap();
        let provider = SqliteDataProvider::new(connection);
        let nations = provider.list_nations().unwrap();
        assert_eq!(nations.len(), 2);
        assert_eq!(nations[0].name, "England");
        assert_eq!(nations[1].name, "Italy");
    }
}
