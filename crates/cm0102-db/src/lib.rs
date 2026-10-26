// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Database layer: game-data sources, validation and SQLite storage.
//!
//! Original CM 01/02 data enters the engine exclusively through this
//! crate, from exactly one source: a user-supplied ISO image of the
//! original disc, behind the `GameDataSource` trait. Nothing from the
//! original game is bundled in this repository.

pub mod formats;
pub mod importer;
pub mod installshield;
pub mod iso9660;
pub mod provider;
pub mod source;
pub mod validator;

pub use importer::{create_schema, import_game_data, import_inventory};
pub use provider::SqliteDataProvider;
pub use source::{CabinetSource, GameDataSource, IsoSource, SourceEntry};
pub use validator::{validate_source, ValidationReport, EXPECTED_DAT_FILE_COUNT};

/// Errors produced by the database layer.
#[derive(Debug)]
pub enum DbError {
    /// An I/O operation failed.
    Io(std::io::Error),
    /// The SQLite store failed.
    Sqlite(rusqlite::Error),
    /// The supplied source is not usable game data.
    InvalidSource(String),
    /// The ISO image is malformed or uses an unsupported feature.
    InvalidIso(String),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Io(e) => write!(f, "I/O error: {e}"),
            DbError::Sqlite(e) => write!(f, "SQLite error: {e}"),
            DbError::InvalidSource(m) => write!(f, "invalid game data source: {m}"),
            DbError::InvalidIso(m) => write!(f, "invalid ISO image: {m}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<std::io::Error> for DbError {
    fn from(value: std::io::Error) -> Self {
        DbError::Io(value)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(value: rusqlite::Error) -> Self {
        DbError::Sqlite(value)
    }
}

/// Result alias for database operations.
pub type Result<T> = std::result::Result<T, DbError>;
