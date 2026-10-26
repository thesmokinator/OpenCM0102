<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Players and attributes

Status: **research in progress**. The data layout is largely
verified (see [dat-format.md](dat-format.md)); the *meaning* of
most fields is still open and nothing about it is fact until the
observation is recorded here.

## Coverage

- The 54 attributes: positional ratings and playing attributes —
  names and order verified, in-match effects open per attribute.
- Current and potential ability: scale, how CA derives from
  attributes, what PA gates, random potentials (-1/-2, verified
  present).
- Reputation: home/current/world (0–200, offsets verified), effects
  on transfers, awards and AI behaviour.
- Position learning and versatility (verified present as
  attributes; mechanics open).
- Training: schedules, coach effects, attribute movement.
- Development and ageing curves (peak ages, decline, retirement).
- Injuries: proneness (verified present), types, durations, match
  injuries, physio effects.
- Morale, form and condition during matches.
- Consistency and important matches (verified present; effects
  open).
- Hidden attributes, if any (to establish).

## Verified so far

- Attribute names and order, CA/PA offsets (PA may be -1/-2),
  reputation offsets, birth date, job codes 11/5 — all in
  [dat-format.md](dat-format.md), cross-checked on known people
  (Henry, Vieira, Bergkamp, Seaman, Wenger).

## Rules surface (editable targets)

Development parameters, ageing curves and injury rates should
become editable data with the 01/02 behaviour as default — see
"Rules as data" in [architecture.md](architecture.md).

## Open questions

- Every open item in Coverage, verified by observing the original
  game (attribute growth under training, age curves over seasons,
  injury frequencies, transfer reactions to reputation).
