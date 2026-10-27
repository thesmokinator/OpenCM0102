# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Michele Broggi
#
# English UI strings for OpenCM0102 (Project Fluent format).
# Fallback language: every other locale falls back to these keys.
# New languages add a sibling directory (e.g. `it/opencm0102.ftl`).

## Setup Game

setup-title = Setup Game
setup-splash-tagline = The 01/02 season, faithfully reborn.
setup-start-new-game = Start New Game
setup-start-new-game-hint = Begin a new career.
setup-restore-saved-game = Restore Saved Game
setup-restore-saved-game-hint = Load a career.
setup-delete-saved-game = Delete Saved Game
setup-delete-saved-game-hint = Remove a career.
setup-game-settings = Game Settings
setup-game-settings-hint = Configure the game.
setup-game-credits = Game Credits
setup-game-credits-hint = Who made this.
setup-select-iso-title = Select the ISO image of your own CM 01/02 disc
setup-select-iso-body =
    Nothing from the original game is bundled with this program.
    If you own the physical CD, create an ISO image of it first
    with your operating system's disc tools.
setup-select-iso-button = Select ISO image…
setup-need-db = Import the database first.
setup-disconnect = Disconnect
setup-iso = ISO image
setup-iso-missing = Not selected
setup-iso-gone = Missing (was { $path })
setup-database = Database
setup-database-ready = Ready ({ $clubs } clubs, { $players } players)
setup-database-missing = Not imported
setup-database-outdated = Outdated (re-import required)
setup-import-title = Import the database
setup-import-body = The ISO is remembered. Import it to unlock new and restored careers.
setup-import-button = Import database
setup-iso-gone-title = ISO image moved away
setup-iso-gone-body = The remembered ISO is gone. Reconnect it or pick the image again.
setup-return-to-game = Return to game
setup-return-to-game-hint = Back to the current career.
setup-tools-title = Tools
setup-tools-hint = Developer utilities and session controls.
shell-menu = Menu
shell-exit-title = Exit game?
shell-exit-body = Unsaved progress will be lost.

## Hub banner and shared actions

app-version = Version { $version }
action-exit-game = Exit Game
tools-data-explorer = Data Explorer
tools-database-editor = Database Editor
tools-data-dir = Game data dir…
tools-pack-loaded = Commentary pack loaded ({ $count } events, { $zones } climate zones).
screen-setup-game = Setup Game
screen-data-explorer = Data Explorer
screen-database-editor = Database Editor
screen-new-career = New Career
screen-load-career = Load Career
screen-main-menu = Main Menu
screen-game-settings = Game Settings
screen-credits = Game Credits

## Main-menu shell

section-squad = Squad
section-tactics = Tactics
section-training = Training
section-fixtures = Fixtures
section-league-table = League Table
section-transfers = Transfers
section-finances-info = Finances & Info
section-board = Board
section-staff = Staff
section-search = Search
section-news = News
section-history = History
section-records = Records
section-awards = Awards
section-club-info = Club Info
shell-holiday-7-days = Holiday (7 days)
shell-save-game = Save game
shell-continue = Continue
shell-squad-title = { $club } — Squad
shell-no-players = No players registered to this club.
shell-th-name = Name
shell-th-role = Role
shell-th-ca = CA
shell-th-pa = PA
shell-not-implemented = Not implemented yet — behaviour is specified in { $spec }.
shell-advanced-to = Advanced to { $date }.
shell-continue-report = { $date } — { $reason }.
shell-stop-own-match = match day
shell-stop-inbox = news waiting
shell-stop-offer = transfer news
shell-stop-deadline = deadline day
shell-stop-max-days = no events in { $days } days
shell-stop-no-calendar = no league calendar
shell-new-season = New season calendar: { $count } fixtures.
shell-match-result = { $home } { $home_goals }–{ $away_goals } { $away }
fixtures-results-title = Results
fixtures-upcoming-title = Upcoming
fixtures-play = Play
fixtures-no-calendar = No league calendar for this club yet.
fixtures-today = Today
fixtures-versus = { $home } v { $away }
fixtures-next-match = Next match: { $match } — { $date }
fixtures-no-report = No match report stored for this fixture.
fixtures-detail-events = Match events
fixtures-detail-ratings = Player ratings
fixtures-th-rating = Rtg
fixtures-th-minutes = Min
fixtures-th-goals = G
fixtures-th-shots = Sh
fixtures-th-passes = Pa
fixtures-th-tackles = Ta
fixtures-th-headers = Hd
fixtures-th-cards = C
stats-shots = Shots
stats-on-target = On target
stats-corners = Corners
stats-free-kicks = Free kicks
stats-throw-ins = Throw-ins
stats-offsides = Offsides
stats-passes = Passes
stats-tackles = Tackles
stats-headers = Headers
matchday-stats-minute = After { $minute } minutes
fixtures-match-day = Match Day
matchday-title = Match Day
matchday-venue = { $venue } · { $date } · { $weather }
matchday-your-eleven = Your eleven
matchday-play-match = Play Match
matchday-speed = Commentary
matchday-speed-very-slow = Very Slow
matchday-speed-slow = Slow
matchday-speed-medium = Medium
matchday-speed-fast = Fast
matchday-speed-very-fast = Very Fast
matchday-tab-overview = Overview
matchday-tab-stats = Stats
matchday-tab-ratings = Ratings
matchday-half-time = Half Time
matchday-second-half = Start Second Half
matchday-full-time = Full Time
matchday-shootout = Shootout { $hs }-{ $as }
matchday-back = Back
matchday-report = Match report
matchday-referee = Referee: { $name }
matchday-attendance = Attendance: { $count }
matchday-goal = GOAL
matchday-red-card = SENT OFF
matchday-live = LIVE
referee-lenient = Lenient
referee-strict = Strict
referee-harsh = Harsh
weather-wind-calm = Calm
weather-wind-breezy = Breezy
weather-wind-gusty = Gusty
weather-wind-strong = Strong wind
weather-wind-gale = Gale
weather-precip-dry = Dry
weather-precip-wet = Wet
weather-precip-drizzle = Drizzle
weather-precip-shower = Showers
weather-precip-downpour = Downpour
weather-precip-sleet = Sleet
weather-precip-light-snow = Light snow
weather-precip-heavy-snow = Heavy snow
weather-pitch-normal = Good pitch
weather-pitch-hard = Hard pitch
weather-pitch-frozen = Frozen pitch
weather-pitch-waterlogged = Waterlogged pitch
news-cat-postponed = Postponed
news-postponed-headline = { $home } v { $away } postponed
news-postponed-body = The match has been rearranged for { $date }: { $reason }.
news-empty = No news yet.
news-cat-match-report = Match report
news-cat-results-roundup = Results
news-cat-injury = Injury
news-cat-suspension = Suspension
news-cat-board = Board
news-roundup-headline = Results roundup
news-roundup-more =
    { $n ->
        [one] …and one more result
       *[other] …and { $n } more results
    }
news-report-no-goals = No goals.
news-injury-headline = { $name } out for { $days } days
news-injury-body = { $name } faces { $days } days on the sidelines.
news-ban-headline = { $name } suspended for { $matches } matches
news-ban-body = { $name } will miss the next { $matches } matches through suspension.
news-board-win-headline = Board pleased with the win
news-board-win-body = The board congratulates the team on the victory.
news-board-draw-headline = Board accepts the draw
news-board-draw-body = The board takes the point and moves on.
news-board-loss-headline = Board disappointed by the defeat
news-board-loss-body = The board expects a response in the next match.
news-queue-title = News ({ $i }/{ $n })
news-next = Next
news-close = Close
tactics-formation = Formation
tactics-auto-pick = Auto-pick best eleven
tactics-starters = Starting eleven
tactics-no-squad = No squad loaded for this club.
tactics-invalid = Lineup incomplete: { $count } of 11 picked.
tactics-reserves = Reserves
tactics-pick-hint = Select a starter, then a reserve to swap them.
tactics-suspended = (susp.)
tactics-injured = (inj.)
table-th-pos = #
table-th-club = Club
table-th-p = P
table-th-w = W
table-th-d = D
table-th-l = L
table-th-gf = GF
table-th-ga = GA
table-th-gd = GD
table-th-pts = Pts
shell-holidayed-to = Holidayed to { $date } (interim: fixed week).
shell-saved-as = Saved as { $file }.
shell-no-active-save = No active save file for this career.

## Game settings

settings-title = Game Settings
settings-language = Language
settings-intro = Settings unlock as their engines land. Planned:
settings-currency = Currency
settings-background-pictures = Background pictures
settings-commentary-speed = Commentary Speed
settings-sound = Sound

## Game credits

credits-title = Game Credits
credits-body =
    Championship Manager 01/02 was developed by Sports Interactive
    and published by Eidos. OpenCM0102 is an independent
    clean-room reimplementation: original engine, original code,
    behaviour specified from observation. You supply your own
    disc image; nothing from the original game ships with this
    program. MIT licensed — see LICENSE.
credits-docs-link = OpenCM0102 documentation (docs/)
credits-repo-link = OpenCM0102 repository

## Shared actions and filters

action-back = Back
action-next = Next
action-prev = Prev
action-cancel = Cancel
action-refresh = Refresh
action-load = Load
action-delete = Delete
action-delete-confirm = Sure?
filter-clubs = Filter clubs…
filter-players = Filter players…
filter-nations = Filter nations…
clubs-count = { $shown } of { $total } clubs
refine-filter = …showing the first { $limit }; refine the filter.
missing-value = —

## Data explorer

explorer-dat-files = { $count } .dat files
explorer-season-date = Season date: { $date }
explorer-manager = Manager: { $name }
explorer-new-career = New career
explorer-no-clubs = No clubs imported yet.
explorer-clubs = Clubs
explorer-players-only = Players only
explorer-players-count = { $shown } of { $total }
explorer-th-born = Born
explorer-player-detail = { $name } — { $role } — CA { $ca } / PA { $pa }
role-player = Player
role-manager = Manager
role-staff = Staff

## Database editor

editor-tables = Tables
editor-search-hint = Search in all columns…
editor-page = { $rows } rows — page { $page } of { $pages }
editor-row-detail = Row detail
editor-blob = <{ $bytes } bytes>

## New-career wizard

career-step = New career — step { $index } of { $total }
career-create-manager = Create manager
career-first-name = First name
career-first-name-hint = Arrigo
career-second-name = Second name
career-second-name-hint = Sacchi
career-need-names = Enter both a first and a second name to continue.
career-select-nationality = Select nationality
career-no-nations = No nations imported yet.
career-nations-count = { $shown } of { $total } nations
career-select-club = Select club
career-save-and-start = Save and start
career-manager-label = Manager
career-nationality-label = Nationality
career-club-label = Club
career-save-name = Save name
career-save-name-hint = My career
career-save-and-start-button = Save & start career
career-pick-first = Pick a nationality and a club first.
career-started = Career started: { $manager } manages { $club }. Saved as { $file }.
career-no-league-data = League data is missing for { $club }. Disconnect and re-import the ISO to populate it.

## Load career

career-back-to-setup = Back to setup
career-no-saves = No saved careers yet. Start a new career first.
career-th-save = Save
career-th-manager = Manager
career-th-club = Club
career-th-date = Date
career-loaded = Loaded { $file }: { $manager }.
career-load-error = { $file }: { $error }
career-refuse-unsafe-delete = Refusing to delete an unsafe file name.
career-cannot-delete = cannot delete save: { $error }
career-cannot-list = cannot list saves: { $error }
career-unknown-club = Club { $id }
career-cannot-create-saves-dir = cannot create saves directory: { $error }
career-cannot-encode = cannot encode save: { $error }
career-cannot-write = cannot write save: { $error }
career-cannot-read = cannot read save: { $error }
career-cannot-decode = cannot decode save: { $error }
career-unsupported-version =
    unsupported save version { $found } (this build reads version { $wanted })

## Application status and ISO import

app-choose-iso = Choose the ISO image of your CM 01/02 disc to begin.
app-iso-missing = The last ISO image was not found at { $path }. Reconnect it or disconnect.
app-no-database = Saved ISO found at { $path }, but no imported database yet. Import it to continue.
app-iso-image = ISO image: { $path }
app-session-restored = Session restored from the imported database.
app-cannot-reopen = cannot reopen the imported database: { $error }
app-disconnected = Disconnected. Choose the ISO image to import again.
app-starting-import = Starting import…
app-import-thread-died = import thread stopped unexpectedly
app-imported = Imported into { $path }. { $count } players and staff available.
app-cannot-reopen-database = cannot reopen database: { $error }
app-progress-opening-iso = Opening ISO image…
app-progress-cabinet = Reading InstallShield cabinet…
app-progress-validating = Validating game data…
app-progress-inventory = Importing file inventory…
app-progress-game-data = Importing clubs, players and every database table…
app-progress-extracting = Extracting game-data files…
app-cannot-create-data-dir = cannot create data directory: { $error }
app-cannot-open-database = cannot open database: { $error }
app-import-failed = import failed: { $error }
app-game-data-failed = game-data import failed: { $error }
app-extraction-failed = extraction failed for { $path }: { $error }
app-cannot-write-file = cannot write extracted file { $file }: { $error }
app-appdata-unset = APPDATA is not set
app-home-unset = HOME is not set
