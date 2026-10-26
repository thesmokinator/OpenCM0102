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
| Data import | [dat-format.md](dat-format.md) | `cm0102-db` | Done; bucket rule, wage/contract offsets open |
| Competitions | [competitions.md](competitions.md) | `cm0102-core` competition | Catalog + menu done; bindings open |
| Match engine | [match-engine-spec.md](match-engine-spec.md) | `cm0102-match` | Placeholder; model open |
| Transfers | [transfers.md](transfers.md) | `cm0102-core` transfers | Flows mapped; formulas open |
| Tactics | [tactics.md](tactics.md) | `cm0102-core` tactics | Grids mapped; effects open |
| Season | [season.md](season.md) | `cm0102-core` season sim | Flows mapped; pipeline open |
| Players | [players.md](players.md) | `cm0102-core` data | Layout done; semantics open |
| Saves | [saves.md](saves.md) | app + `cm0102-core` | Native done; container solved, payload layouts open |
| App flows | [user-guide.md](user-guide.md) | app screens | Explorer + career done; rest open |
| Versions | [versions.md](versions.md) | all specs | Policy set: 3.9.60 data, 3.9.68 behaviour; playtest diff open |

## Rules files

All game rules ship as editable data (see "Rules as data" in
[architecture.md](architecture.md)). Planned files, with 01/02
defaults: tiebreak, league, cup, discipline, match, weather, lineup,
tactics, players, medical, training, morale/condition, career,
transfers, loans, permits, finance, season, setup, game settings,
persistence. Each gameplay spec doc lists its file's knobs.

## Gaps

Closed through analysis (see spec docs for details):

- Per-league slot counts and sizes, including cup-exclusion proof.
- Cup draw algorithm: full shuffle, random home/away, bye carry.
- Referee strictness mapping (Lenient/Strict/Harsh bands).
- Save container solved: 268-byte file table, byte-RLE payloads,
  109-file inventory incl. per-competition state, atomic temp-pack
  flow, localized-DB insight.
- Full 3.9.60→3.9.68 data diff: counts, nation move/renumbering,
  Asian expansion, Finland fix, NSL reputation, scripted-events
  and weather config formats, record-size stability.
- Patch container fully mapped; both 3.9.68 executables recovered
  and string-diffed (Korean/Japanese content, new languages, save
  version validation, playoff news rework).

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
9. Save payload record layouts + 3.9.68 sample (container and
    compression solved on a 3.9.60 save).
10. New-game screen order and rollover UX.
11. Exe-side 3.9.68 fixes (free-kick chance, MoM logic, interest
    severity, selection interplay, West Ham mechanism, ARSE):
    needs playtest observation on the patched game.
12. Bucket/spill assignment rule (distribution characterized;
    generative rule open).

## Build order

Each phase lists its spec, crate target and acceptance criteria.
A phase is done when the criteria pass and the workspace gate is
green.

1. **Premier season loop** ([competitions.md](competitions.md),
   `cm0102-core` competition + `cm0102-db`). Import league
   fields, generate the 20-team double round robin, apply
   results to the table. Done when: 380 fixtures scheduled,
   pro/rel cross-checks (Blackburn/Bolton/Fulham up, City/
   Coventry/Bradford down) reproduce from the data.
2. **Faithful match engine v1**
   ([match-engine-spec.md](match-engine-spec.md), `cm0102-match`).
   Replace the placeholder behind `MatchEngine`: scorer/penalty/
   own-goal events, cards with scoped suspensions, 17-column
   stats, weather postponement hook. Done when: deterministic
   per seed, event strings match the specified catalogue.
3. **Tactics model** ([tactics.md](tactics.md), `cm0102-core`
   tactics). Preset loading (`.pct` structure), 14-row
   instruction grid, team settings, lineup legality (11, caps,
   taker/captain rules). Done when: presets round-trip and an
   illegal lineup is rejected with the specified reason.
4. **Transfer engine v1** ([transfers.md](transfers.md),
   `cm0102-core` transfers). Windows/deadlines, offer lifecycle,
   Bosman compensation, loans, permits, board caps. Done when:
   scripted `player_setup.cfg` events apply and deadline/loan
   rules from the spec hold on fixtures.
5. **Season simulation** ([season.md](season.md),
   [players.md](players.md), `cm0102-core` season sim). Day-tick
   pipeline, board ladder, finance, youth intake, awards,
   news flow. Done when: a scripted season advances with
   balanced books and the specified board/finance events.
6. **Full app flows** ([user-guide.md](user-guide.md), app).
   League screens, options, holiday, jobs, search, editor
   parity; network last. Done when: the new-career flow runs
   end to end and every screen has a spec cross-check.
7. **Original save compatibility** ([saves.md](saves.md), app +
   `cm0102-core`). Read 3.9.60/3.9.68 saves (container + RLE
   done; payload layouts as specified), then write 3.9.68.
   Done when: the sample saves load with identical squads and
   tables, and our saves load in the original game.

## Verification

Every phase ships with tests: synthetic fixtures for parsers,
deterministic seeds for engines, spec cross-checks on known 01/02
entities (squads, tables, winners). A phase is done when its spec
section is fully implemented and the workspace gate (`cargo build`,
`cargo test`, `cargo clippy -- -D warnings`) is green.
