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
and write saves the original game accepts. Ground rules:

- The format is recovered **empirically from user-supplied save
  files** (same method as [dat-format.md](dat-format.md)): layout
  notes land in `docs/SAVE_FORMAT.md` before any parser ships, and
  tests use small synthetic fixtures, never original files.
- No decompiled or disassembled original code is used, per the
  clean-room rules in [idea.md](idea.md).
- Verification needs the original game: our saves must load in it,
  and its saves must reproduce the same squads and tables in our
  importer. The user runs those checks locally.
- Compatibility means resuming an old career with our engine, not
  bit-identical future results: simulation algorithms are a faithful
  reimplementation, not the original binaries.

Blocked on: a set of user-supplied saves with known in-game state
(ideally a fresh career plus the same career a few days later, with
club, date and league position noted).
