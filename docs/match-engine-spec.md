<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# match-engine-spec.md — original match behaviour

Status: **not yet specified**.

The engine in `crates/cm0102-match` resolves matches minute by minute:
chances arise per side, each chance picks a shooter, competing
attribute rolls select how the shot is taken, the keeper contests it,
and a distance falloff feeds the goal test. The mechanism below is
observed behaviour; rates, weights and thresholds are calibrated
placeholders until measured (each is a named constant in the engine).

Every fixture — managed or AI — runs through the same engine. AI sides
field an auto-picked eleven drawn from available players only. Cards,
bans and injuries carry over into the season state: suspensions exclude
the player from upcoming picks, yellow counts towards the five-card ban
reset each season, and injuries lay the player off for days.

Chance creation bends with team mentality (attacking sides create more,
defensive sides less), the opponent sitting deep, average side morale
and individual fitness: exhausted players perform at half their rated
attributes. The managed club plays its own instructions; AI clubs play
balanced.

## Resolving a chance (observed mechanism)

- Attributes evaluate on a 0–200 scale (raw value times ten).
- All randomness is uniform integer rolls bounded by an attribute.
- The shot category comes from competing rolls — one per candidate
  category, each bounded by its attribute — with the best roll
  winning; the keeper's state gates which categories may compete.
- The goal test pits the winning roll against a keeper roll plus a
  falloff that grows quadratically with distance, plus small noise.
- Shots carry a minute, a scorer and variants (penalty, own goal);
  team shot counts track shots and shots on target.

## Match record (implemented)

- Event set: goals with minute and scorer, scored and missed
  penalties, own goals, bookings, dismissals (second yellow or
  straight red), injuries with forced substitutions, tactical
  substitutions, and a shootout summary for decided cup ties.
- Bookings weigh dirtiness and aggression; injuries weigh injury
  proneness; tactical changes replace the weakest performer.
- Ratings derive from recorded events (goals, cards, clean sheets
  for defenders, result) on a 5.0–10.0 scale; the exact original
  formula is still open.
- Cup ties play two 15-minute periods when level, then a shootout:
  five rounds with early decision, then sudden death, taker order
  by penalty ability. Wiring to cup competitions waits on cups.
- Suspensions and injury layoffs from match incidents are not yet
  applied to the season; tactics, morale and condition weights are
  still open.

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

- Chance-creation rates, exact attribute-to-category mapping, keeper
  state effects, goal-test constants, the exact rating formula,
  man-of-the-match selection, and the weights of tactics, morale
  and condition: needs black-box sampling plus function-level
  analysis.
- Referee strictness, weather/pitch gameplay modifiers, home
  advantage, AI substitutions, assist definition, ban accounting:
  needs playthrough observation plus function-level analysis.
