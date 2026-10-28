<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Screens

Status: **research in progress**. Screen inventory (which screens
exist, their sections and actions) from the shipped UI text; layout
and flow per screen need playthrough observation before any
faithful replica is implemented. Nothing about a screen's layout is
fact until the observation is recorded here.

## Method

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the screen or action exists, layout still
needs playtest observation).

- Screen families come from the shipped code-module names
  (`*_screens`): 16 families covering every game area.
- Section, action and column titles come from the local exe-string
  inventory (title-case UI strings grouped by keyword: ~900
  distinct titles across 86 groups, plus 3,679 short UI words).
- The executable ships (near-)no Win32 dialog resources (its
  `.rsrc` section is ~4.6 KB: version info and icons only), so the
  UI is custom-drawn: control-level layout cannot be recovered
  statically and must be observed in play (screenshots per
  section, see Open questions).

## Screen families (from shipped module names)

| Family | Role | Our coverage |
|---|---|---|
| `award_screens` | Awards (per nation/competition, votes, shortlists, hall of fame) | Placeholder section |
| `club_screens` | Club info (stadium, sponsors, rivals, records) | Placeholder section |
| `comp_screens` | Competitions (tables, fixtures, draws, history) | Fixtures + table done; cups/history open |
| `contract_screens` | Contracts (offers, renewals, clauses, expiry) | Placeholder (Transfers) |
| `file_screens` | Save/load careers, hall of fame files | Done (native saves) |
| `find_screens` | Game setup finds (club/nation pick lists) | Done (new-career flow) |
| `manager_screens` | Manager profile, job applications, resignations | Placeholder (Board/Staff) |
| `match_screens` | Match view (overview, stats, ratings) | Done (live match day) |
| `national_teams_screens` | International management | Open |
| `new_transfer_rule_screens` | Transfer-regulation updates | Open |
| `news_screens` | Inbox and news queue | Done |
| `search_screens` | Player/staff search, filters, shortlists | Placeholder section |
| `staff_screens` | Staff profile, notes, coach/scout reports | Placeholder section |
| `tactics_screens` | Formation, instructions, set pieces, presets | Formation + lineup done; effects open |
| `training_screens` | Schedules, intensities, physio, fitness | Placeholder section |
| `transfer_screens` | Bids, negotiations, loans, permits, clauses | Engine done; UI open |

## Sections in detail

### Squad

Views for Main, Reserve, Senior, Under-21 and Under-23 squads
with squad numbers and the status ladder (`Squad Status`,
`Transfer Status`). Actions: `Clear Squad`, `Confirm Squad
Selection`, `Submit Squad Numbers`, `Submit European Squad`,
`Set Transfer Status`. Availability states name Available,
Injured and suspended players; `Squad Invalid` guards illegal
picks. Our squad list is done; European submission and the
status-ladder UI stay open.

### Tactics

Formation load/save as preset plus `Lock Formation` /
`Unlock Formation`, With-Ball / Without-Ball views, `Set
Pieces` with taker roles (`Penalty Takers`, corner/free-kick
takers), tackling/passing/crossing instructions. Our formation
and lineup model is done; instruction effects and slot mapping
stay open.

### Training

Named schedules (`Load/Save/Delete Training Schedule`) with
intensity levels, coach styles, and the physio pipeline
(`Physio`, `Physiotherapy`, surgery, light/full training). The
injury catalogue names body parts (knee, groin, hip, shoulder,
shin, thigh, head, groin). Form, condition and morale show per
player. Our section is a placeholder.

### Fixtures

Round ladder from qualifying through numbered rounds to
quarter/semi/final, each with first/second legs, replays
(`Final Replay`, `Playoff Replay`, `Qual.Rnd Replay`) and
deciders, plus postponement requests, abandonments and neutral
venues. Our league fixtures are done; cup rounds, replays and
history stay open.

### League Table

Table with `Games Won`, `Games Lost`, points and positions,
promotion ladders (`First/Fourth Promotion`), playoffs
(`North Playoff`, `Playoff Final`) and relegation groups, plus
a `Live League Table` view. Ours is provisional (points–goal
difference–goals order); tiebreak order and playoff flows stay
open.

### Transfers

Bid verbs (`Approach To Buy/Loan/Sign`, `Negotiate`,
`Accept`, `Reject`, `Reject All Bids`, `Withdraw`, `Delayed`),
`Clauses`, `Free Transfer`, loans (`For Loan`, `On Loan`,
`Recall From Loan`, `Terminate Loan Deal`, `Loan Expiry
Date`), `Work Permit`, trials (`Offer Trial`, `Trial
Period`), contracts (`Offer/Withdraw Contract`,
`Contract Renewal`, `Contract Expires`, `End of contract`)
and wages (weekly, monthly, yearly, `Signing On Fee`) under
`Transfer Windows`. Our engine (permits, setup loans,
deadlines) is done; the UI stays open.

### Finances & Info

`Finances & Info` with wage budgets, sponsor events, season
tickets, bank balance and a debt-to-`Bankrupt` distress path
(sparse titles; content mostly from the finance file layout
and news strings in [season.md](season.md)). Our section is a
placeholder.

### Board

`Board Confidence` with updates, season expectations
(`Board announce expectations`), `Meet with Board`,
`Board Request`, derby reactions, plus jobs (`Apply for
Job`, `Job Description`, `Resign from Club`) and `F.A. /
FA Confidence`. Our match-reaction news is done; confidence
deltas, jobs and international management stay open.

### Staff

Profiles for players, managers (`Manager Stats`,
`Manager Status`, `Manager Chat`), coaches, scouts and
physios, `Staff Search` with saveable filters, `Edit/Create
Staff Note`, coach verdicts and contracts. Our section is a
placeholder.

### Search

`Player Search`, `Staff Search`, `Quick Search` and `Scout
Search` with `Filters`, attribute/position/age/wage/value
criteria, `Compare Players`, shortlists (`Add/Remove From
Shortlist`, buy-only-from-shortlist) and youth search. Our
section is a placeholder.

### News

Inbox queue with messages, memos and media (`Media Pundit`,
local/newspaper items). Ours is done (queue, categories,
fresh-news Continue stops).

### History

Appearances by season (`This Season`, past seasons),
`Written History`, backed by the history payloads
(`club history.dat`). Our section is a placeholder.

### Records

`Records` with `Biggest Defeat`, oldest/youngest players,
`Sequences`, attendances, backed by `club records.dat`. Our
section is a placeholder.

### Awards

Votes (`Vote For Award`), shortlists, previous winners and
monthly/yearly cadence (`Every Month`, `Every Year`),
per-nation and per-competition awards plus hall of fame
(`Hall Of Fame`, `Merge/Save Hall Of Fame`). Our section is
a placeholder.

### Club Info

Stadium (with `Unknown Stadium` fallback), reserves
(`Reserve Squad`) and rivals. Our section is a placeholder.

## Game flows

New-career picks (club/nation lists via `find_screens`) and
save/load (`Save Game`, `Restore Saved Game`,
`Delete Saved Game`, `Save Game Before Exit`) are done in
our hub. `Continue`, `Go on Holiday`, `Game Settings` and
the match screens are done; national-team management and
transfer-regulation update flows stay open.

## Open questions

- Control-level layout per screen (column sets, button rows,
  tab orders): needs playthrough screenshots, one per sidebar
  section plus the new-career flow and the match screens.
- Flows (transfer negotiation steps, job application flow,
  awards voting): needs click-through observation.
- Which grids are sortable/filterable and their default sorts:
  needs playthrough.
