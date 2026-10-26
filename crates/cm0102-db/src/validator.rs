// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Source validation, run by the setup wizard before any import.
//!
//! A genuine CM 01/02 data set ships about twenty `.dat` files. Validation
//! only requires that at least one `.dat` file is present; how the count
//! compares with the reference set is reported so the wizard can warn
//! without refusing partial or trimmed data sets outright.

use crate::source::GameDataSource;
use crate::{DbError, Result};

/// Reference number of `.dat` files in a full CM 01/02 data set.
pub const EXPECTED_DAT_FILE_COUNT: usize = 20;

/// Outcome of validating one game-data source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// Every `.dat` path found in the source, sorted.
    pub dat_files: Vec<String>,
    /// Total number of files (any extension) found in the source.
    pub total_files: usize,
}

impl ValidationReport {
    /// Whether the `.dat` count matches the full reference data set.
    pub fn matches_expected_dat_count(&self) -> bool {
        self.dat_files.len() == EXPECTED_DAT_FILE_COUNT
    }
}

/// Validate a source, failing when it contains no `.dat` files at all.
pub fn validate_source(source: &dyn GameDataSource) -> Result<ValidationReport> {
    let entries = source.list_entries()?;
    let dat_files: Vec<String> = entries
        .iter()
        .filter(|entry| entry.is_dat_file())
        .map(|entry| entry.path.clone())
        .collect();
    if dat_files.is_empty() {
        return Err(DbError::InvalidSource(
            "no .dat game-data files found in the selected source".to_string(),
        ));
    }
    Ok(ValidationReport {
        dat_files,
        total_files: entries.len(),
    })
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

    /// A source without any `.dat` file fails validation.
    #[test]
    fn rejects_source_without_dat_files() {
        let source = StubSource(vec![SourceEntry {
            path: "readme.txt".to_string(),
            size: 4,
        }]);
        assert!(validate_source(&source).is_err());
    }

    /// `.dat` detection is case-insensitive and counted against the
    /// reference set size.
    #[test]
    fn counts_dat_files_case_insensitively() {
        let source = StubSource(vec![
            SourceEntry {
                path: "Data/PLAYER.DAT".to_string(),
                size: 1,
            },
            SourceEntry {
                path: "Data/club.dat".to_string(),
                size: 1,
            },
            SourceEntry {
                path: "notes.txt".to_string(),
                size: 1,
            },
        ]);
        let report = validate_source(&source).unwrap();
        assert_eq!(report.dat_files.len(), 2);
        assert_eq!(report.total_files, 3);
        assert!(!report.matches_expected_dat_count());
    }
}
