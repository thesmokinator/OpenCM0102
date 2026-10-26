<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# match-engine-spec.md — original match behaviour

Status: **not yet specified**.

The engine in `crates/cm0102-match` is a declared placeholder (Poisson goal
counts, deterministic per seed) that exists only to exercise the season
loop and UI behind the `MatchEngine` trait. It must never be presented to
users as a reproduction of CM 01/02.

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

## Open questions

- The full per-item behaviour, verified by observing the original
  game (same fixture replayed, stat samples over many matches).
