<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# dat-format.md — CM 01/02 game-data formats

Status: **research in progress**. Nothing in this document may be treated
as fact until it has been observed on real user-supplied data and the
observation is recorded here with the file name, offset and sample values.

## Rules

- Observations come from the user's own ISO, inspected locally.
  No original file is ever committed to this repository; tests use small
  synthetic fixtures built from the layouts written here.
- A `.dat` parser may be added under `crates/cm0102-db/src/formats/` only
  for layouts documented in this file. Guessed layouts do not ship.

## Observed on the real ISO (23 Oct 2026)

- The image is an **installer disc** (ISO 9660 + Joliet volume `CM0102`).
  Its filesystem holds 44 files, among them only two `.dat` files:
  `lang.dat` (23,541 bytes) and `os.dat` (450 bytes) — installer data,
  not game data.
- The game payload is inside `data1.cab` (275,209,381 bytes), an
  InstallShield cabinet, next to `data1.hdr` (85,952 bytes).
- Joliet file names on this disc keep the `;1` version suffix (e.g.
  `lang.dat;1`); the ISO reader strips it after decoding.

## InstallShield cabinet (implemented, verified 23 Oct 2026)

Reader: `crates/cm0102-db/src/installshield.rs`, layout from the
structures documented by the unshield project (MIT), implemented
independently in Rust. Version 5 layout only:

- Common header (20 bytes) in both files: signature `ISc(`, cabinet
  descriptor offset at byte 12.
- Cabinet descriptor (at that offset, in `data1.hdr`): file-table offset
  `+0x0c`, directory count `+0x1c`, file count `+0x28`, file-group
  offset-list slots from `+0x3e` (71 slots, chained).
- File table: `directory_count + file_count` little-endian `u32` offsets
  relative to the table base; file descriptors are 58 bytes: name offset
  `+0`, flags `+8` (4 = compressed, 8 = invalid), expanded size `+10`,
  compressed size `+14`, data offset `+46`. Names are NUL-terminated,
  relative to the table base.
- File groups give the install path prefix (e.g. `CM3 Data`): group
  descriptor name offset `+0`, first/last file index at `+0x4c`/`+0x50`.
- Compressed payloads in `data1.cab` are chunked raw DEFLATE: a `u16`
  little-endian chunk length, then that many bytes inflating to at most
  64 KiB, repeated to the compressed size.
- Verified against the `unshield` tool on the real ISO: the 20 `.dat`
  files under `CM3 Data/` list identically and extract byte-identically.

## Record layouts (implemented, verified 23 Oct 2026)

Parser: `crates/cm0102-db/src/formats.rs`, derived empirically from the
real data and cross-checked against known entities. Counts below are
3.9.60; 3.9.68 deltas have their own section at the end.

- Name files (`first_names.dat`, `second_names.dat`,
  `common_names.dat`): 60-byte records. Name in bytes 0–50 (NUL padded),
  record id `i32` at 51, another `i32` at 55, usage counter at 59. The
  record index is the id other files reference
  (34,363 first names, 82,338 second names, 8,493 common names).
- `club.dat`: 581-byte records, 10,580 clubs. Club id `i32` at 0, long
  name at 4 (52-byte field), short name at 56.
- `club.dat` league membership (verified 26 Oct 2026 by clustering
  all 10,580 records per field): current-season league id `u8` at
  byte 87, previous-season (00/01) league id `u8` at byte 91. Values
  equal `club_comp.dat` ids for the major leagues: byte 87 = 7 lists
  exactly the twenty 01/02 Premier League clubs, = 8 the
  twenty-four 01/02 First Division clubs, = 24 the eighteen 01/02
  Serie A clubs. Promotion/relegation cross-checks hold both ways:
  Blackburn/Bolton/Fulham read 87 = 7 / 91 = 8, Manchester
  City/Coventry/Bradford read 87 = 8 / 91 = 7, and Chievo/Piacenza/
  Torino/Venezia read 87 = 24 / 91 = 25. Small clubs carry bucket
  values (58/98/101/254, …) or spill under real league ids — full
  distribution in [competitions.md](competitions.md); the assignment
  rule is still open. Neighbor bytes 86/90 are
  separate 0/255 `u8` flags, not part of the league field.
- `staff.dat`, staff section: 157-byte packed records. The section is
  the run of records whose staff id `i32` at byte 0 equals the record
  index; on the real file it is 132,722 records long and the parser
  stops where the sequence breaks. First-name id at 4, second-name id
  at 8, common-name id at 12 (when positive it replaces the first name
  for display), club id `i32` at byte 57 (unaligned; negative when
  unattached).
- Date of birth in the staff record: zero-based day of year `u16` at
  byte 16, year `i16` at byte 18. Verified on Thierry Henry
  (228/1977 = 17 Aug 1977), Patrick Vieira (173/1976 = 22 Jun 1976;
  the data is one day off his real 23 Jun birthday),
  Dennis Bergkamp (129/1969 = 10 May 1969), David Seaman
  (261/1963 = 19 Sep 1963) and Tony Adams (282/1966 = 10 Oct 1966).
- Job code at byte 61: 11 = player (99% of playing records), 5 =
  manager (verified on Arsène Wenger and Gérard Houllier). Other codes
  are non-playing staff roles and are not labelled until individually
  verified. `is_player` is exactly `job == 11`.
- After the person section, `staff.dat` continues with a non-player
  section and a player section (record layouts cross-checked against
  the open-source TransferTool editor's published structures, then
  verified byte-by-byte on known people). Section sizes follow from
  the classification byte: 23,785 non-player records (classification
  1 or 3) of 68 bytes, then 109,940 player records (classification
  2 or 3) of 70 bytes — together they account for every remaining
  byte of the real file exactly. The k-th person of a classification
  owns the k-th record of the matching section.
- Non-player record (68 bytes): current/potential ability `i16` at
  4/6, home/current/world reputation `i16` at 8/10/12.
- Player record (70 bytes): squad number at 4, current/potential
  ability `i16` at 5/7 (PA may be -1/-2 for random potential),
  home/current/world reputation `i16` at 9/11/13, and 54 attributes
  at bytes 15..69 in the order of `ATTRIBUTE_NAMES` in `cm0102-core`
  (positional ratings first, then playing attributes).
- Verified on the real ISO: Thierry Henry CA 175 / PA 183, squad
  number 14, Attacker 20; Patrick Vieira 189/195, number 4;
  Dennis Bergkamp 165/185, number 10; David Seaman 163/190, number 1,
  Goalkeeper 20; Arsène Wenger (non-player section) 190/200.
- Verified end to end from the real ISO: 10,580 clubs imported,
  Arsenal (id 676) lists 63 staff/players including Thierry Henry,
  Patrick Vieira, Dennis Bergkamp and David Seaman.

## Remaining tables (implemented, verified 23 Oct 2026)

All parsed in `crates/cm0102-db/src/formats.rs` and imported by
`import_game_data`; record layouts cross-checked against the
TransferTool structures and known entities on the real file:

- `nation.dat`: 290-byte records, 213 nations. Name at 4, short name
  at 56, three-letter code at 83, continent id at 113 (Italy = ITA,
  continent 2 = Europe).
- `city.dat`: 56-byte records, 5,418 cities. Name at 4, nation id
  at 31.
- `stadium.dat`: 78-byte records, 7,096 stadiums. Name at 4, city id
  at 56, capacity/seating/expansion at 60/64/68 (Giuseppe Meazza:
  85,443).
- `continent.dat`: 198-byte records, 6 continents. Name at 4,
  three-letter code at 31, federation name at 61.
- `colour.dat`: 58-byte records, 34 colours. Name at 4, RGB at
  55/56/57.
- `club_comp.dat` + `nation_comp.dat`: 107-byte records sharing one
  id space (390 + 19 = 409 competitions). Name at 4, short name at
  56, code at 83, continent at 89, nation at 93, reputation at 105.
- `staff_comp.dat`: 101-byte records, 542 staff competitions.
- `officials.dat`: 43-byte records, 3,124 officials. Name ids at
  4/8, nation at 22, city at 26, abilities/reputation at 30/32/34.
- `club_comp_history.dat` + `nation_comp_history.dat`: 26-byte
  records sharing one id space (7,194 + 143). Competition at 4,
  year at 8, winners/runners-up/third/hosts clubs at 10/14/18/22.
- `staff_history.dat`: 17-byte records, 270,654 rows. Staff at 4,
  year at 8, club at 10, loan flag at 14, apps/goals at 15/16.
- `staff_comp_history.dat`: 58-byte records, 1,998 rows. Competition
  at 4, year at 8, placed staff ids at 18/34/50.

`nat_club.dat` and `index.dat` are not imported as tables: the first
is an index by-product and the second ships runtime pointers rather
than data in this version; both remain visible in the imported file
inventory (`source_files`).

## 3.9.68 data deltas (VERIFIED-DATA, patched bytes)

The patch ships full replacements for the database files below; all
record sizes are unchanged (verified by exact section accounting,
e.g. staff sections sum to the byte). Counts 3.9.60 → 3.9.68:

| File | 3.9.60 | 3.9.68 |
|---|---|---|
| `club.dat` | 10,580 | 13,214 |
| `staff.dat` persons | 132,722 | 169,342 |
| player records | 109,940 | 139,769 |
| non-player records | 23,785 | 30,865 |
| `city.dat` | 5,418 | 6,682 |
| `stadium.dat` | 7,096 | 8,917 |
| `officials.dat` | 3,124 | 4,567 |
| `first_names.dat` | 34,363 | 44,281 |
| `second_names.dat` | 82,338 | 104,395 |
| `common_names.dat` | 8,493 | 13,256 |
| `club_comp.dat` | 390 | 433 |
| `staff_comp.dat` | 542 | 543 |
| `*.lng` (×10) | 1,621,488 B | 1,993,486 B |
| `language.ldb` | 34,562,266 B | 204,672,142 B |
| `club_comp_history.dat` rows | 7,194 | 10,766 |
| `nation_comp_history.dat` rows | 143 | 273 |
| `staff_history.dat` rows | 270,654 | 385,846 |
| `staff_comp_history.dat` rows | 1,998 | 2,609 |

Structural notes:

- Record ids are positional, not stable across data versions: e.g.
  West Ham United is club 10,224 in 3.9.60 but 12,765 in 3.9.68.
  Importers must resolve by name, never hardcode ids.
- Taiwan is renamed to Chinese Taipei and moves from nation slot 183
  to slot 43; nations 43–182 shift up by one and every nation
  reference (competitions, cities, staff) is renumbered to match.
- `club_comp.dat` grows by 43 Asian competitions and repurposes
  slots 364–389 (Arab cups move to 373–377); see
  [competitions.md](competitions.md).
- `nation_comp.dat` is reordered ('B' and Under-21 internationals
  move to the front) with the same 19 entries.
- Not shipped by the patch (3.9.60 bytes stay current):
  `events_*.cfg`, `euro.cfg`, `fancluburls.txt`, `pundit.cfg`,
  `si.rgn`, `Cpuinf32.dll`, `game.mbr`/`match.mbr` templates
  (verified byte-identical in the patched install).
- `language.ldb` grows ~6× (34.5 → 204.6 MB): it is the
  multilingual text bank (~9,807 templates × 10 languages);
  record layout still open.
- Default tactics ship as `Data/*.pct` (one file per formation
  variant, e.g. `442_default.pct`); format still open — see
  [tactics.md](tactics.md).
