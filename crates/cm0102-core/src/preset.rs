// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Preset tactic files (`Data/*.pct`): structural reader.
//!
//! Observed shape from `docs/tactics.md`: files are 1,428, 1,432 or
//! 1,476 bytes with a size-linked header word and a plain-text
//! author at offset 54; formation player bytes pack five high bits
//! per player (masked reads by side type) and slot accessors
//! validate indices 0–10; file names follow the
//! `<shape>[_attacking|_defensive]_default.pct` catalogue.
//! Per-byte instruction semantics stay open, so this reader locks
//! the structure (size, header, author, slot validation, masking)
//! without inventing field meanings.

/// Observed preset sizes, in bytes.
pub const PRESET_SIZES: [usize; 3] = [1428, 1432, 1476];
/// Offset of the plain-text author field.
pub const AUTHOR_OFFSET: usize = 54;
/// Longest author read: bounded so a missing NUL cannot run away.
pub const AUTHOR_MAX_LEN: usize = 32;
/// Formation slots per preset.
pub const PRESET_SLOTS: usize = 11;
/// Mask of the five high bits packed per formation player byte.
pub const PLAYER_BITS_MASK: u8 = 0xF8;

/// Why a preset file was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetError {
    /// Length is not one of the observed sizes.
    BadSize(usize),
    /// Header word does not link to the file size.
    BadHeader(u16),
    /// Slot index outside 0–10.
    BadSlot(usize),
}

/// One parsed preset file: the raw bytes plus structural views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// Raw file bytes (one of the observed sizes).
    bytes: Vec<u8>,
}

impl Preset {
    /// Parse raw file bytes: accept an observed size whose header
    /// word links back to it.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, PresetError> {
        if !PRESET_SIZES.contains(&bytes.len()) {
            return Err(PresetError::BadSize(bytes.len()));
        }
        let header = u16::from_le_bytes([bytes[0], bytes[1]]);
        if usize::from(header) != bytes.len() {
            return Err(PresetError::BadHeader(header));
        }
        Ok(Self { bytes })
    }

    /// File size in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// True for every parsed preset (sizes are never empty).
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Plain-text author at offset 54 (e.g. `Marc Vaughan`),
    /// NUL-terminated and bounded.
    pub fn author(&self) -> String {
        let end = self.bytes.len().min(AUTHOR_OFFSET + AUTHOR_MAX_LEN);
        let field = &self.bytes[AUTHOR_OFFSET..end];
        let len = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
        String::from_utf8_lossy(&field[..len]).into_owned()
    }

    /// Validate a formation slot index (0–10).
    pub fn check_slot(slot: usize) -> Result<(), PresetError> {
        if slot < PRESET_SLOTS {
            Ok(())
        } else {
            Err(PresetError::BadSlot(slot))
        }
    }

    /// Five high bits packed in one formation player byte.
    pub fn player_bits(byte: u8) -> u8 {
        byte & PLAYER_BITS_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic preset with a well-formed header and author.
    fn fixture(size: usize, author: &str) -> Vec<u8> {
        let mut bytes = vec![0u8; size];
        bytes[0..2].copy_from_slice(&(size as u16).to_le_bytes());
        let name = author.as_bytes();
        bytes[AUTHOR_OFFSET..AUTHOR_OFFSET + name.len()].copy_from_slice(name);
        bytes
    }

    /// Observed sizes with linked headers parse; the author reads
    /// back exactly.
    #[test]
    fn well_formed_presets_parse_with_author() {
        for size in PRESET_SIZES {
            let preset = Preset::parse(fixture(size, "Marc Vaughan")).expect("parses");
            assert_eq!(preset.len(), size);
            assert_eq!(preset.author(), "Marc Vaughan");
        }
    }

    /// Unknown sizes, broken headers and bad slots all fail.
    #[test]
    fn malformed_presets_fail() {
        assert_eq!(Preset::parse(vec![0u8; 100]).unwrap_err(), PresetError::BadSize(100));
        let mut broken = fixture(1476, "Marc Vaughan");
        broken[0] = 1;
        assert!(matches!(
            Preset::parse(broken).unwrap_err(),
            PresetError::BadHeader(_)
        ));
        assert_eq!(Preset::check_slot(10), Ok(()));
        assert_eq!(Preset::check_slot(11).unwrap_err(), PresetError::BadSlot(11));
    }

    /// Masking keeps the five high bits only.
    #[test]
    fn masking_keeps_five_high_bits() {
        assert_eq!(Preset::player_bits(0b1110_0101), 0b1110_0000);
        assert_eq!(Preset::player_bits(0b0000_0111), 0);
    }
}
