<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Transfers

Status: **research in progress**. Nothing about transfer behaviour is
fact until the observation is recorded here.

## Coverage

Every item below must be specified before the transfer engine is
implemented:

- Transfer windows and deadlines per nation (dates, differences
  between leagues, deadline-day flow).
- Bids and negotiation (how offers are made, counter-offers,
  accept/reject conditions, unsettled players).
- Fees, installments and clauses (which clauses exist, how they
  trigger and pay out).
- Contracts, renewals and expiry (lengths, wage demands, expiry
  handling, youth contracts).
- Bosman and free transfers (out-of-contract rules, age thresholds).
- Loans (durations, fees, wage splits, recall rules).
- Work permits (which nations require them, grant/appeal rules).
- Squad registration and foreign-player limits per competition.
- Budgets (transfer/wage budgets, board adjustments, overspend
  handling).
- AI club behaviour (when and how AI clubs bid, value assessment).

## Verified so far

- Data facts: `staff_history.dat` records a loan flag plus
  appearances and goals (7,637 of 270,654 rows on loan); player
  records carry a squad number (see [dat-format.md](dat-format.md)).
- Scripted setup events live in `player_setup.cfg` as quoted
  commands naming the person and clubs, e.g. `"FUTURE_TRANSFER"
  "Steinar" "" "Pedersen" "IFK Göteborg" "IK Start" 01 12 2001
  0 -1 4` and `"LOAN" "Andrey" "Demkine" "Demkin" "Germinal
  Beerschot Antwerpen" "KSK Beveren" 01 07 2001 30 06 2002`.
  The 3.9.60 file holds 36 future transfers and 1,471 loans; the
  3.9.68 file (02/03 data) drops scripted future transfers and
  keeps 56 loans. (VERIFIED-DATA, config text)

## Findings

Evidence levels: VERIFIED-DATA (imported bytes) vs STRING-EVIDENCE
(shipped UI text: proves the feature exists, formulas still need
playtest or function-level analysis).

- Deadlines are per nation with separate transfer and free-transfer
  deadlines; post-deadline deals are blocked or cancelled.
  (STRING-EVIDENCE)
- Transfer regulations: international windows 1–31 July and
  15 December–15 January, contract protection 3 years (under 28)
  or 2 years (28+), maximum 5-year contracts, compensation for
  unprotected moves. (STRING-EVIDENCE)
- Offer lifecycle: accept / reject with reasons / negotiate /
  one-week delay / withdraw; release-clause-value offers cannot be
  delayed, negotiated or rejected. (STRING-EVIDENCE)
- Fees: base plus league/international appearance add-ons, percent
  of next sale and monthly installments; former clubs receive
  sell-on cuts. (STRING-EVIDENCE)
- Five release-clause types (manager job, relegation, non-playing,
  minimum fee, non-promotion) with fixed-fee activation events.
  (STRING-EVIDENCE)
- Loans are nation-parameterized: concurrency and seasonal caps,
  same-competition/division bans, appearance thresholds, recall
  and send-back locks (some absolute, some date-locked), wage
  splits. (STRING-EVIDENCE)
- Work permits: delayed grant-or-refuse, conditional permits,
  yearly renewal on games played, EU-passport exemption.
  (STRING-EVIDENCE)
- Board gates: wage/fee caps, unknown-player restrictions, blocks
  for small squads, no space/funds or poor finances.
  (STRING-EVIDENCE)
- Squad-status ladder gates contract talks; players reject with
  enumerated reasons; nationality and foreign-player signing bans
  exist (including club-specific matrices). (STRING-EVIDENCE)
- Player-club interaction verbs: praise, criticise, warn, promise,
  unsettle, transfer request, transfer listing, trial offers and
  approach-to-buy. (STRING-EVIDENCE)
- AI clubs bid, negotiate and withdraw (toggleable).
  (STRING-EVIDENCE)

## Rules surface (editable targets)

Deadlines and windows per nation, loan/permit/fee/clause rules,
board caps, squad statuses and nationality bans must all become
editable data files with the 01/02 season as default — see "Rules
as data" in [architecture.md](architecture.md).

## Open questions

- Permit formula, AI valuation, tribunal formula, wage/contract
  storage, Bosman thresholds, installment timing, deadline dates:
  needs playtest plus function-level analysis.
- Function-level status: contract and transfer load/save bodies
  are fully decompiled (field order enumerable from their read/
  write sequences). Work permits: a permit is required exactly
  when the destination club's nation is flagged as
  permit-requiring and the player holds no exempt (EU/second)
  nationality. Permits are 14-byte records (player, two counters,
  expiry season, nation) in a sorted array with a per-player
  held-flag; a periodic processor grants when the player's
  qualifying date is within five days of today and otherwise
  marks a refusal, renewals refresh the expiry and nation and
  clear the counters, and removal swaps in the last record.
  Outcomes surface as queued news events (grant / conditional /
  refusal / expiry / renewal). AI valuation and deadline-date
  tables remain open. (Function-level analysis.)
