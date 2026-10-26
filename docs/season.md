<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Season and management

Status: **research in progress**. Nothing about season behaviour is
fact until the observation is recorded here.

## Coverage

Every item below must be specified before the season simulation is
implemented:

- Per-nation calendar bounds and breaks (season start/end, winter
  breaks, pre-season).
- Board confidence, sackings, resignations and job
  applications/offers (including international management, if
  playable — to establish).
- Finances (income streams, wage bill, transfers accounting, debt
  handling, ticket and sponsor money).
- Non-playing staff hiring and effects (coaches, scouts, physios —
  only job codes 5 and 11 are identified so far).
- Youth intake and regens (when and how new players appear).
- Awards, records and history tracking.
- News, inbox and board messages flow.
- Holiday/continue mechanics (what a day tick processes, in order).

## Verified so far

- New games are assumed to start 1 July 2001 (current engine
  convention — to confirm per league against the original).
- Staff job codes 11 (player) and 5 (manager) are verified; other
  roles are open (see [dat-format.md](dat-format.md)).
- `staff_history.dat` tracks appearances and goals per
  staff/year/club. Nothing else seasonal is verified yet.

## Rules surface (editable targets)

Calendar dates, financial parameters and board strictness should
become editable data with the 01/02 behaviour as default — see
"Rules as data" in [architecture.md](architecture.md).

## Open questions

- Full per-item behaviour, verified by observing the original game
  across at least one full season (board messages, finances,
  intake dates, award winners).
