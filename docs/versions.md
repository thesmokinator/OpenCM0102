<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# Target versions

Two versions matter, for different things:

- **v3.9.60** — the original CD install. It is our *data baseline*:
  the ISO import reads a 3.9.60 disc, and every VERIFIED-DATA note in
  `docs/` describes 3.9.60 bytes unless stated otherwise.
- **v3.9.68** — the final official SI Games patch. It is our
  *behaviour target*: where a patch fixed a 3.9.60 bug, we reproduce
  the fixed behaviour, never the bug. Rules defaults, specs and tests
  describe 3.9.68.

## Official patch history (cumulative, 3.9.60 → 3.9.68)

Summarised from the SI Games version history shipped with the patch.
Each item maps to the spec area it affects.

**3.9.62** (save compatible): compensation for non-Bosman-2 moves
(transfers); Brazilian playoffs used for history; Libertadores
Brazilian-team picks; regional prize money (competitions).

**3.9.65** (save compatible): free-kick wall chance, big-club
protection, Man-of-Match rating logic (match engine); non-league
bankruptcy rescue, stadium-expansion guard, debt interest (season);
aging-player move logic (transfers); Asian Super Cup second-leg gap
and Pakistan's section (competitions); `nomemcheck` option,
club-records and news-corruption repairs (saves/app).

**3.9.67**: South Korean league made playable (the competitions
already exist in 3.9.60 data, so this is selection/rules, not new
entries); player/team data updated to 02/03 with the season start
unchanged — clubs 10,580→13,214, persons 132,722→169,342, plus
43 new Asian competitions (see [dat-format.md](dat-format.md)).

**3.9.68**: West Ham strikers (no duplicated staff rows found — fix
mechanism still open, likely engine-side) and Finnish membership
(Yugoslav contamination removed — confirmed data-side);
England/Conference/Scotland interplay, Australian NSL counts (plus
NSL reputation 6→8), Polish relegations, Argentine Second Division
(competitions); Team-of-the-Week surnames (original UI bug — not
reproduced, our UI is new); language support incl. Chinese,
Brazilian Portuguese and Mexican Spanish (i18n — English-first for
us); Korean media strings and data; ARSE command (unidentified —
open).

Confirmed on the patched executable's shipped text: South Korean
and Japanese league content, the new languages, and a save-version
validation step (`save_game_version` id check).

## Consequences

- Input is the 3.9.60 ISO as-is: the user never installs a patch.
  All patch knowledge serves the reimplementation only — fixed
  behaviours are baked into the engine and rules, and documented
  data-bug corrections (e.g. Finland) apply at import as editable
  rules. No patch bytes ship with us or are needed at runtime.
- Specs in `docs/` describe 3.9.68 behaviour wherever the difference
  is known; anything still verified only on 3.9.60 is marked so until
  the patched game confirms it.
- Original-format save compatibility targets 3.9.68 saves (see
  [saves.md](saves.md)); most saves in circulation are that version.
- Code-level diff of the fixed mechanics (free-kick chance, MoM
  logic, interest severity, selection interplay) is still open: it
  needs playtest observation against the patched game (see
  [roadmap.md](roadmap.md)).
