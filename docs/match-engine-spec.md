<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# match-engine-spec.md — original match behaviour

Status: **not yet specified**.

The engine in `crates/cm0102-match` is a declared placeholder (Poisson goal
counts scaled by the relative average current ability of the two
starting elevens, deterministic per seed) that exists only to exercise
the season loop and UI behind the `MatchEngine` trait. It must never be
presented to users as a reproduction of CM 01/02: the strength scaling
is invented, and the real attribute weights are still open (see the
coverage list below).

Observations of the original game's match behaviour are recorded here
from playing the original game before any faithful engine is
implemented.

## Coverage

Every item below must be specified:

- Inputs: which attributes, ratings, condition, morale, tactics and
  reputation feed the simulation, and how each is weighted.
- Chance creation and conversion (shot counts, on-target rates,
  goal timing distribution across minutes).
- Assists, ratings and man of the match.
- Cards and injuries during matches (frequencies, triggers).
- AI substitutions and tactical reactions.
- Extra time and penalty shoot-out procedure.
- Home advantage, referee and weather effects (to establish).
- Determinism: what varies run to run and what does not.

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, mechanics still need
playtest or function-level analysis). Line numbers refer to the
local exe-string inventory.

- Event model: scorer lines `<name> <min>` with `pen`, `og` and
  `m/pen` (missed penalty) variants; shoot-outs render per-taker
  `scored|missed` plus `Pens <s1>-<s2>`, and a separate `35 Yard
  Shootout` (`35yd <s1>-<s2>`) exists. (STRING-EVIDENCE)
- Tie-break ladder is configured per competition: extra time none /
  standard / golden goal, away goals after extra time or after
  ninety minutes, penalty or US-style shoot-out. (STRING-EVIDENCE)
- Cards feed scoped suspensions: red-card day bans, sent-off match
  bans with repeat-offender `further` variants, yellow-accumulation
  bans including a successive-matches variant. (STRING-EVIDENCE)
- Stats: per-player condition/offsides/interceptions/shots/key
  headers/tackles/passes columns, team block (headers, tackles,
  passes, offsides, throw-ins, free kicks, corners, shots on/off
  target), a `Ball in play` timer, average ratings and man-of-the-
  match awards aggregated per competition. Stats screens offer 17
  columns: Appearances, Goals, Conceded, Penalties, Assists, Team
  Goals/Conceded, Games Won/Lost, Yellow/Red Cards, Man of Match,
  Pass Completion, Tackles/Game, Dribbles/Game, Shots On Target,
  Average Rating. (STRING-EVIDENCE)
- Weather and pitch: per-nation climate config, precipitation/wind/
  pitch states driving postponement and mid-match abandonment, with
  league-table flags; undersoil heating protects fixtures but can
  fail. (STRING-EVIDENCE)
- `weather.cfg` defines 44 climate zones (id + name, many with
  Köppen codes, e.g. `8 Caf - Tokyo`; 3.9.68 inserts zone 41
  `South East Asia Tropical`). Each zone has four season lines
  (SPRING/SUMMER/AUTUMN/WINTER) giving the season start day plus
  cumulative probability tables for Wind (Calm/Breezy/Gusty/
  Strong/Gale), Precipitation (Dry/Wet/Drizzle/Shower/Down Pour)
  and Warmth (Freezing/Cold/Mild/Fine/Warm/Hot/Very Hot).
  (VERIFIED-DATA, config text; gameplay modifiers still open)
- Referees are database entities (`officials`, 3,124 rows) with a
  stats screen and an FA complaint flow. (VERIFIED-DATA +
  STRING-EVIDENCE)
- Referee strictness shows as a name descriptor — Lenient (values
  0–5), Strict (15–19), Harsh (20) — with mid values unlabeled.
  (STRING-EVIDENCE; in-game wording to confirm by playtest.)
- Injuries: 60+ body-part types, database-seeded starting and
  recurring injuries, doubtful/out availability states.
  (STRING-EVIDENCE)
- Derby fixtures get dedicated board-reaction coverage; no
  attendance or performance modifier is evidenced.
  (STRING-EVIDENCE)

## Rules surface (editable targets)

Weather zones and seasonal tables, referee strictness bands,
suspension catalogs, stat columns, man-of-the-match policy and the
chance/goal model constants must all become editable data with the
01/02 behaviour as default — see "Rules as data" in
[architecture.md](architecture.md).

## Open questions

- Goal/chance model, rating and man-of-the-match formulas, and the
  weights of attributes, tactics, morale and condition: needs
  black-box sampling plus function-level analysis.
- Referee strictness, weather/pitch gameplay modifiers, home
  advantage, AI substitutions, assist definition, ban accounting:
  needs playthrough observation plus function-level analysis.
