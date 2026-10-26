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
- Player records carry a squad number. Nothing else tactical is
  verified yet.

## Rules surface (editable targets)

Formation definitions, instruction sets and substitution/lineup
constraints should become editable data with the 01/02 behaviour as
default — see "Rules as data" in [architecture.md](architecture.md).

## Open questions

- Full instruction lists and their observable effects, verified by
  observing the original game (same fixture under different
  instructions, over many runs).
