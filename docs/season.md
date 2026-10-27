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

- Intake/regen rules, finance formulas, job labels, friendlies:
  needs playtest plus function-level analysis. The pipeline order
  below is our reconstruction until the original's is observed.
- Function-level status: the job-vacancy filler (best-score
  selection with a -1,000,000 floor and a 5,000 eligibility
  threshold on the candidate record) and the regen processor
  are located and decompiled. Regen consumes the retired-staff
  list and, when more than two entries are pending, generates
  that count minus two new players: unattached regens draw two
  uniform 1–1500 values and one 1–500 value, while attached
  regens inherit clamped fields from the retiree with
  strength-ratio gates at one-half and three-quarters and a
  7,501 threshold branch. Board-confidence outcomes (vote of
  confidence, patience warnings, sackings with three cause
  variants) surface as queued news events, so the numeric
  confidence ladder sits upstream of the news queue and stays
  open, as do youth-intake counts. The persisted manager-
  confidence file is a per-manager variable-length record: a
  main block plus counted dated sub-records. The confidence
  value itself is a short in the main block on a roughly
  0–10,000 scale, displayed in six bands: very disappointed
  below 3,501; unsatisfied to 4,275; satisfied to 6,500;
  pleased to 8,500; very pleased to 9,500; delighted above.
  How results move the value remains open. Separately, club
  finance records board investment: a counter capped at 20
  pays counter×50,000+75,000 when the balance passes
  2,250,000 (with a 1-in-40-style random gate), one-off
  injections of 1,500,000 above a 17,500,000 balance and
  7,500,000 above 75,000,000 at full counter, and a declining
  branch of 4,500,000−200,000×counter floored at 1,000,000.
  (Function-level analysis.)

## Continue model (implemented)

The day is the simulation tick while Continue is event-driven: one
press simulates days until the next date needing the manager (own
match, inbox items, offer responses, deadlines). Each simulated
day runs the pipeline in order — date advance, fixtures (due AI
matches plus a results roundup), then the stop check — and fresh
inbox items dated today stop the loop like any other event.

Fresh news is presented first: pressing Continue with unread
today's items opens the news queue instead of advancing. Match
days enter the match-day screen (pre-match, live commentary with
a half-time pause, full-time report); the match cannot be skipped
— unplayed fixtures keep stopping Continue until played — and the
Continue button hides until full time. Holiday runs the same
daily pipeline seven days without stops, with the assistant
playing due matches.

Implementation shape, in `cm0102-core` `simulation` plus the app:
`process_day()` advances one day, the app simulates the day's
world and pushes news, and `stops_for()` matches the day against
the editable stop rules (`ContinueRules`: own match, inbox, offer
response, deadline, plus a safety day cap).

## Season rollover (implemented)

`GameState` owns this season's `Schedule`. When no fixture remains
on or after today, the app generates next season's calendar from the
same league membership (`Schedule::for_club_league`, first round
18 August of the advancing year), so careers continue year after
year; old saves with an empty schedule roll into a fresh calendar
on the next Continue. Promotion and relegation between seasons,
per-league start dates and the 01/02 calendar quirks are still
open — the generator takes them as parameters so they land as
editable rules. Only played results persist per fixture; other
clubs' matches are not auto-simulated yet (needed for the league
table).
