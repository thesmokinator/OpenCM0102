<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Implementation roadmap

The complete overview for implementing the game in Rust: what is
specified, what each system needs, and in which order to build it.

Evidence levels used across specs: VERIFIED-DATA (imported bytes),
STRING-EVIDENCE (shipped UI text: proves the feature exists,
mechanics need playtest or function-level analysis), OPEN.

## System map

| System | Spec | Target | Status |
|---|---|---|---|
| Data import | [dat-format.md](dat-format.md) | `cm0102-db` | Done; bucket encoding, wage/contract offsets open |
| Competitions | [competitions.md](competitions.md) | `cm0102-core` competition | Catalog + menu done; bindings open |
| Match engine | [match-engine-spec.md](match-engine-spec.md) | `cm0102-match` | Placeholder; model open |
| Transfers | [transfers.md](transfers.md) | `cm0102-core` transfers | Flows mapped; formulas open |
| Tactics | [tactics.md](tactics.md) | `cm0102-core` tactics | Grids mapped; effects open |
| Season | [season.md](season.md) | `cm0102-core` season sim | Flows mapped; pipeline open |
| Players | [players.md](players.md) | `cm0102-core` data | Layout done; semantics open |
| Saves | [saves.md](saves.md) | app + `cm0102-core` | Native done; original format open |
| App flows | [user-guide.md](user-guide.md) | app screens | Explorer + career done; rest open |
| Versions | [versions.md](versions.md) | all specs | Policy set: 3.9.60 data, 3.9.68 behaviour; exe diff open |

## Rules files

All game rules ship as editable data (see "Rules as data" in
[architecture.md](architecture.md)). Planned files, with 01/02
defaults: tiebreak, league, cup, discipline, match, weather, lineup,
tactics, players, medical, training, morale/condition, career,
transfers, loans, permits, finance, season, setup, game settings,
persistence. Each spec doc lists its file's knobs.

## Gaps

Closed through analysis (see spec docs for details):

- Per-league slot counts and sizes, including cup-exclusion proof.
- Cup draw algorithm: full shuffle, random home/away, bye carry.
- Referee strictness mapping (Lenient/Strict/Harsh bands).
- Save sketch: `.mbr` container, versioned header fields, 33
  temp-file inventory, atomic temp-pack flow.

Still open (each needs playtest observation, function-level
analysis, or both — details in the per-system specs):

1. Day-tick pipeline order (what Continue runs in order).
2. League tiebreak order and points value per league.
3. Per-cup bindings (tiebreak, legs, draws, venues).
4. Goal/chance model, ratings, attribute and tactic weights.
5. Lineup legality, sub caps, taker/captain fallback.
6. Display-vs-storage attribute mapping.
7. Wage/contract/expiry storage offsets.
8. Regen/intake dates and generation rules.
9. Byte-level save confirmation (needs user-supplied saves).
10. New-game screen order and rollover UX.
11. 3.9.60-vs-3.9.68 code-level behaviour diff (strings diff and
    official history done — needs playtest observation).

## Build order

1. League fields import + Premier season loop (fixtures, table).
2. Faithful match engine v1 behind `MatchEngine`.
3. Tactics model wired into lineups and matches.
4. Transfer engine v1 (windows, bids, loans, permits).
5. Season simulation (board, finance, youth, awards).
6. Full app flows (leagues, options, holiday, jobs, network later).
7. Original save compatibility (read, then write).

## Verification

Every phase ships with tests: synthetic fixtures for parsers,
deterministic seeds for engines, spec cross-checks on known 01/02
entities (squads, tables, winners). A phase is done when its spec
section is fully implemented and the workspace gate (`cargo build`,
`cargo test`, `cargo clippy -- -D warnings`) is green.
