// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Game-data source. OpenCM0102 supports exactly one input: an ISO image
//! of the original CM 01/02 disc, supplied by the user. The image is read
//! in place (no OS mount, no administrator rights) and is only ever read,
//! never written to. Installed game folders are deliberately not a source:
//! if the game is installed, it can already be played as-is.

use std::path::{Path, PathBuf};

use crate::installshield::InstallShieldCab;
use crate::iso9660::IsoImage;
use crate::{DbError, Result};

/// One file discovered inside the ISO image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    /// Path inside the image, using `/` separators.
    pub path: String,
    /// File size in bytes.
    pub size: u64,
}

impl SourceEntry {
    /// Whether this entry is a `.dat` game-data file (case-insensitive).
    pub fn is_dat_file(&self) -> bool {
        Path::new(&self.path)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"))
    }
}

/// Read-only access to the user-supplied ISO image.
pub trait GameDataSource {
    /// Human-readable description shown in the wizard, e.g. the path.
    fn describe(&self) -> String;
    /// List every file in the image (directories excluded).
    fn list_entries(&self) -> Result<Vec<SourceEntry>>;
    /// Read one file previously returned by `list_entries`.
    fn read_entry(&self, path: &str) -> Result<Vec<u8>>;
}

/// An ISO image of the original game disc, read in place without mounting.
#[derive(Debug)]
pub struct IsoSource {
    path: PathBuf,
    image: IsoImage,
}

impl IsoSource {
    /// Open and parse an ISO image file.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.is_file() {
            return Err(DbError::InvalidSource(format!(
                "ISO file does not exist: {}",
                path.display()
            )));
        }
        let bytes = std::fs::read(&path)?;
        let image = IsoImage::from_bytes(bytes)?;
        Ok(Self { path, image })
    }

    /// The image file this source reads from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// The playable game-data source: the InstallShield cabinet inside the
/// ISO image. Listing and reading expose the cabinet's files (paths such
/// as `CM3 Data/club.dat`), extracted on demand from `data1.cab`.
#[derive(Debug)]
pub struct CabinetSource {
    description: String,
    cabinet_header: InstallShieldCab,
    cabinet_bytes: Vec<u8>,
}

impl CabinetSource {
    /// Find and parse `data1.cab`/`data1.hdr` inside an ISO source.
    pub fn from_iso(iso: &IsoSource) -> Result<Self> {
        let entries = iso.list_entries()?;
        let find = |wanted: &str| -> Result<String> {
            entries
                .iter()
                .map(|entry| entry.path.clone())
                .find(|path| {
                    path.rsplit('/')
                        .next()
                        .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
                })
                .ok_or_else(|| {
                    DbError::InvalidSource(format!(
                        "ISO image does not contain {wanted} (InstallShield cabinet)"
                    ))
                })
        };
        let header = iso.read_entry(&find("data1.hdr")?)?;
        let cabinet_header = InstallShieldCab::parse_header(&header)?;
        let cabinet_bytes = iso.read_entry(&find("data1.cab")?)?;
        Ok(Self {
            description: format!("{} (InstallShield cabinet)", iso.describe()),
            cabinet_header,
            cabinet_bytes,
        })
    }
}

impl GameDataSource for CabinetSource {
    fn describe(&self) -> String {
        self.description.clone()
    }

    fn list_entries(&self) -> Result<Vec<SourceEntry>> {
        let mut entries: Vec<SourceEntry> = self
            .cabinet_header
            .files()
            .iter()
            .map(|file| SourceEntry {
                path: file.path.clone(),
                size: file.expanded_size,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    fn read_entry(&self, path: &str) -> Result<Vec<u8>> {
        self.cabinet_header.extract(&self.cabinet_bytes, path)
    }
}

impl GameDataSource for IsoSource {
    fn describe(&self) -> String {
        format!("ISO image: {}", self.path.display())
    }

    fn list_entries(&self) -> Result<Vec<SourceEntry>> {
        let mut entries: Vec<SourceEntry> = self
            .image
            .entries()
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| SourceEntry {
                path: entry.path.clone(),
                size: entry.size,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    fn read_entry(&self, path: &str) -> Result<Vec<u8>> {
        self.image.read_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A missing ISO file is an error, not an empty source.
    #[test]
    fn missing_iso_is_rejected() {
        assert!(IsoSource::new("/definitely/not/a/real/image.iso").is_err());
    }

    /// `.dat` detection is case-insensitive and ignores other extensions.
    #[test]
    fn dat_detection_is_case_insensitive() {
        let dat = SourceEntry {
            path: "lang.dat".to_string(),
            size: 1,
        };
        let upper = SourceEntry {
            path: "PLAYER.DAT".to_string(),
            size: 1,
        };
        let other = SourceEntry {
            path: "data1.cab".to_string(),
            size: 1,
        };
        assert!(dat.is_dat_file());
        assert!(upper.is_dat_file());
        assert!(!other.is_dat_file());
    }
}
