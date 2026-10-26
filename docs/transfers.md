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

- Data facts only: `staff_history.dat` records a loan flag plus
  appearances and goals per staff/year/club; player records carry a
  squad number (see [dat-format.md](dat-format.md)). No transfer
  *rule* is verified yet.

## Rules surface (editable targets)

Windows and deadlines per nation, permit rules, loan rules, foreign
limits, squad caps and budget parameters must all become editable
data files with the 01/02 season as default — see "Rules as data" in
[architecture.md](architecture.md). The exact file shapes are drawn
once the rules above are specified.

## Open questions

- Full rule set per item in Coverage, verified by observing the
  original game (bids at various dates, clauses offered, permit
  messages, AI activity over a season).
