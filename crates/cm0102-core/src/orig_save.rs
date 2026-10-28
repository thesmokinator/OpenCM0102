// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Original save container: file table, byte-RLE payloads and known
//! runtime record layouts.
//!
//! Format from `docs/saves.md` (VERIFIED-DATA on a user-supplied
//! 3.9.60 save; entry order cross-checked by carving the
//! `continent.dat` payload of a second sample: six 198-byte records
//! with the six continent names). The `.sav` opens with a file table
//! of 268-byte entries — five little-endian `u32` values (payload
//! offset and size come last) plus a 248-byte NUL-padded file name —
//! followed by contiguous payloads carved strictly by each entry's
//! own offset and size. 3.9.60 payloads are byte-RLE packed
//! (`0x80` is a literal `0x80`; bytes above `0x80` repeat the next
//! byte `b & 0x7F` times; bytes below are literal); modern saves mix
//! raw and packed payloads and the per-entry compression flag still
//! needs locating, so decoding stays an explicit caller choice.
//! Known payload layouts: finance is a flat array of 359-byte club
//! records; discipline entries are 29-byte records and injury
//! entries 31-byte records. Other payload layouts stay open.

/// File-table entry size in bytes.
pub const ENTRY_SIZE: usize = 268;
/// NUL-padded file-name length inside an entry.
pub const NAME_LEN: usize = 248;
/// Finance club-record size in bytes.
pub const FINANCE_RECORD: usize = 359;
/// Discipline record size in bytes.
pub const DISCIPLINE_RECORD: usize = 29;
/// Injury record size in bytes.
pub const INJURY_RECORD: usize = 31;

/// First-entry flag on 3.9.60 saves (RLE-packed payloads).
pub const FLAG_PACKED: u32 = 4;
/// First-entry flag on the modern mixed raw/packed sample.
pub const FLAG_MIXED: u32 = 3;

/// One file-table entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveEntry {
    /// First header word: 4 on the 3.9.60 sample, 3 on the modern
    /// sample. Per-entry meaning stays open.
    pub flag: u32,
    /// Second header word (22 on both samples' first entry).
    /// Semantics open.
    pub word1: u32,
    /// Third header word: the entry count on the first entry of the
    /// modern sample (317). Semantics on other entries open.
    pub word2: u32,
    /// Payload offset from the start of the file.
    pub offset: usize,
    /// Payload size in bytes.
    pub size: usize,
    /// NUL-padded file name (e.g. `continent.dat`).
    pub name: String,
}

/// Why a save slice was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    /// Table slice is not a whole number of entries.
    BadTableLen(usize),
    /// Entry carves outside the file.
    OutOfBounds,
    /// RLE stream ends mid-repeat (a repeat byte with no next byte).
    TruncatedRle,
    /// Payload is not a whole number of fixed-size records.
    BadRecordLen,
}

/// Parse `count` file-table entries from the start of `file`.
pub fn parse_table(file: &[u8], count: usize) -> Result<Vec<SaveEntry>, SaveError> {
    let table_len = count * ENTRY_SIZE;
    if file.len() < table_len {
        return Err(SaveError::BadTableLen(file.len()));
    }
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let base = index * ENTRY_SIZE;
        let entry = &file[base..base + ENTRY_SIZE];
        let word = |at: usize| {
            u32::from_le_bytes([entry[at], entry[at + 1], entry[at + 2], entry[at + 3]])
        };
        let raw_name = &entry[ENTRY_SIZE - NAME_LEN..];
        let len = raw_name.iter().position(|byte| *byte == 0).unwrap_or(raw_name.len());
        entries.push(SaveEntry {
            flag: word(0),
            word1: word(4),
            word2: word(8),
            offset: word(12) as usize,
            size: word(16) as usize,
            name: String::from_utf8_lossy(&raw_name[..len]).into_owned(),
        });
    }
    Ok(entries)
}

/// Carve one entry's payload strictly by its own offset and size.
pub fn carve<'a>(file: &'a [u8], entry: &SaveEntry) -> Result<&'a [u8], SaveError> {
    let end = entry.offset.saturating_add(entry.size);
    if end > file.len() {
        return Err(SaveError::OutOfBounds);
    }
    Ok(&file[entry.offset..end])
}

/// Decode the byte RLE: `0x80` is a literal `0x80` byte, bytes above
/// `0x80` repeat the following byte `b & 0x7F` times, bytes below
/// are literal.
pub fn rle_decode(packed: &[u8]) -> Result<Vec<u8>, SaveError> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < packed.len() {
        let byte = packed[index];
        index += 1;
        if byte == 0x80 {
            out.push(0x80);
        } else if byte > 0x80 {
            let next = *packed.get(index).ok_or(SaveError::TruncatedRle)?;
            index += 1;
            for _ in 0..(byte & 0x7F) {
                out.push(next);
            }
        } else {
            out.push(byte);
        }
    }
    Ok(out)
}

/// Split a payload into fixed-size records; fails on a remainder.
pub fn records(payload: &[u8], record_size: usize) -> Result<Vec<&[u8]>, SaveError> {
    if record_size == 0 || payload.len() % record_size != 0 {
        return Err(SaveError::BadRecordLen);
    }
    Ok(payload.chunks_exact(record_size).collect())
}

/// Offsets of known `general.dat` anchors (see `docs/saves.md`).
pub const GENERAL_DEFAULT_NATION: usize = 4;
/// Save-version anchor offset.
pub const GENERAL_SAVE_VERSION: usize = 2004;
/// Manager-name anchor offset (bounded C string).
pub const GENERAL_MANAGER_NAME: usize = 2024;
/// Managed-club-name anchor offset (bounded C string).
pub const GENERAL_CLUB_NAME: usize = 2084;
/// Bound for the anchor C strings.
pub const GENERAL_NAME_MAX: usize = 60;

/// Read a little-endian `u32` anchor from a payload.
pub fn anchor_u32(payload: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes = payload.get(offset..end)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Read a bounded C string anchor from a payload.
pub fn anchor_name(payload: &[u8], offset: usize) -> Option<String> {
    let field = payload.get(offset..offset.checked_add(GENERAL_NAME_MAX)?)?;
    let len = field.iter().position(|byte| *byte == 0).unwrap_or(field.len());
    Some(String::from_utf8_lossy(&field[..len]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two-entry synthetic table with names and offsets.
    fn table_bytes() -> Vec<u8> {
        let mut table = vec![0u8; 2 * ENTRY_SIZE];
        let mut entry = |index: usize, flag: u32, offset: u32, size: u32, name: &str| {
            let base = index * ENTRY_SIZE;
            table[base..base + 4].copy_from_slice(&flag.to_le_bytes());
            table[base + 12..base + 16].copy_from_slice(&offset.to_le_bytes());
            table[base + 16..base + 20].copy_from_slice(&size.to_le_bytes());
            let name_bytes = name.as_bytes();
            table[base + 20..base + 20 + name_bytes.len()].copy_from_slice(name_bytes);
        };
        entry(0, FLAG_PACKED, 536, 10, "continent.dat");
        entry(1, FLAG_PACKED, 546, 5, "notes.dat");
        table
    }

    /// Tables parse with names, offsets and sizes intact.
    #[test]
    fn tables_parse_entries_in_order() {
        let entries = parse_table(&table_bytes(), 2).expect("parses");
        assert_eq!(entries[0].name, "continent.dat");
        assert_eq!((entries[0].offset, entries[0].size), (536, 10));
        assert_eq!(entries[1].name, "notes.dat");
        assert!(parse_table(&table_bytes(), 3).is_err());
    }

    /// Carving is strictly by offset and size, bounds-checked.
    #[test]
    fn carving_uses_each_entry_layout() {
        let mut file = table_bytes();
        file.extend(vec![0u8; 536 - file.len()]);
        file.extend(1u8..=15u8);
        let entries = parse_table(&file, 2).expect("parses");
        assert_eq!(carve(&file, &entries[0]).expect("carves"), &(1u8..=10u8).collect::<Vec<_>>());
        assert_eq!(carve(&file, &entries[1]).expect("carves"), &(11u8..=15u8).collect::<Vec<_>>());
        let mut bad = entries[1].clone();
        bad.size = usize::MAX;
        assert_eq!(carve(&file, &bad).unwrap_err(), SaveError::OutOfBounds);
    }

    /// Literals, the 0x80 escape and repeats decode exactly.
    #[test]
    fn rle_decodes_documented_scheme() {
        assert_eq!(rle_decode(&[0x01, 0x02]).expect("decodes"), vec![0x01, 0x02]);
        assert_eq!(rle_decode(&[0x80]).expect("decodes"), vec![0x80]);
        assert_eq!(rle_decode(&[0x83, 0x41]).expect("decodes"), vec![0x41, 0x41, 0x41]);
        assert_eq!(rle_decode(&[0x81]).unwrap_err(), SaveError::TruncatedRle);
    }

    /// Record chunking accepts exact multiples only.
    #[test]
    fn record_layouts_chunk_exactly() {
        let finance = vec![0u8; 2 * FINANCE_RECORD];
        assert_eq!(records(&finance, FINANCE_RECORD).expect("chunks").len(), 2);
        let discipline = vec![0u8; DISCIPLINE_RECORD];
        assert_eq!(records(&discipline, DISCIPLINE_RECORD).expect("chunks").len(), 1);
        let injury = vec![0u8; INJURY_RECORD];
        assert_eq!(records(&injury, INJURY_RECORD).expect("chunks").len(), 1);
        assert_eq!(
            records(&[0u8; 30], DISCIPLINE_RECORD).unwrap_err(),
            SaveError::BadRecordLen
        );
    }

    /// Anchors read at their documented offsets.
    #[test]
    fn general_anchors_read_at_offsets() {
        let mut general = vec![0u8; GENERAL_CLUB_NAME + GENERAL_NAME_MAX];
        general[GENERAL_DEFAULT_NATION..GENERAL_DEFAULT_NATION + 4]
            .copy_from_slice(&7u32.to_le_bytes());
        general[GENERAL_SAVE_VERSION..GENERAL_SAVE_VERSION + 4]
            .copy_from_slice(&1u32.to_le_bytes());
        general[GENERAL_MANAGER_NAME..GENERAL_MANAGER_NAME + 6].copy_from_slice(b"Sacchi");
        general[GENERAL_CLUB_NAME..GENERAL_CLUB_NAME + 7].copy_from_slice(b"Arsenal");
        assert_eq!(anchor_u32(&general, GENERAL_DEFAULT_NATION), Some(7));
        assert_eq!(anchor_u32(&general, GENERAL_SAVE_VERSION), Some(1));
        assert_eq!(anchor_name(&general, GENERAL_MANAGER_NAME), Some("Sacchi".to_string()));
        assert_eq!(anchor_name(&general, GENERAL_CLUB_NAME), Some("Arsenal".to_string()));
        assert_eq!(anchor_u32(&general, usize::MAX - 1), None);
    }
}
