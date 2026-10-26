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
  notes land in `docs/save-format.md` before any parser ships, and
  tests use small synthetic fixtures, never original files.
- No decompiled or disassembled original code is used, per the
  clean-room rules in [idea.md](idea.md).
- Verification needs the original game: our saves must load in it,
  and its saves must reproduce the same squads and tables in our
  importer. The user runs those checks locally.
- Compatibility means resuming an old career with our engine, not
  bit-identical future results: simulation algorithms are a faithful
  reimplementation, not the original binaries.

## Original format sketch (STRING-EVIDENCE)

Recovered from the original's shipped UI and error text — field
order and meaning still need byte-level confirmation against real
saves:

- Container files use the `.mbr` extension (`game.mbr` for the
  career, `match.mbr` for match data). Layout, verified on a
  community save (patched exe, modern DB — offsets may differ on
  3.9.60): a short header opening with the version id, then a file
  table naming each embedded `.dat` with offset and size, then the
  payloads.
- The career header is versioned (`save_game_version` id) and
  holds, in order: save summary, unique game id, database size,
  date and time of day (plus last date/time), number of selected
  leagues and foreground leagues, default nation id, staff runtime
  data (with counter), club runtime data, and a club reputation
  table.
- Past the header, the save is a set of 33 per-system temp files:
  general, player, nonplayer, transfer, contract,
  squad_manager, scout_manager, finance, discipline,
  dispute, fine, awol, injury, training, tactics, match_stats,
  simulated_stats, stats, history, records, awards, fifa_rankings,
  media, news, notes, human_manager, man_conf, national_teams,
  fog, index, off, references, retire (all `*.dat`), each with
  its own load/save error reporting.
- Saving packs temporary files and moves the result into place
  atomically; temporary and replaced files are removed afterwards.
  Loading and saving run per-field progress reporting.

Blocked on: a set of user-supplied saves with known in-game state
(ideally a fresh career plus the same career a few days later, with
club, date and league position noted) for byte-level confirmation.
