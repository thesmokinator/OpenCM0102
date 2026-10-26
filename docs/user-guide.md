<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 Michele Broggi -->

# User guide

How to install, import your data and play OpenCM0102.

## What you need

- Your own Championship Manager 01/02 disc, as an **ISO image** file.
  If you own the physical CD, create an ISO of it first with your
  operating system's disc tools.
- The ISO is only ever read, never modified, and no original game
  file is distributed with or bundled into this program.

## First launch: importing your data

1. Start OpenCM0102. The setup wizard asks for the ISO image.
2. Pick the `.iso` file. The import runs in the background (reading
   the disc, extracting the game data, building the database) while
   the window stays responsive.
3. When the import finishes, the data explorer opens.

Your imported data lives in the user data directory:

| OS      | Location                                              |
|---------|-------------------------------------------------------|
| macOS   | `~/Library/Application Support/OpenCM0102/`           |
| Windows | `%APPDATA%\OpenCM0102\`                               |
| Linux   | `$XDG_DATA_HOME/OpenCM0102/` or `~/.local/share/OpenCM0102/` |

It contains `game.db` (the imported database), `extracted/` (the
game-data files), `saves/` (your careers) and `session.txt` (the
remembered ISO position).

On later launches the app reopens the imported database directly.
Keep the ISO where it was: if it moved, the wizard shows an error
and offers Disconnect, which forgets the session so you can import
again.

## Data explorer

The explorer is a read-only browser over your imported data:

- **Clubs**: filterable list (10,580 clubs). With long lists only
  the first 500 matches render — refine the filter to narrow down.
- **Squad**: everyone registered to the selected club, with role,
  birth date and current/potential ability. "Players only" hides
  non-playing staff.
- **Player detail**: clicking a player shows all 54 attributes.
- **Continue** advances the season date by one day. A new game
  starts on 1 July 2001.

## New career

`New career` (button or Settings menu) follows the original game's
flow:

1. **Create manager** — first and second name.
2. **Nationality** — pick from the 213 imported nations.
3. **Club** — pick the team to manage.
4. **Save and start** — review the choices, name the save, start.

The career is saved as `saves/<name>.json` and begins on the first
day of the season. The explorer header shows the manager's name,
and the managed club's squad is selected automatically.

## Load career

`Load career` in the Settings menu lists every saved career with
manager, club and in-game date. `Refresh` rescans the directory.
Loading a save resumes it exactly where it was left.

Saves are versioned: if a save was written by a newer release than
the one you run, loading is refused with an explicit error instead
of misreading it.

## Database editor

`Database editor` in the Settings menu is a read-only viewer over
every imported table: pick a table on the left, search across all
columns, page through rows, click a row for its full detail. It is
the starting point for inspecting the original data set.

## Troubleshooting

- **"The last ISO image was not found"** — the ISO moved. Put it
  back, or Disconnect and import again.
- **"unsupported save version"** — the save needs a newer (or older)
  matching release; it is left untouched.
- **Import fails partway** — check the ISO is a complete image of
  the disc and that the data directory is writable, then retry.
  The originals are never at risk: the import only reads them.
