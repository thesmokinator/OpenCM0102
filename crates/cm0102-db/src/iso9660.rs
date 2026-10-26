// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Minimal read-only ISO 9660 reader with Joliet support.
//!
//! The reader works on the raw image bytes in memory: no OS mount and no
//! administrator rights are needed on any platform. It supports single-
//! extent files (the layout used by pressed game discs) and rejects
//! multi-extent files instead of guessing. Only the primary volume
//! descriptor and, when present, a Joliet supplementary descriptor are
//! consulted.

use crate::{DbError, Result};

/// Size of one ISO 9660 logical sector.
const SECTOR_SIZE: usize = 2048;
/// Byte offset of the volume descriptor set (sector 16).
const VOLUME_DESCRIPTOR_OFFSET: usize = 16 * SECTOR_SIZE;

/// One file or directory inside the image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoEntry {
    /// Full path inside the image, using `/` separators.
    pub path: String,
    /// File size in bytes (zero for directories).
    pub size: u64,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Logical block address of the file data.
    extent_lba: u32,
}

/// A parsed ISO 9660 image held in memory.
#[derive(Debug)]
pub struct IsoImage {
    data: Vec<u8>,
    entries: Vec<IsoEntry>,
}

impl IsoImage {
    /// Parse an image from its raw bytes.
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        if data.len() < VOLUME_DESCRIPTOR_OFFSET + SECTOR_SIZE
            || &data[VOLUME_DESCRIPTOR_OFFSET + 1..VOLUME_DESCRIPTOR_OFFSET + 6] != b"CD001"
        {
            return Err(DbError::InvalidIso(
                "missing ISO 9660 primary volume descriptor".to_string(),
            ));
        }
        let mut root_record_offset = VOLUME_DESCRIPTOR_OFFSET + 156;
        let mut joliet = false;

        // Scan the volume descriptor set: prefer a Joliet supplementary
        // descriptor (type 2 with a UCS-2 escape sequence) when present.
        let mut sector = 16usize;
        loop {
            let offset = sector * SECTOR_SIZE;
            if offset + SECTOR_SIZE > data.len() {
                break;
            }
            let kind = data[offset];
            if kind == 255 {
                break;
            }
            if kind == 2 {
                let escape = &data[offset + 88..offset + 91];
                if escape == b"%/@" || escape == b"%/C" || escape == b"%/E" {
                    root_record_offset = offset + 156;
                    joliet = true;
                }
            }
            sector += 1;
            if sector > 32 {
                break;
            }
        }

        let (root_extent, root_size) = parse_record_header(&data, root_record_offset)?;
        let mut entries = Vec::new();
        walk_directory(&data, root_extent, root_size, "", joliet, &mut entries, 0)?;
        Ok(Self { data, entries })
    }

    /// Every file and directory in the image, in traversal order.
    pub fn entries(&self) -> &[IsoEntry] {
        &self.entries
    }

    /// Read the full contents of the file at `path`.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let entry = self
            .entries
            .iter()
            .find(|e| !e.is_dir && e.path.eq_ignore_ascii_case(path))
            .ok_or_else(|| DbError::InvalidIso(format!("file not found in image: {path}")))?;
        let start = entry.extent_lba as usize * SECTOR_SIZE;
        let end = start
            .checked_add(entry.size as usize)
            .filter(|end| *end <= self.data.len())
            .ok_or_else(|| DbError::InvalidIso(format!("file extends past image end: {path}")))?;
        Ok(self.data[start..end].to_vec())
    }
}

/// Read the little-endian extent LBA and data length of a directory record.
fn parse_record_header(data: &[u8], offset: usize) -> Result<(u32, u32)> {
    if offset + 34 > data.len() || data[offset] == 0 {
        return Err(DbError::InvalidIso(
            "truncated directory record".to_string(),
        ));
    }
    let extent = u32::from_le_bytes(data[offset + 2..offset + 6].try_into().unwrap());
    let size = u32::from_le_bytes(data[offset + 10..offset + 14].try_into().unwrap());
    Ok((extent, size))
}

/// Recursively collect entries below one directory extent.
fn walk_directory(
    data: &[u8],
    extent: u32,
    size: u32,
    prefix: &str,
    joliet: bool,
    entries: &mut Vec<IsoEntry>,
    depth: usize,
) -> Result<()> {
    if depth > 16 {
        return Err(DbError::InvalidIso(
            "directory nesting too deep".to_string(),
        ));
    }
    let start = extent as usize * SECTOR_SIZE;
    let end = start
        .checked_add(size as usize)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| DbError::InvalidIso("directory extends past image end".to_string()))?;
    let dir = &data[start..end];

    let mut pos = 0usize;
    while pos < dir.len() {
        let record_len = dir[pos] as usize;
        if record_len == 0 {
            // Padding to the next sector boundary.
            pos = ((pos / SECTOR_SIZE) + 1) * SECTOR_SIZE;
            continue;
        }
        if pos + record_len > dir.len() || record_len < 34 {
            break;
        }
        let record = &dir[pos..pos + record_len];
        let flags = record[25];
        if flags & 0x80 != 0 {
            return Err(DbError::InvalidIso(
                "multi-extent files are not supported".to_string(),
            ));
        }
        let name_len = record[32] as usize;
        let raw_name = &record[33..33 + name_len];
        pos += record_len;

        // `.` and `..` are stored as single bytes 0x00 and 0x01.
        if raw_name == [0] || raw_name == [1] {
            continue;
        }
        let name = decode_name(raw_name, joliet);
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let (child_extent, child_size) = parse_record_header(dir, pos - record_len)?;
        let is_dir = flags & 0x02 != 0;
        entries.push(IsoEntry {
            path: path.clone(),
            size: u64::from(child_size),
            is_dir,
            extent_lba: child_extent,
        });
        if is_dir {
            walk_directory(
                data,
                child_extent,
                child_size,
                &path,
                joliet,
                entries,
                depth + 1,
            )?;
        }
    }
    Ok(())
}

/// Decode a directory-record file name (ISO 9660 or Joliet UCS-2BE).
///
/// Both encodings may carry the `;1` ISO 9660 version suffix (pressed discs
/// routinely keep it in Joliet names too), so it is stripped after decoding
/// in either case.
fn decode_name(raw: &[u8], joliet: bool) -> String {
    let name = if joliet {
        let chars: Vec<u16> = raw
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16_lossy(&chars)
            .trim_end_matches('\0')
            .to_string()
    } else {
        String::from_utf8_lossy(raw).to_string()
    };
    strip_version_suffix(&name)
}

/// Strip a trailing `;` + digits ISO 9660 version suffix, e.g. `;1`.
fn strip_version_suffix(name: &str) -> String {
    match name.rfind(';') {
        Some(idx)
            if idx > 0
                && !name[idx + 1..].is_empty()
                && name[idx + 1..].chars().all(|c| c.is_ascii_digit()) =>
        {
            name[..idx].to_string()
        }
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal ISO image holding one file `DATA/PLAYER.DAT`.
    fn synthetic_iso(contents: &[u8]) -> Vec<u8> {
        const FILE_LBA: u32 = 21;
        const DIR_LBA: u32 = 19;
        let mut image = vec![0u8; SECTOR_SIZE * 24];

        // Directory record helper: writes one record, returns its length.
        fn write_record(buf: &mut [u8], extent: u32, size: u32, flags: u8, name: &[u8]) -> usize {
            let len = 33 + name.len() + usize::from(name.len() % 2 == 0);
            buf[0] = len as u8;
            buf[2..6].copy_from_slice(&extent.to_le_bytes());
            buf[6..10].copy_from_slice(&extent.to_be_bytes());
            buf[10..14].copy_from_slice(&size.to_le_bytes());
            buf[14..18].copy_from_slice(&size.to_be_bytes());
            buf[25] = flags;
            buf[32] = name.len() as u8;
            buf[33..33 + name.len()].copy_from_slice(name);
            len
        }

        // Primary volume descriptor at sector 16 with root record at +156.
        let pvd = VOLUME_DESCRIPTOR_OFFSET;
        image[pvd] = 1;
        image[pvd + 1..pvd + 6].copy_from_slice(b"CD001");
        write_record(
            &mut image[pvd + 156..],
            DIR_LBA,
            SECTOR_SIZE as u32,
            0x02,
            &[0],
        );
        image[pvd + SECTOR_SIZE] = 255; // Volume descriptor terminator.
        image[pvd + SECTOR_SIZE + 1..pvd + SECTOR_SIZE + 6].copy_from_slice(b"CD001");

        // Root directory: `.`, `..`, then `DATA` in the next sector.
        let root = DIR_LBA as usize * SECTOR_SIZE;
        let data_dir_lba = DIR_LBA + 1;
        let mut p = root;
        p += write_record(&mut image[p..], DIR_LBA, SECTOR_SIZE as u32, 0x02, &[0]);
        p += write_record(&mut image[p..], DIR_LBA, SECTOR_SIZE as u32, 0x02, &[1]);
        write_record(
            &mut image[p..],
            data_dir_lba,
            SECTOR_SIZE as u32,
            0x02,
            b"DATA",
        );

        // DATA directory: `.`, `..`, then the file.
        let data_dir = data_dir_lba as usize * SECTOR_SIZE;
        let mut p = data_dir;
        p += write_record(
            &mut image[p..],
            data_dir_lba,
            SECTOR_SIZE as u32,
            0x02,
            &[0],
        );
        p += write_record(
            &mut image[p..],
            data_dir_lba,
            SECTOR_SIZE as u32,
            0x02,
            &[1],
        );
        write_record(
            &mut image[p..],
            FILE_LBA,
            contents.len() as u32,
            0,
            b"PLAYER.DAT;1",
        );

        let file_offset = FILE_LBA as usize * SECTOR_SIZE;
        image[file_offset..file_offset + contents.len()].copy_from_slice(contents);
        image
    }

    /// A synthetic image is walked and its file contents round-trip.
    #[test]
    fn reads_file_from_synthetic_iso() {
        let image = IsoImage::from_bytes(synthetic_iso(b"synthetic-player-data")).unwrap();
        let paths: Vec<&str> = image.entries().iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"DATA"), "entries: {paths:?}");
        assert!(paths.contains(&"DATA/PLAYER.DAT"), "entries: {paths:?}");
        let bytes = image.read_file("DATA/PLAYER.DAT").unwrap();
        assert_eq!(bytes, b"synthetic-player-data");
    }

    /// Joliet names keep the `;1` version suffix on pressed discs; it must
    /// be stripped or extensions such as `.dat` are never recognised
    /// (regression test for the real CM 01/02 ISO, whose files are listed
    /// as e.g. `lang.dat;1`).
    #[test]
    fn joliet_names_have_version_suffix_stripped() {
        let raw: Vec<u8> = "lang.dat;1"
            .encode_utf16()
            .flat_map(|unit| unit.to_be_bytes())
            .collect();
        assert_eq!(decode_name(&raw, true), "lang.dat");
    }

    /// Plain ISO 9660 names have their version suffix stripped as well,
    /// while a semicolon that is not a version suffix is kept.
    #[test]
    fn iso_names_have_version_suffix_stripped() {
        assert_eq!(decode_name(b"PLAYER.DAT;1", false), "PLAYER.DAT");
        assert_eq!(decode_name(b"NOTES;DRAFT", false), "NOTES;DRAFT");
    }

    /// Non-ISO bytes are rejected instead of being guessed at.
    #[test]
    fn rejects_non_iso_bytes() {
        let result = IsoImage::from_bytes(vec![0u8; SECTOR_SIZE * 20]);
        assert!(matches!(result, Err(DbError::InvalidIso(_))));
    }
}
