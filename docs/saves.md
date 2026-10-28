<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Saves

Status: **native saves ship; original-format compatibility is planned**.

## Native saves (shipped)

New careers are stored as versioned JSON (`SavedCareer` envelope in
`cm0102-core`) under `saves/` in the user data directory
(`OpenCM0102/saves/<name>.json`). Readers accept only the envelope
version they know; unknown versions are rejected with an error instead
of being silently misread.

This format is ours and interim: it exists so the original game's
new-career flow (create manager, nationality, club, save, start) works
end to end today.

## Original save compatibility (planned, bidirectional)

Goal: load a save written by the original CM 01/02 into OpenCM0102,
and write saves the original game accepts. The format target is
v3.9.68 (see [versions.md](versions.md)): most saves in circulation
are that version. Ground rules:

- The format is recovered **empirically from user-supplied save
  files** (same method as [dat-format.md](dat-format.md)): layout
  notes land in this file before any parser ships, and tests use
  small synthetic fixtures, never original files.
- No decompiled or disassembled original code is used, per the
  clean-room rules in [idea.md](idea.md).
- Verification needs the original game: our saves must load in it,
  and its saves must reproduce the same squads and tables in our
  importer. The user runs those checks locally.
- Compatibility means resuming an old career with our engine, not
  bit-identical future results: simulation algorithms are a faithful
  reimplementation, not the original binaries.

## Container reader (implemented)

`cm0102-core` `orig_save` parses the file table (268-byte entries:
five `u32` values with offset and size last, then the 248-byte
name), carves payloads strictly by offset and size, decodes the
byte RLE, chunks the finance (359-byte), discipline (29-byte) and
injury (31-byte) record arrays, and reads the documented
`general.dat` anchors. Entry order was cross-checked by carving the
`continent.dat` payload of a second sample (six 198-byte records,
six continent names). Raw-vs-packed selection per entry and the
remaining payload layouts stay open.

## Original format (VERIFIED-DATA, 3.9.60 sample)

Container verified byte-exact on a user-supplied 3.9.60 save (109
payloads unpacked, 57 cross-checked against the game's own unpacked
temp files):

- The `.sav` opens with a file table: 268-byte entries holding
  five `u32` values (offset and expanded size come last) plus a
  248-byte NUL-padded file name. Payloads follow contiguously.
- Payloads are compressed with a byte RLE: `0x80` is a literal
  `0x80` byte, bytes above `0x80` repeat the next byte
  (`b & 0x7F`) times, bytes below are literal. Table sizes are the
  expanded sizes. (A modern patched-DB save instead stores some
  payloads raw with non-monotonic offsets — carve strictly by
  each entry's own offset and size; its first-entry flag reads 3
  where the 3.9.60 sample reads 4.)
- Payloads are the database actually playing (a reduced,
  save-local selection — 3,532 clubs in the sample — with ids
  renumbered from the ISO set, localized at new-game creation:
  names are rewritten in the active language, e.g. `dell'Africa`),
  the ten `.lng` files, per-system runtime files (`general`,
  `player`, `nonplayer`, `transfer`, `contract`, `finance`,
  `discipline`, `injury`, `tactics`, `news`, `media`, `notes`,
  `human_manager`, `man_conf`, `national_teams`, `fog`, `index`,
  `off`, `retire`, …), one state file per active competition
  (`<name>_<id>.tmp`, e.g. `Serie A Italiana_24.tmp`), plus
  `fixtures_<year>.tmp`, stats, history and seeding files.
- During play the game works on these files unpacked in its own
  directory; saving packs them and moves the result into place
  atomically. Loading and saving run per-field progress reporting.
- A modern patched-database save (community sample, not the
  3.9.60 baseline) holds 317 table entries under the same
  268-byte entry format; its first entry carries the values
  3 / 22 / 317, the last matching the entry count. Its runtime
  sections add per-year fixture and transfer files and a
  69,000,000-byte match-stats ring (3,450 bytes times 20,000
  slots), plus UEFA/CONMEBOL seeding state files. Some of its
  payloads are stored raw instead of RLE-packed (tiny sections
  such as notes and fog still decode exactly; larger runtime
  sections do not), so the per-entry compression flag still
  needs locating before those payloads can be carved reliably.

Known payload layouts so far: the finance file is a flat
array of 359-byte club records (load/save pair verified);
manager confidence is variable-length per manager (main block
plus dated sub-records); discipline entries are 29-byte
records (20/8/10-byte blocks with 16-bit counts) and injury
entries are 31-byte records (48/22/17-byte injury records
with 32-bit counts), both with load/save pairs verified.
Still open: other per-payload record layouts (competition state,
fixtures, most runtime files), most career header fields
(`save_game_version` id, game id, dates, league counts — known
from shipped UI text, order unconfirmed), and a 3.9.68 sample to
confirm the target version byte-identically. Known `general.dat`
anchors in the sample: default nation at offset 4, save version
at 2004, manager name at 2024, club name at 2084. A fresh career
plus the same career a few days later (club, date, league
position noted) would pin the runtime layouts down. Known:
`player.dat`/`nonplayer.dat` keep the 70/68-byte database record
sizes (16,562 + 3,210 rows in the sample).
