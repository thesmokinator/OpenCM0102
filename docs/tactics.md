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
  sub caps, captaincy effects, slot legality, preset binary layout:
  needs playtest plus function-level analysis.
