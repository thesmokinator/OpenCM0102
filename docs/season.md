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

- New games start 1 July 2001 (engine convention, consistent with
  the 1–31 July pre-season window).
- Staff job codes 11 (player) and 5 (manager) are verified; 17
  codes exist in total, the rest unlabeled (see
  [dat-format.md](dat-format.md)).
- `staff_history.dat` spans 1984–2001 with 270,654 rows; reference
  tables hold 3,124 officials, 7,096 stadiums, 5,418 cities,
  6 continents.
- `player_setup.cfg` scripts season-start events: retirements
  (`"RETIREMENT" ... "Manchester United" 23 7 2002`),
  international retirements, injuries (`"INJURY" ...
  "PERMANENT_HIP_INJURY" 15`), manager confidences, plus rare
  doping (`NANDROLONE`), false-passport and misconduct entries.
  The 02/03 update trims most 01/02-specific lines (injuries
  534→184, international retirements 147→49). (VERIFIED-DATA,
  config text)
- International 'B' and Under-21 management requires the nation to
  have over 400 active players in the database and is optional via
  'control all teams' (per the shipped SI release notes).

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, formulas still need
playtest or function-level analysis).

- Board confidence is a graded message ladder driven by form
  streaks, ending in sacking with enumerated causes (results,
  league position, supporter pressure, expensive squad, staff
  relations); boards publish season expectations.
  (STRING-EVIDENCE)
- Club, international and under-21 jobs are playable; vacancies are
  engine objects with apply/acknowledge/dismiss flow; managers can
  resign. (STRING-EVIDENCE)
- Money: wage-budget requests, sponsor events, season tickets, bank
  balance, and a debt-to-receivership distress path.
  (STRING-EVIDENCE)
- Youth pipeline: scout search, coach verdicts, promotion to
  reserve or B team; retirement and regen systems are adjacent.
  (STRING-EVIDENCE)
- Awards run per competition and nation with votes, shortlists and
  a persisted roll of honour; records are tracked per club.
  (STRING-EVIDENCE)
- Fixtures can be postponed (manager-requestable) or abandoned
  with result codes; stadiums can expand or relocate.
  (STRING-EVIDENCE)
- Managers can go on holiday; network play exists with a join
  flow; game settings cover currency, background pictures and
  commentary speed; season-ticket income and a Director of
  Football role exist. (STRING-EVIDENCE)

## Rules surface (editable targets)

Calendar dates, board strictness, finance parameters, job rules,
youth and regen scheduling, awards and fixture policies should all
become editable data with the 01/02 behaviour as default — see
"Rules as data" in [architecture.md](architecture.md).

## Open questions

- Day-tick pipeline order, intake/regen rules, inbox flow, finance
  formulas, job labels, friendlies: needs playtest plus
  function-level analysis.
