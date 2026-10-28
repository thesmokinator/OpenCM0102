<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Competitions

Status: **research in progress**. The catalog below is fact from the
imported data; league membership and per-competition rules are under
analysis and nothing about them may be treated as fact until the
observation is recorded here.

## Catalog (from the imported 3.9.60 data)

409 competitions share one id space: 390 club competitions (from
`club_comp.dat`) and 19 nation competitions (from `nation_comp.dat`),
each with a name, short name, code, continent, nation and reputation
(0–20). (3.9.68 grows this to 452 — see
[dat-format.md](dat-format.md).) Top by reputation:

| ID  | Competition                  | Nation  | Rep |
|-----|------------------------------|---------|-----|
| 24  | Italian Serie A              | Italy   | 20  |
| 326 | European Champions Cup       | —       | 20  |
| 404 | FIFA World Cup               | —       | 20  |
| 16  | German First Division        | Germany | 19  |
| 52  | Spanish First Division       | Spain   | 19  |
| 406 | European Football Championship | —     | 19  |
| 7   | English Premier Division     | England | 18  |
| 58  | South American Copa Libertadores | —   | 18  |
| 65  | Brazilian National First Division | Brazil | 18 |

England's league ladder, as named in the data: Premier Division (7),
First (8), Second (9), Third Division (10), Conference (93),
plus FA Cup (351), League Cup (352), Charity Shield (353),
Vans Trophy (354) and three regional premier divisions (358–360).

## Membership

Solved for league fields: `club.dat` byte 87 is the current-season
league id and byte 91 the previous-season one (see
[dat-format.md](dat-format.md) for the verified layout).
Competition ids below 364 are stable across data versions (only
slot 213 is renamed and 364–389 repurposed), so byte-87 values
are comparable; same-name club moves between versions are real
promotion/relegation updates (e.g. Köln 16→17 for 02/03).

Small clubs that play in no simulated league share bucket slots
instead of league ids (VERIFIED-DATA distribution on 10,580
clubs): slot 101 holds 2,351 worldwide minnows, 254 holds 2,071,
58 holds 993, with smaller buckets (97/98/57/90/…). Buckets
persist and grow in 3.9.68. Some minnows also spill under real
league ids (12 Australians under Serie B's 25, Turkish sides
under Serie C2/B's 29). The rule assigning a club to a bucket or
a spill slot is still open.

## Coverage

Per-competition rules to specify, each verified by observing the
original game before it lands here:

- Leagues: fixture format (home/away legs), points for a win,
  tiebreaker order, promotion/relegation slots and play-offs,
  prize money. (Implemented provisionally as 3/1/0 with
  points–goal difference–goals scored order until observed.)
- Domestic cups: rounds, byes and seeding, one or two legs,
  replays, extra time, away goals and penalty order, final venue,
  qualification reward.
- Continental cups: 01/02 Champions Cup and UEFA Cup formats
  (qualifying, phases, draws, seedings).
- Internationals: World Cup and European Championship qualifying
  and finals formats.
- One-off ties: charity shields, super cups.
- Reserve and youth competitions, if simulated (to establish).

Once specified, every rule becomes an editable data file with the
01/02 season as default — see "Rules as data" in
[architecture.md](architecture.md).

## Cup draws (implemented)

`cm0102-core` `cup` draws one round from a pot of entrants plus
carried-in byes: `DrawMode::Plain` shuffles, `Seeded` and
`SeededNationAvoidance` sort by seeding value highest-first and deal
from a random start with wraparound, flipping a coin for home/away
per tie. `seeding_score` sums the documented key (squad-value term
plus sixteen times club strength plus weighted best-fifteen
ratings). Per-round mode bindings stay open.

## Calendars (implemented)

League fixture lists are generated, not stored: `Schedule` in
`cm0102-core` builds a dated calendar from a club list with the
circle-method round robin (`Schedule::league_calendar`, parameterized
by first-round date, round spacing and legs) and feeds Continue as a
`DayEventSource`, stopping on the managed club's match days. The 01/02
calendar quirks (derby placement, TV moves) are still open, so the
generator takes the structural parameters and absorbs the quirks as
editable rules when observed. Calendars are wired into careers:
byte 87 is imported into `Club.league_id` (bucket slots in
`NON_LEAGUE_SLOTS` get no calendar), the career start generates the
league calendar, and the season rollover regenerates it year after
year (see "Season rollover" in [season.md](season.md)).

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, exact bindings still
need playtest or function-level analysis).

- Tiebreak menu is data-driven per round: replay, extra time,
  golden-goal extra time, away goals (post-90 or post-extra-time),
  penalties, US-style shoot-out, higher-seeding progression, and a
  best-over-3-matches series. (STRING-EVIDENCE)
- Points for a win is a per-competition 2-or-3 knob ("Two"/"Three"
  labels); Argentina relegates on a 3-season points average.
  (STRING-EVIDENCE)
- League tiebreak criteria name Games Won, Goals For and Goal
  Difference; their order is still open. (STRING-EVIDENCE)
- Promotion and relegation playoffs exist (numbered promotion
  ladders, relegation playoffs, playoff groups), and promotion can
  be vetoed when the stadium is below required capacity.
  (STRING-EVIDENCE)
- Generic cup engine: pre/qualifying/numbered rounds through
  quarter/semi/final, each with first/second leg, replay(s) and
  decider variants; draws are dated events with automatic mode and
  byes; ties can be on neutral ground. Each round selects one of
  three draw modes from its own configuration: a plain random
  shuffle, a seeding-ordered draw, or a seeding-ordered draw with
  same-nation avoidance. Seeded modes order entrants by seeding
  value (highest first) and then deal them into ties from a random
  starting slot with a wraparound scan, flipping a coin for
  home/away per tie; byes carry through. (Function-level
  analysis; exact per-competition mode bindings still open.)
- Club ranking for seeding is a strength ranking, not a table
  order: a value term (squad value divided by 400, or by 300 in
  the alternative accounting mode), plus sixteen times a club
  strength field, plus the weighted ratings of the best fifteen
  players under formation-slot weights. League and cup ranking
  share this key and sort it highest-first. The league-table
  tiebreak order is a separate mechanism and remains open.
  (Function-level analysis.)
- Drawn-tie progression marks each entrant with a status pair:
  one pair of codes for most round formats and a distinct pair
  for one specific round mode (numeric mode 140 in the data).
  Which sporting format mode 140 denotes (two-legged vs
  single-match is the leading hypothesis) needs playtest
  confirmation. (Function-level analysis.)
- Prize money binds to six triggers: match win, match draw, final
  position, round qualification, title, runners-up; amounts are
  per-competition data. (STRING-EVIDENCE)
- 01/02 European Cup: three two-legged qualifying phases, Phase 1
  groups A–H, Phase 2 groups A–D, with squad registration caps.
  (STRING-EVIDENCE)
- Cup Winners' Cup history ends in 1999 (defunct); qualifying
  sections carry no history rows; finals tournaments record hosts.
  (VERIFIED-DATA)
- League sizes (membership counts by `club.dat` league byte):
  Premier 20, First/Second/Third Division 24 each, Conference 22,
  Serie A 18, German First Division 18, Spanish First Division 20;
  cups store 0 (league fields only). Small-club spill and buckets
  are covered under Membership above. (VERIFIED-DATA)
- 3.9.68 Asian expansion: 43 new competitions (Indonesia, Singapore,
  Hong Kong, India, Malaysia, Thailand, China, Gulf and Far East
  cups) plus slots 364–389 repurposed to Asian competitions; the
  Arab cups move to 373–377; 'Chinese Super Cup' becomes 'Chinese
  FA Super Cup'. (VERIFIED-DATA, patched bytes)
- 3.9.68 Finland fix: the 19 Yugoslav clubs wrongly parked under
  the Finnish Premier Division (id 114) in 3.9.60 are gone; the
  slot holds the 12 correct 2002 Veikkausliiga clubs (plus
  bucket-sharing minnows — the spill encoding still applies).
  (VERIFIED-DATA, patched bytes)
- 3.9.68 reputation tweaks: Australian NSL 6→8, Oceania Champions
  Cup 6→7, NSW and Victorian leagues 1→2. (VERIFIED-DATA)
- Korean and Japanese competitions already exist in 3.9.60 data
  unchanged, so the 3.9.67 "added South Korean league" is a
  playability change (league selection/rules), not new competitions.
  (VERIFIED-DATA)
- Runtime league state (`<name>_<id>.tmp` in saves) holds 118-byte
  team slots, club id first, alphabetical (all 18 Serie A clubs
  verified); all stat fields read zero on a season-start save.
  Fixtures live in 316-byte records holding competition id,
  home/away club ids and date/venue fields (partially mapped).
  Cup state holds packed `u32` entrant arrays (16 Serie A sides
  verified in Coppa Italia). Ids are save-local (see
  [saves.md](saves.md)). (VERIFIED-DATA structure; field
  semantics need a mid-season save)

## Open questions

- League tiebreaker order and points value per league, per-cup
  bindings, final venues and legs, qualifying group sizes,
  whether reserves/youth are simulated: needs playtest plus
  function-level analysis.

## Version notes (3.9.68)

Behaviour target is v3.9.68 (see [versions.md](versions.md)). The
patch fixed competition bugs whose 3.9.60 form must NOT be
reproduced: the England/Conference/Scotland selection interplay,
Australian NSL team and foreign-player counts, Polish First Division
relegations, the Argentine Second Division format, and the Finnish
Premier Division membership (data-side). Per-competition rules land
here only in their fixed form, once confirmed against the patched
game.
