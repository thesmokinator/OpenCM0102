<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Competitions

Status: **research in progress**. The catalog below is fact from the
imported data; league membership and per-competition rules are under
analysis and nothing about them may be treated as fact until the
observation is recorded here.

## Catalog (from the imported data)

409 competitions share one id space: 390 club competitions (from
`club_comp.dat`) and 19 nation competitions (from `nation_comp.dat`),
each with a name, short name, code, continent, nation and reputation
(0–20). Top by reputation:

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
[dat-format.md](dat-format.md) for the verified layout). The
lower-league bucket encoding used by small clubs is still open.

## Coverage

Per-competition rules to specify, each verified by observing the
original game before it lands here:

- Leagues: fixture format (home/away legs), points for a win,
  tiebreaker order, promotion/relegation slots and play-offs,
  prize money.
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
