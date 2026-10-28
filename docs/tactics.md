<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Tactics

Status: **research in progress**. Nothing about tactical behaviour is
fact until the observation is recorded here.

## Coverage

Every item below must be specified before the tactics model and its
match-engine effects are implemented:

- Formations and position slots (which formations exist, slot
  legality, position abbreviations).
- Player instructions (the full instruction set and what each one
  changes on the pitch).
- Team instructions (mentality, tempo, width and the rest, with
  their effects).
- Set pieces (takers, routines, defensive setups).
- Captaincy and leadership effects.
- Substitution rules and lineup constraints (allowed changes,
  minimum players, suspended/injured handling).
- Match-day squad selection (match squad size, reserves use).

## Verified so far

- The 12 positional ratings exist in fixed order at the head of the
  54 player attributes (Goalkeeper through Free Role — see
  [dat-format.md](dat-format.md)). Their in-match meaning is open.
- Player records carry a squad number.
- Formation catalog (STRING-EVIDENCE, exe-string inventory lines
  28082–28090): `3-4-3`, `4-5-1`, `4-1-2-1-2`, `3-5-2`, `4-2-4`,
  `4-3-3`, `4-4-2`, `5-3-2`, `5-3-2 Sweeper`, in this list order
  under "Preferred Formation". Style-qualified variants appear in
  news text (`3-5-2 Attacking` line 23702, `5-3-2 Sweeper` line
  27001, `a 4-4-2` line 26992). Slot layouts per formation are not
  evidenced and stay open.
- Team instruction set (STRING-EVIDENCE): playmaker (line 28702),
  men behind ball (28707), counter attack (28708), offside trap
  preference (28070), corner/free-kick takers, penalty takers
  (28679), mentality attacking/defensive/normal. Effects open.

## Implemented model

`Formation`, `Lineup` (11 distinct starters, best-eleven
auto-pick), `PlayerInstructions` (only rows with observed option
sets: marking, set-piece roles, pass direction),
`TeamInstructions` and `TeamTactics` live in `cm0102-core`
`tactics`. `cm0102-core` `preset` reads the shipped `.pct` files
structurally: observed sizes (1,428/1,432/1,476) with the
size-linked header word, the plain-text author at offset 54, slot
validation 0–10 and the five-high-bits masking. Per-byte
instruction semantics and the preset-to-formation mapping stay
open. The match engine reads lineup strength (average current
ability) through `MatchContext`; all instruction effects are
explicitly unmodelled until observed.

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, effects still need
playtest or function-level analysis).

- Player instruction grid has 14 rows: marking, cross ball, through
  balls, long shots, hold-up ball, run with ball, forward runs,
  free role, defensive and attacking set pieces, pass to, pressing,
  tackling, passing. (STRING-EVIDENCE)
- Enumerated option sets: marking (man, man-to-player, zonal,
  none), set pieces (forward/back/normal × attack/defence), pass to
  (right/centre/left/all), pressing, tackling and passing (each
  with team-default inheritance). (STRING-EVIDENCE)
- Team level: playmaker, left/right corner and free-kick takers,
  men behind ball, counter attack, offside trap, mentality
  (attacking/defensive/normal). No throw-in taker exists.
  (STRING-EVIDENCE)
- No tempo, width, creative-freedom, time-wasting or defensive-line
  instructions exist (strong negative). (STRING-EVIDENCE)
- Formations ship as named presets with a 12-field file format;
  arrows are per-player runs in with-ball / without-ball views.
  (STRING-EVIDENCE)
- Preset files (`Data/*.pct`) are 1,476 bytes with an author field
  (e.g. `Marc Vaughan`) and per-player instruction bytes: two
  presets differ in ~250 mostly single bytes spread over the
  player section, team settings shared. Field semantics still
  open. (VERIFIED-DATA)
- Lineups: exactly 11 outfield players, competition squad caps,
  substitutes from the match squad, takers must be in the eleven
  and stay on the pitch, automatic captain fallback, and a
  10-class unavailability catalog (permits, injuries, bans, loans,
  cups, deadlines, trials). (STRING-EVIDENCE)
- AI managers have preferred styles and formations; reserves keep
  separate tactics. (STRING-EVIDENCE)

## Rules surface (editable targets)

Formation presets and file format, instruction rows and option
enums, team settings, lineup constraints and eligibility should all
become editable data with the 01/02 behaviour as default — see
"Rules as data" in [architecture.md](architecture.md).

## Open questions

- Toggle-instruction values, rating-to-slot mapping, per-competition
  sub caps, captaincy effects, slot legality, preset byte semantics:
  needs playtest plus function-level analysis.
- Function-level status: formation player bytes pack five high
  bits per player (masked reads by side type), slot accessors
  validate indices 0–10 (eleven slots), and preset file names
  follow the `<shape>[_attacking|_defensive]_default.pct`
  catalogue. Across the 40 shipped presets, files are 1,428,
  1,432 or 1,476 bytes with a size-linked header word and a
  plain-text author at offset 54; an attacking variant differs
  from its default in only 51 bytes, concentrated in the
  instruction blocks. Per-byte instruction semantics remain
  open. Finance-side fee computation (club
  finance module) uses value-thirds with per-thousand uplifts
  and 500-rounding — see [transfers.md](transfers.md).
  (Function-level analysis.)
