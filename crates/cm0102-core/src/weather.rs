// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Match weather: the `weather.cfg` format (climate zones with
//! seasonal wind/precipitation/warmth tables), the per-match roll,
//! and the default nation-to-zone mapping.
//!
//! Format (zone count, then per zone four one-line seasons):
//! ```text
//! 44
//! 0 English Coastal
//! SPRING: 45, W75, W85, W95, W99, W100, P50, P70, P85, P90, P100, H-1, H12, ...
//! SUMMER: 120, ...
//! AUTUMN: 272, ...
//! WINTER: 341, ...
//! ```
//! Each season line carries its start day-of-year and the cumulative
//! W/P/H tables (rolls of 1-100 pick the first bucket the roll fits
//! in; -1 marks an impossible bucket). Match bytes are wind 0-4
//! (Calm..Gale) and precipitation 0-9 (Dry, Wet, Drizzle, Shower,
//! Downpour at 5, Sleet, Light and Heavy Snow at 8-9; 4 is unused),
//! plus a warmth index.
//!
//! How precipitation buckets become snow, and how zones attach to
//! clubs, was not observed: the mapping below is our documented
//! reconstruction and stays editable data.

use serde::{Deserialize, Serialize};

/// One season slice of a climate zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeasonSlice {
    /// Day-of-year the season starts (1-based).
    pub start_day: u16,
    /// Cumulative wind table: Calm, Breezy, Gusty, Strong, Gale.
    pub wind: [u8; 5],
    /// Cumulative precipitation table: Dry, Wet, Drizzle, Shower,
    /// Downpour.
    pub precip: [u8; 5],
    /// Cumulative warmth table: Freezing, Cold, Mild, Fine, Warm,
    /// Hot, Very Hot. 0 marks an impossible band.
    pub warmth: [u8; 7],
}

/// One climate zone: four seasonal slices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClimateZone {
    /// Zone name from the file.
    pub name: String,
    /// Four seasons in file order.
    pub seasons: Vec<SeasonSlice>,
}

/// Parsed weather table: zones by file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeatherTable {
    /// Climate zones.
    pub zones: Vec<ClimateZone>,
}

/// What went wrong parsing a table, with a 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherError {
    /// Offending line, 1-based (0 when unknown).
    pub line: usize,
    /// What was expected.
    pub message: String,
}

impl std::fmt::Display for WeatherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for WeatherError {}

impl WeatherTable {
    /// Parse a table from cfg text.
    pub fn parse(text: &str) -> Result<Self, WeatherError> {
        let mut lines = text.lines().enumerate().peekable();
        let mut zones = Vec::new();
        // Skip leading blanks/comments to the zone count.
        let count = loop {
            let Some((index, raw)) = lines.next() else {
                return Err(WeatherError {
                    line: 0,
                    message: "missing zone count".to_string(),
                });
            };
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            break line.parse::<usize>().map_err(|_| WeatherError {
                line: index + 1,
                message: format!("zone count is not a number: {line}"),
            })?;
        };
        for _ in 0..count {
            zones.push(parse_zone(&mut lines)?);
        }
        Ok(Self { zones })
    }

    /// Zone by index, clamped to the table.
    pub fn zone(&self, index: u8) -> Option<&ClimateZone> {
        if self.zones.is_empty() {
            return None;
        }
        Some(&self.zones[(index as usize).min(self.zones.len() - 1)])
    }

    /// Minimal built-in table: one temperate zone, our own mild
    /// numbers, used until the player's file loads.
    pub fn builtin() -> Self {
        let mild = SeasonSlice {
            start_day: 1,
            wind: [70, 90, 96, 99, 100],
            precip: [70, 80, 88, 95, 100],
            warmth: [0, 5, 30, 70, 90, 99, 100],
        };
        Self {
            zones: vec![ClimateZone {
                name: "Temperate".to_string(),
                seasons: vec![mild.clone(), mild.clone(), mild.clone(), mild],
            }],
        }
    }
}

/// Parse one zone: id line, then four one-line seasons.
fn parse_zone(
    lines: &mut std::iter::Peekable<std::iter::Enumerate<std::str::Lines<'_>>>,
) -> Result<ClimateZone, WeatherError> {
    let next_content = |lines: &mut std::iter::Peekable<
        std::iter::Enumerate<std::str::Lines<'_>>,
    >|
     -> Result<(usize, String), WeatherError> {
        loop {
            let Some((index, raw)) = lines.next() else {
                return Err(WeatherError {
                    line: 0,
                    message: "unexpected end of file in zone".to_string(),
                });
            };
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            return Ok((index + 1, line.to_string()));
        }
    };
    let (id_line, id_text) = next_content(lines)?;
    id_text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .parse::<u32>()
        .map_err(|_| WeatherError {
            line: id_line,
            message: format!("zone id is not a number: {id_text}"),
        })?;
    let name = id_text
        .split_whitespace()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ");
    let mut seasons = Vec::with_capacity(4);
    for _ in 0..4 {
        let (line_no, text) = next_content(lines)?;
        seasons.push(parse_season(&text, line_no)?);
    }
    Ok(ClimateZone { name, seasons })
}

/// Parse one season line: `NAME: start, W×5, P×5, H×7`.
fn parse_season(text: &str, line: usize) -> Result<SeasonSlice, WeatherError> {
    let fail = |message: String| WeatherError { line, message };
    let (_, rest) = text.split_once(':').ok_or_else(|| {
        fail(format!("season without NAME: header: {text}"))
    })?;
    let fields: Vec<&str> = rest.split(',').map(str::trim).collect();
    if fields.len() != 18 {
        return Err(fail(format!(
            "season needs 1 start + 17 buckets, has {}",
            fields.len()
        )));
    }
    let start_day = fields[0].parse::<u16>().map_err(|_| {
        fail(format!("season start is not a number: {}", fields[0]))
    })?;
    let number = |field: &str, prefix: char| -> Result<u8, WeatherError> {
        let value = field.strip_prefix(prefix).ok_or_else(|| {
            fail(format!("bucket without {prefix} prefix: {field}"))
        })?;
        if value == "-1" {
            return Ok(0);
        }
        value.parse::<u8>().map_err(|_| {
            fail(format!("bucket is not a number: {field}"))
        })
    };
    let mut wind = [0u8; 5];
    let mut precip = [0u8; 5];
    let mut warmth = [0u8; 7];
    for (i, slot) in wind.iter_mut().enumerate() {
        *slot = number(fields[1 + i], 'W')?;
    }
    for (i, slot) in precip.iter_mut().enumerate() {
        *slot = number(fields[6 + i], 'P')?;
    }
    for (i, slot) in warmth.iter_mut().enumerate() {
        let field = fields[11 + i];
        let value = field.strip_prefix('H').ok_or_else(|| {
            fail(format!("band without H prefix: {field}"))
        })?;
        *slot = if value == "-1" {
            0
        } else {
            value.parse::<u8>().map_err(|_| {
                fail(format!("band is not a number: {field}"))
            })?
        };
    }
    Ok(SeasonSlice {
        start_day,
        wind,
        precip,
        warmth,
    })
}

/// Pitch state derived from warmth and precipitation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Pitch {
    /// Playable, no modifier.
    #[default]
    Normal,
    /// Hard and fast.
    Hard,
    /// Frozen surface.
    Frozen,
    /// Waterlogged surface.
    Waterlogged,
}

/// Per-match weather state: the three observed bytes plus pitch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MatchWeather {
    /// Wind 0-4: Calm, Breezy, Gusty, Strong, Gale.
    pub wind: u8,
    /// Precipitation 0-9: Dry, Wet, Drizzle, Shower, Downpour(5),
    /// Sleet(6), Light Snow(8), Heavy Snow(9).
    pub precip: u8,
    /// Warmth 0-6: Freezing, Cold, Mild, Fine, Warm, Hot, Very Hot.
    pub warmth: u8,
    /// Derived pitch state.
    pub pitch: Pitch,
}

impl MatchWeather {
    /// True for rain or snow that scraps up play.
    pub fn is_wet(&self) -> bool {
        self.precip >= 3
    }

    /// True for winds that move the ball.
    pub fn is_windy(&self) -> bool {
        self.wind >= 3
    }

    /// True for severe surfaces (frozen or waterlogged), whether
    /// or not the fixture survives them.
    pub fn is_severe(&self) -> bool {
        matches!(self.pitch, Pitch::Frozen | Pitch::Waterlogged)
    }
}

/// Pick the season slice covering `day_of_year` (1-based).
fn season_for(zone: &ClimateZone, day_of_year: u16) -> &SeasonSlice {
    let mut best = &zone.seasons[zone.seasons.len() - 1];
    for season in &zone.seasons {
        if season.start_day <= day_of_year {
            best = season;
        }
    }
    best
}

/// Bucket index for a 1-100 roll against a cumulative table.
fn bucket<const N: usize>(table: &[u8; N], roll: u32) -> usize {
    for (i, ceiling) in table.iter().enumerate() {
        if roll <= u32::from(*ceiling) {
            return i;
        }
    }
    N - 1
}

/// Tiny deterministic mixer so weather needs no RNG dependency.
fn mix(seed: u64, salt: u64) -> u32 {
    let mut x = seed.wrapping_add(salt).wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) % 100 + 1) as u32
}

/// Roll match weather for `zone` on `day_of_year`, deterministically
/// from `seed`. Bucket-to-byte mapping is our reconstruction:
/// frozen precipitation falls as snow or sleet, and standing water
/// or frost sets the pitch.
pub fn roll_weather(table: &WeatherTable, zone: u8, day_of_year: u16, seed: u64) -> MatchWeather {
    let Some(climate) = table.zone(zone) else {
        return MatchWeather::default();
    };
    if climate.seasons.is_empty() {
        return MatchWeather::default();
    }
    let season = season_for(climate, day_of_year);
    let wind = bucket(&season.wind, mix(seed, 1)) as u8;
    let wet = bucket(&season.precip, mix(seed, 2));
    let warmth = bucket(&season.warmth, mix(seed, 3)) as u8;
    let precip = match (warmth, wet) {
        (0, 0) => 0,
        // Freezing wet falls as sleet or snow.
        (0, 1) => 6,
        (0, 2 | 3) => 8,
        (0, _) => 9,
        (_, 0) => 0,
        (_, 1) => 1,
        (_, 2) => 2,
        (_, 3) => 3,
        (_, _) => 5,
    };
    let pitch = if warmth == 0 && wet >= 1 {
        Pitch::Frozen
    } else if precip == 5 || precip == 9 {
        Pitch::Waterlogged
    } else if (warmth >= 5 || warmth == 0) && wet == 0 {
        Pitch::Hard
    } else {
        Pitch::Normal
    };
    MatchWeather {
        wind,
        precip,
        warmth,
        pitch,
    }
}

/// Share of frozen pitches bad enough to postpone (a cold snap
/// freezes the ground solid; most frosts stay playable).
pub const FROZEN_POSTPONE_PCT: u32 = 20;
/// Share of waterlogged pitches bad enough to postpone (standing
/// water after prolonged rain; most downpours drain away).
pub const WATERLOGGED_POSTPONE_PCT: u32 = 10;

/// True when the pitch calls the fixture off. Severe surfaces
/// only threaten; the call itself is a rare deterministic roll
/// from the fixture seed, so replays roll again.
pub fn is_postponed(weather: &MatchWeather, seed: u64) -> bool {
    match weather.pitch {
        Pitch::Frozen => mix(seed, 11) <= FROZEN_POSTPONE_PCT,
        Pitch::Waterlogged => mix(seed, 12) <= WATERLOGGED_POSTPONE_PCT,
        Pitch::Normal | Pitch::Hard => false,
    }
}

/// Default climate zone for a nation name. Our own mapping
/// (editable data): the original's club-to-zone wiring was not
/// observed, so each nation takes the zone whose anchor city best
/// matches its football heartland.
pub fn nation_zone(nation: &str) -> u8 {
    match nation {
        "England" => 1,
        "Scotland" => 7,
        "Wales" | "N. Ireland" | "Northern Ireland" | "Rep. of Ireland" | "Republic of Ireland"
        | "Ireland" => 0,
        "Italy" => 9,
        "Spain" => 9,
        "Portugal" => 9,
        "Greece" => 9,
        "Turkey" => 9,
        "France" => 1,
        "Germany" => 5,
        "Austria" => 5,
        "Switzerland" => 5,
        "Poland" => 5,
        "Czech Republic" => 5,
        "Russia" => 5,
        "Holland" => 1,
        "Belgium" => 1,
        "Denmark" => 1,
        "Norway" | "Sweden" | "Finland" | "Iceland" => 4,
        "Croatia" | "Serbia" | "Yugoslavia" | "Romania" | "Bulgaria" | "Hungary" => 3,
        "Cyprus" | "Israel" => 10,
        "Morocco" | "Algeria" | "Tunisia" | "Egypt" => 10,
        "U.S.A." | "USA" | "United States" => 30,
        "Canada" => 6,
        "Mexico" => 17,
        "Brazil" => 12,
        "Argentina" | "Uruguay" | "Chile" | "Paraguay" => 11,
        "Colombia" | "Venezuela" | "Peru" | "Ecuador" | "Bolivia" => 25,
        "Japan" => 8,
        "South Korea" | "Korea" => 19,
        "China" | "China PR" => 19,
        "Australia" => 34,
        "South Africa" => 29,
        _ => 1,
    }
}

/// Competition nation for a league slot (`club.dat` byte 87),
/// observed in the data files. Unknown slots read as England.
pub fn league_nation(slot: u32) -> &'static str {
    match slot {
        0..=3 | 107 => "Belgium",
        4..=6 => "Denmark",
        7..=10 | 93 => "England",
        11..=15 => "France",
        16 | 17 | 20 | 21 | 105 => "Germany",
        22 | 23 | 102 => "Holland",
        24..=30 => "Italy",
        31..=33 | 90 => "United States",
        34..=37 | 101 => "Scotland",
        38..=45 | 97 | 108 => "Sweden",
        46..=51 | 106 => "Portugal",
        52..=57 | 98 | 103 => "Spain",
        63 | 64 | 88 => "Argentina",
        65 | 79 | 80 | 254 => "Brazil",
        69 | 82..=84 | 100 => "Japan",
        114..=118 => "Finland",
        119..=127 => "Republic of Ireland",
        133..=137 | 198 => "Poland",
        138 | 140 | 141 | 146..=150 | 152 | 153 | 197 => "Croatia",
        142..=145 => "Greece",
        151 | 162..=166 => "Australia",
        154..=161 => "Northern Ireland",
        167..=174 => "Turkey",
        176..=184 | 190 => "Russia",
        186..=189 => "Wales",
        194..=196 | 200 => "China PR",
        201 | 238 | 239 | 241 | 244 => "South Africa",
        206 | 207 | 221..=228 => "Czech Republic",
        208 | 210 => "Yugoslavia",
        214..=216 => "Mexico",
        229 | 231 | 232 | 234..=236 => "South Korea",
        245 | 246 | 248 | 249 => "Austria",
        250..=252 => "Switzerland",
        _ => "England",
    }
}

/// Climate zone for a league slot (`club.dat` byte 87): the slot's
/// competition nation, observed in the data files, through
/// [`nation_zone`]. Unknown slots play in English weather.
pub fn league_zone(slot: u32) -> u8 {
    nation_zone(league_nation(slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "1\n0 Test Coast\nSPRING: 1, W70, W90, W96, W99, W100, P70, P80, P88, P95, P100, H0, H5, H30, H70, H90, H99, H100\nSUMMER: 90, W70, W90, W96, W99, W100, P70, P80, P88, P95, P100, H0, H5, H30, H70, H90, H99, H100\nAUTUMN: 180, W70, W90, W96, W99, W100, P70, P80, P88, P95, P100, H0, H5, H30, H70, H90, H99, H100\nWINTER: 270, W70, W90, W96, W99, W100, P70, P80, P88, P95, P100, H0, H5, H30, H70, H90, H99, H100\n";

    #[test]
    fn parses_zones_and_seasons() {
        let table = WeatherTable::parse(SAMPLE).expect("parses");
        assert_eq!(table.zones.len(), 1);
        assert_eq!(table.zones[0].name, "Test Coast");
        assert_eq!(table.zones[0].seasons.len(), 4);
        assert_eq!(table.zones[0].seasons[0].wind, [70, 90, 96, 99, 100]);
        assert_eq!(table.zones[0].seasons[0].warmth[0], 0);
        assert_eq!(table.zones[0].seasons[0].warmth[3], 70);
    }

    #[test]
    fn rolls_are_deterministic_and_bounded() {
        let table = WeatherTable::parse(SAMPLE).expect("parses");
        let first = roll_weather(&table, 0, 200, 99);
        let second = roll_weather(&table, 0, 200, 99);
        assert_eq!(first, second);
        assert!(first.wind <= 4);
        assert!(first.precip <= 9);
        assert!(first.warmth <= 6);
    }

    #[test]
    fn seasons_cover_the_year() {
        let table = WeatherTable::parse(SAMPLE).expect("parses");
        let zone = &table.zones[0];
        assert_eq!(season_for(zone, 0).start_day, 270);
        assert_eq!(season_for(zone, 1).start_day, 1);
        assert_eq!(season_for(zone, 100).start_day, 90);
        assert_eq!(season_for(zone, 365).start_day, 270);
    }

    #[test]
    fn nation_mapping_covers_home_nations() {
        assert_eq!(nation_zone("England"), 1);
        assert_eq!(nation_zone("Scotland"), 7);
        assert_eq!(nation_zone("Atlantis"), 1);
    }

    #[test]
    fn original_table_parses_when_present() {
        let Some(dir) = std::env::var_os("OPENCM0102_DATA_DIR") else {
            eprintln!("skipping: OPENCM0102_DATA_DIR unset");
            return;
        };
        let path = std::path::Path::new(&dir).join("weather.cfg");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} unreadable", path.display());
            return;
        };
        let text = String::from_utf8_lossy(&bytes);
        eprintln!("parsing {} ({} bytes)", path.display(), bytes.len());
        let table = WeatherTable::parse(&text).expect("original parses");
        assert_eq!(table.zones.len(), 44);
        assert!(table.zones.iter().all(|zone| zone.seasons.len() == 4));
    }

    #[test]
    fn league_slots_map_to_home_nations() {
        assert_eq!(league_zone(7), 1);
        assert_eq!(league_zone(10), 1);
        assert_eq!(league_zone(34), 7);
        assert_eq!(league_zone(24), 9);
        assert_eq!(league_zone(52), 9);
        assert_eq!(league_zone(176), 5);
        assert_eq!(league_zone(69), 8);
        assert_eq!(league_zone(63), 11);
        assert_eq!(league_zone(999), 1);
    }

    #[test]
    fn builtin_table_rolls_mild_weather() {
        let table = WeatherTable::builtin();
        assert_eq!(table.zones.len(), 1);
        let calls = (0..1000u64)
            .filter(|seed| {
                let weather = roll_weather(&table, 0, 200, *seed);
                is_postponed(&weather, *seed)
            })
            .count();
        assert!(calls <= 20, "builtin calls: {calls}");
    }

    #[test]
    fn only_severe_pitches_postpone() {
        let calm = MatchWeather::default();
        assert!(!calm.is_severe());
        for seed in 0..200u64 {
            assert!(!is_postponed(&calm, seed));
        }
        let frozen = MatchWeather {
            pitch: Pitch::Frozen,
            ..MatchWeather::default()
        };
        assert!(frozen.is_severe());
        let calls = (0..2000u64)
            .filter(|seed| is_postponed(&frozen, *seed))
            .count();
        assert!((300..500).contains(&calls), "frozen calls: {calls}");
    }

    #[test]
    fn january_postponements_are_rare_but_present() {
        let Some(dir) = std::env::var_os("OPENCM0102_DATA_DIR") else {
            eprintln!("skipping: OPENCM0102_DATA_DIR unset");
            return;
        };
        let path = std::path::Path::new(&dir).join("weather.cfg");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} unreadable", path.display());
            return;
        };
        let table =
            WeatherTable::parse(&String::from_utf8_lossy(&bytes)).expect("original parses");
        let calls = (0..4000u64)
            .filter(|seed| {
                let weather = roll_weather(&table, 1, 15, *seed);
                is_postponed(&weather, *seed)
            })
            .count();
        assert!((20..150).contains(&calls), "january calls: {calls}");
    }
}
