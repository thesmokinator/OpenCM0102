<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Players and attributes

Status: **research in progress**. The data layout is largely
verified (see [dat-format.md](dat-format.md)); the *meaning* of
most fields is still open and nothing about it is fact until the
observation is recorded here.

## Coverage

- The 54 attributes: positional ratings and playing attributes —
  names and order verified, in-match effects open per attribute.
- Current and potential ability: scale, how CA derives from
  attributes, what PA gates, random potentials (-1/-2, verified
  present).
- Reputation: home/current/world (0–200, offsets verified), effects
  on transfers, awards and AI behaviour.
- Position learning and versatility (verified present as
  attributes; mechanics open).
- Training: schedules, coach effects, attribute movement.
- Development and ageing curves (peak ages, decline, retirement).
- Injuries: proneness (verified present), types, durations, match
  injuries, physio effects.
- Morale, form and condition during matches.
- Consistency and important matches (verified present; effects
  open).
- Hidden attributes, if any (to establish).

## Verified so far

- Attribute names and order, CA/PA offsets, reputation offsets,
  birth date, job codes 11/5 — all in [dat-format.md](dat-format.md),
  cross-checked on known people (Henry, Vieira, Bergkamp, Seaman,
  Wenger).
- Scale facts: attributes are 0–20 with 0 as unset (~74% zeros);
  CA spans -1–199, PA -2–200 with 4,120 random-potential (-1) and
  327 (-2) rows; reputations span -1/-2–200 with negative sentinels.

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, numerics still need
playtest or function-level analysis).

- Morale shows on a 6-band scale and falls with frequent fining or
  press speculation; jadedness is a percent with rest bands.
  (STRING-EVIDENCE)
- Recovery pipeline: injury, possible surgery, physiotherapy, light
  training, full training; recurring injuries are curable by long
  surgery or career-ending. (STRING-EVIDENCE)
- Training uses named schedule files with 7 categories, 4
  intensities and per-side/position targeting, plus absence and
  bust-up events. (STRING-EVIDENCE)
- Coach reports are tenure-gated with a verdict ladder
  (first-team member to release) and upside callouts.
  (STRING-EVIDENCE)
- Reputation shows in bands up to World Class plus per-nation
  values; send-off histories and grievances feed it.
  (STRING-EVIDENCE)
- Retirement has separate full and international configs; regens
  key off retired staff ids. (STRING-EVIDENCE)
- Displayed attribute labels differ from stored names for several
  attributes; the render mapping is unresolved. (STRING-EVIDENCE,
  partly open)

## Rules surface (editable targets)

Ability scales and bands, growth and ageing curves, training
categories, injury tables, morale drivers, retirement tables should
all become editable data with the 01/02 behaviour as default — see
"Rules as data" in [architecture.md](architecture.md).

## Open questions

- CA formula, PA ranges, growth curves, morale numerics, attribute
  effects, hidden attributes, label mapping: needs playtest plus
  function-level analysis.
