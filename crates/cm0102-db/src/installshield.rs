// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! InstallShield cabinet reader (`data1.cab` + `data1.hdr`), version 5
//! layout — the format used by the CM 01/02 installer disc.
//!
//! The game payload of the ISO lives inside this cabinet, not on the ISO
//! filesystem. Structures and offsets follow the layout documented by the
//! unshield project (MIT); this is an independent Rust implementation.
//! Reading is in-memory and read-only: list the cabinet directory from
//! the header file, then extract single files from the cabinet on demand.
//! Compressed payloads are chunked raw-DEFLATE blocks, each introduced by
//! a little-endian `u16` compressed length, exactly as unshield reads
//! them.
//!
//! Only the version 5 descriptor layout observed on the real disc is
//! supported; other layouts fail with an explicit error instead of a
//! guess. Split (multi-volume) and obfuscated entries are rejected for
//! the same reason.

use std::io::Read;

use crate::{DbError, Result};

/// `ISc(` little-endian signature shared by `.cab` and `.hdr` files.
const CAB_SIGNATURE: u32 = 0x2863_5349;
/// File descriptor flag: payload is stored compressed.
const FILE_COMPRESSED: u16 = 0x0004;
/// File descriptor flag: entry must be ignored.
const FILE_INVALID: u16 = 0x0008;
/// File descriptor flag: payload is obfuscated (unsupported here).
const FILE_OBFUSCATED: u16 = 0x0002;
/// Maximum file-group slots in the cabinet descriptor.
const MAX_GROUPS: usize = 71;

/// One file stored in the cabinet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CabFile {
    /// Full path inside the cabinet, `/`-separated, e.g.
    /// `CM3 Data/club.dat` (file group + file name).
    pub path: String,
    /// Bare file name, e.g. `club.dat`.
    pub name: String,
    /// Size after decompression.
    pub expanded_size: u64,
    /// Size stored in the cabinet.
    pub compressed_size: u64,
    /// Byte offset of the payload inside `data1.cab`.
    data_offset: u64,
    /// Descriptor flags.
    flags: u16,
}

impl CabFile {
    /// Whether this entry is a `.dat` game-data file.
    pub fn is_dat_file(&self) -> bool {
        self.name
            .rsplit('.')
            .next()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dat"))
    }
}

/// A parsed cabinet directory (from `data1.hdr`) plus extraction.
#[derive(Debug)]
pub struct InstallShieldCab {
    files: Vec<CabFile>,
}

impl InstallShieldCab {
    /// Parse the cabinet directory from the raw `data1.hdr` bytes.
    pub fn parse_header(header: &[u8]) -> Result<Self> {
        let reader = HeaderReader { data: header };
        if reader.u32(0)? != CAB_SIGNATURE {
            return Err(invalid("missing ISc( signature in header file"));
        }
        let descriptor_offset = reader.u32(12)? as usize;
        let file_table_offset = reader.u32(descriptor_offset + 0x0c)? as usize;
        let directory_count = reader.u32(descriptor_offset + 0x1c)? as usize;
        let file_count = reader.u32(descriptor_offset + 0x28)? as usize;
        let table_base = descriptor_offset + file_table_offset;

        let groups = reader.file_groups(descriptor_offset)?;
        let mut files = Vec::new();
        for index in 0..file_count {
            let entry_offset = reader.u32(table_base + (directory_count + index) * 4)? as usize;
            let entry = table_base + entry_offset;
            let name_offset = reader.u32(entry)? as usize;
            let flags = reader.u16(entry + 8)?;
            if flags & FILE_INVALID != 0 {
                continue;
            }
            let name = reader.string_at(table_base + name_offset)?;
            if name.is_empty() {
                continue;
            }
            let expanded_size = u64::from(reader.u32(entry + 10)?);
            let compressed_size = u64::from(reader.u32(entry + 14)?);
            let data_offset = u64::from(reader.u32(entry + 10 + 8 + 0x14)?);
            let group = groups
                .iter()
                .find(|group| group.first_file <= index as i64 && index as i64 <= group.last_file)
                .map(|group| group.name.as_str())
                .unwrap_or("");
            let path = if group.is_empty() {
                name.clone()
            } else {
                format!("{group}/{name}")
            };
            files.push(CabFile {
                path,
                name,
                expanded_size,
                compressed_size,
                data_offset,
                flags,
            });
        }
        if files.is_empty() {
            return Err(invalid("cabinet contains no readable files"));
        }
        Ok(Self { files })
    }

    /// Every readable file in the cabinet, in descriptor order.
    pub fn files(&self) -> &[CabFile] {
        &self.files
    }

    /// Extract one file (by full path or bare name) from `data1.cab`.
    pub fn extract(&self, cabinet: &[u8], path: &str) -> Result<Vec<u8>> {
        let file = self
            .files
            .iter()
            .find(|file| {
                file.path.eq_ignore_ascii_case(path) || file.name.eq_ignore_ascii_case(path)
            })
            .ok_or_else(|| invalid(&format!("file not found in cabinet: {path}")))?;
        if file.flags & FILE_OBFUSCATED != 0 {
            return Err(invalid(&format!(
                "obfuscated cabinet entries are not supported: {path}"
            )));
        }
        let start = file.data_offset as usize;
        let stored_len = if file.flags & FILE_COMPRESSED != 0 {
            file.compressed_size
        } else {
            file.expanded_size
        } as usize;
        let stored = cabinet
            .get(
                start
                    ..start
                        .checked_add(stored_len)
                        .ok_or_else(|| invalid("offset overflow"))?,
            )
            .ok_or_else(|| invalid(&format!("payload extends past cabinet end: {path}")))?;

        let expanded = if file.flags & FILE_COMPRESSED != 0 {
            decompress_chunks(stored)?
        } else {
            stored.to_vec()
        };
        if expanded.len() as u64 != file.expanded_size {
            return Err(invalid(&format!(
                "extracted size mismatch for {path}: expected {}, got {}",
                file.expanded_size,
                expanded.len()
            )));
        }
        Ok(expanded)
    }
}

/// Decompress the chunked raw-DEFLATE payload of one compressed file.
///
/// Each chunk is a `u16` little-endian compressed length followed by that
/// many raw-DEFLATE bytes expanding to at most 64 KiB.
fn decompress_chunks(stored: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut pos = 0usize;
    while pos < stored.len() {
        if pos + 2 > stored.len() {
            return Err(invalid("truncated compressed chunk header"));
        }
        let chunk_len = u16::from_le_bytes([stored[pos], stored[pos + 1]]) as usize;
        pos += 2;
        if chunk_len == 0 {
            return Err(invalid(
                "zero-length compressed chunk (old compression variant?)",
            ));
        }
        let chunk = stored
            .get(pos..pos + chunk_len)
            .ok_or_else(|| invalid("truncated compressed chunk"))?;
        let mut decoder = flate2::read::DeflateDecoder::new(chunk);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .map_err(|error| DbError::InvalidIso(format!("deflate failed: {error}")))?;
        output.extend_from_slice(&decoded);
        pos += chunk_len;
    }
    Ok(output)
}

/// One InstallShield file group (an install directory bucket).
#[derive(Debug)]
struct FileGroup {
    name: String,
    first_file: i64,
    last_file: i64,
}

/// Bounds-checked little-endian reader over the header bytes.
struct HeaderReader<'a> {
    data: &'a [u8],
}

impl HeaderReader<'_> {
    fn bytes(&self, offset: usize, len: usize) -> Result<&[u8]> {
        self.data
            .get(
                offset
                    ..offset
                        .checked_add(len)
                        .ok_or_else(|| invalid("offset overflow"))?,
            )
            .ok_or_else(|| invalid("header extends past end of file"))
    }

    fn u32(&self, offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.bytes(offset, 4)?.try_into().unwrap(),
        ))
    }

    fn u16(&self, offset: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.bytes(offset, 2)?.try_into().unwrap(),
        ))
    }

    fn i32(&self, offset: usize) -> Result<i64> {
        Ok(i64::from(self.u32(offset)? as i32))
    }

    /// Read a NUL-terminated name. Cabinet names predate UTF-8; bytes are
    /// mapped through Latin-1, which covers the observed file names.
    fn string_at(&self, offset: usize) -> Result<String> {
        let slice = self
            .data
            .get(offset..)
            .ok_or_else(|| invalid("string extends past end of header"))?;
        let end = slice
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(slice.len());
        Ok(slice[..end].iter().map(|byte| *byte as char).collect())
    }

    /// Collect file groups from the descriptor's offset-list slots,
    /// following each slot's chain of linked entries.
    fn file_groups(&self, descriptor_offset: usize) -> Result<Vec<FileGroup>> {
        let mut groups = Vec::new();
        for slot in 0..MAX_GROUPS {
            let mut offset = self.u32(descriptor_offset + 0x3e + slot * 4)? as usize;
            while offset != 0 {
                let entry = descriptor_offset + offset;
                let group_offset = self.u32(entry + 4)? as usize;
                let next_offset = self.u32(entry + 8)? as usize;
                let group = descriptor_offset + group_offset;
                let group_name_offset = self.u32(group)? as usize;
                groups.push(FileGroup {
                    name: self.string_at(descriptor_offset + group_name_offset)?,
                    first_file: self.i32(group + 4 + 0x48)?,
                    last_file: self.i32(group + 4 + 0x48 + 4)?,
                });
                offset = next_offset;
            }
        }
        Ok(groups)
    }
}

fn invalid(message: &str) -> DbError {
    DbError::InvalidIso(format!("InstallShield cabinet: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a minimal version 5 header + cabinet with two files:
    /// `one.dat` stored raw and `two.dat` DEFLATE-compressed, both in a
    /// single file group named `CM3 Data`.
    fn synthetic_cabinet() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
        // Compressed payload for file 2: one chunk.
        let plain_two = b"compressed-two".to_vec();
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&plain_two).unwrap();
        let deflated = encoder.finish().unwrap();
        let mut payload_two = (deflated.len() as u16).to_le_bytes().to_vec();
        payload_two.extend_from_slice(&deflated);

        let plain_one = b"stored-one".to_vec();
        let data_offset_one = 0u32;
        let data_offset_two = plain_one.len() as u32;
        let mut cabinet = plain_one.clone();
        cabinet.extend_from_slice(&payload_two);

        // Header layout mirrors the parser: common header, descriptor at
        // 512, file table after it, descriptors and strings in the table.
        let descriptor_offset = 512usize;
        let file_table_offset = 0x400usize;
        let table_base = descriptor_offset + file_table_offset;
        let mut header = vec![0u8; 8192];
        let put_u32 = |header: &mut Vec<u8>, offset: usize, value: u32| {
            header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        };
        let put_str = |header: &mut Vec<u8>, offset: usize, text: &str| {
            header[offset..offset + text.len()].copy_from_slice(text.as_bytes());
        };

        put_u32(&mut header, 0, CAB_SIGNATURE);
        put_u32(&mut header, 12, descriptor_offset as u32);
        put_u32(
            &mut header,
            descriptor_offset + 0x0c,
            file_table_offset as u32,
        );
        put_u32(&mut header, descriptor_offset + 0x1c, 0); // directory count
        put_u32(&mut header, descriptor_offset + 0x28, 2); // file count

        // File table: two descriptor offsets (relative to table base).
        let descriptor_one = 0x100usize;
        let descriptor_two = 0x140usize;
        put_u32(&mut header, table_base, descriptor_one as u32);
        put_u32(&mut header, table_base + 4, descriptor_two as u32);

        // Names live right after the descriptors.
        let name_one = 0x200usize;
        let name_two = 0x220usize;
        put_str(&mut header, table_base + name_one, "one.dat");
        put_str(&mut header, table_base + name_two, "two.dat");

        // Descriptor 1: stored. Descriptor 2: compressed (flag 4).
        let write_descriptor = |header: &mut Vec<u8>,
                                at: usize,
                                name: usize,
                                flags: u16,
                                expanded: u32,
                                compressed: u32,
                                data_offset: u32| {
            put_u32(header, at, name as u32);
            header[at + 8..at + 10].copy_from_slice(&flags.to_le_bytes());
            put_u32(header, at + 10, expanded);
            put_u32(header, at + 14, compressed);
            put_u32(header, at + 10 + 8 + 0x14, data_offset);
        };
        write_descriptor(
            &mut header,
            table_base + descriptor_one,
            name_one,
            0,
            plain_one.len() as u32,
            plain_one.len() as u32,
            data_offset_one,
        );
        write_descriptor(
            &mut header,
            table_base + descriptor_two,
            name_two,
            FILE_COMPRESSED,
            plain_two.len() as u32,
            payload_two.len() as u32,
            data_offset_two,
        );

        // One file group covering files 0..=1, chained from slot 0.
        let list_offset = 0x200usize; // relative to descriptor, past the slot array
        let group_offset = 0x240usize;
        let group_name = 0x280usize; // relative to descriptor
        put_str(&mut header, descriptor_offset + group_name, "CM3 Data");
        put_u32(&mut header, descriptor_offset + 0x3e, list_offset as u32);
        put_u32(
            &mut header,
            descriptor_offset + list_offset,
            group_name as u32,
        );
        put_u32(
            &mut header,
            descriptor_offset + list_offset + 4,
            group_offset as u32,
        );
        put_u32(&mut header, descriptor_offset + list_offset + 8, 0);
        put_u32(
            &mut header,
            descriptor_offset + group_offset,
            group_name as u32,
        );
        put_u32(&mut header, descriptor_offset + group_offset + 4 + 0x48, 0);
        put_u32(
            &mut header,
            descriptor_offset + group_offset + 4 + 0x48 + 4,
            1,
        );

        (header, cabinet, plain_one, plain_two)
    }

    /// Listing resolves names, groups, sizes and `.dat` detection.
    #[test]
    fn lists_synthetic_cabinet() {
        let (header, _cabinet, _one, _two) = synthetic_cabinet();
        let cab = InstallShieldCab::parse_header(&header).unwrap();
        assert_eq!(cab.files().len(), 2);
        assert_eq!(cab.files()[0].path, "CM3 Data/one.dat");
        assert_eq!(cab.files()[1].path, "CM3 Data/two.dat");
        assert!(cab.files().iter().all(|file| file.is_dat_file()));
    }

    /// Stored and compressed payloads both round-trip.
    #[test]
    fn extracts_stored_and_compressed() {
        let (header, cabinet, plain_one, plain_two) = synthetic_cabinet();
        let cab = InstallShieldCab::parse_header(&header).unwrap();
        assert_eq!(
            cab.extract(&cabinet, "CM3 Data/one.dat").unwrap(),
            plain_one
        );
        assert_eq!(cab.extract(&cabinet, "two.dat").unwrap(), plain_two);
    }

    /// A bad signature is rejected, not guessed at.
    #[test]
    fn rejects_bad_signature() {
        let result = InstallShieldCab::parse_header(&[0u8; 1024]);
        assert!(matches!(result, Err(DbError::InvalidIso(_))));
    }
}
