// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Parsers for the CM 01/02 `.dat` files extracted from the cabinet.
//!
//! Layouts here were derived empirically from the real freeware data and
//! cross-checked against known entities (see `docs/dat-format.md`):
//!
//! - Name files (`first_names.dat`, `second_names.dat`,
//!   `common_names.dat`): fixed 60-byte records — name in the first 51
//!   bytes (NUL padded), the record index as `i32` at byte 51, another
//!   `i32` at byte 55 and a usage counter at byte 59. The record index is
//!   the id other files reference.
//! - `club.dat`: fixed 581-byte records — club id `i32` at byte 0, long
//!   name at byte 4 (52-byte field), short name at byte 56.
//! - `staff.dat`: fixed 157-byte records (packed) — staff id `i32` at
//!   byte 0, first-name id at byte 4, second-name id at byte 8,
//!   common-name id at byte 12, club id `i32` at byte 57 (unaligned,
//!   negative when unattached). The file may end with a trailing partial
//!   record, which is ignored.
//!
//! Ability fields (CA/PA) are not yet located in the staff record, so
//! parsed players carry `None` abilities until that layout is documented.

use cm0102_core::{Club, Player};

use crate::{DbError, Result};

/// Record size of the three name files.
const NAME_RECORD_SIZE: usize = 60;
/// Length of the name field inside a name record.
const NAME_FIELD_SIZE: usize = 51;
/// Record size of `club.dat`.
const CLUB_RECORD_SIZE: usize = 581;
/// Record size of `staff.dat`.
const STAFF_RECORD_SIZE: usize = 157;

/// Read a NUL-terminated Latin-1 string from a fixed-width field.
fn field_string(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    field[..end].iter().map(|byte| *byte as char).collect()
}

/// Read a little-endian `i32`, tolerating unaligned offsets (packed data).
fn field_i32(data: &[u8], offset: usize) -> Option<i32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(i32::from_le_bytes(bytes))
}

/// Parse one of the name files into an index-addressable vector.
///
/// Entry `i` is the name other files reference with id `i`; entry 0 is
/// conventionally empty.
pub fn parse_names(data: &[u8]) -> Vec<String> {
    data.chunks_exact(NAME_RECORD_SIZE)
        .map(|record| field_string(&record[..NAME_FIELD_SIZE]))
        .collect()
}

/// Parse `club.dat` into clubs, skipping records without a name.
pub fn parse_clubs(data: &[u8]) -> Result<Vec<Club>> {
    let mut clubs = Vec::new();
    for record in data.chunks_exact(CLUB_RECORD_SIZE) {
        let id = field_i32(record, 0)
            .ok_or_else(|| DbError::InvalidIso("truncated club record".to_string()))?;
        let name = field_string(&record[4..56]);
        if id < 0 || name.is_empty() {
            continue;
        }
        clubs.push(Club {
            id: id as u32,
            name,
        });
    }
    Ok(clubs)
}

/// Job code of a player in the staff record.
const JOB_PLAYER: u8 = 11;
/// Record size of the non-player section inside `staff.dat`.
const NON_PLAYER_RECORD_SIZE: usize = 68;
/// Record size of the player section inside `staff.dat`.
const PLAYER_RECORD_SIZE: usize = 70;
/// Number of stored player attributes.
const ATTRIBUTE_COUNT: usize = 54;

/// Parse `staff.dat` (all sections) into people with abilities.
///
/// The file stores, in order: 157-byte person records (the run whose id
/// equals the record index), then 68-byte non-player records, then
/// 70-byte player records; preferences are embedded in the person
/// record in this data version. Section sizes are derived from the
/// classification byte at offset 95 of each person record
/// (1 = non-player, 2 = player, 3 = both): the k-th person of a
/// classification owns the k-th record of the matching section.
/// Verified on the real file, where the three sections account for
/// every remaining byte exactly.
///
/// Verified person fields: date of birth as zero-based day-of-year
/// `u16` at byte 16 plus year `i16` at byte 18, club id `i32` at
/// byte 57, job code at byte 61 (11 = player, 5 = manager).
/// Verified player fields: squad number at byte 4, current/potential
/// ability `i16` at bytes 5/7, home/current/world reputation `i16`
/// at bytes 9/11/13, and the 54 attributes at bytes 15..69.
/// Non-player records carry abilities at bytes 4/6 and reputations
/// at bytes 8/10/12.
pub fn parse_staff(
    data: &[u8],
    first_names: &[String],
    second_names: &[String],
    common_names: &[String],
) -> Vec<Player> {
    // First pass: every person record in the opening section.
    let mut people: Vec<Player> = Vec::new();
    let mut classifications: Vec<u8> = Vec::new();
    for (index, record) in data.chunks_exact(STAFF_RECORD_SIZE).enumerate() {
        // The staff section ends when the sequential id run breaks.
        if field_i32(record, 0) != Some(index as i32) {
            break;
        }
        let (Some(id), Some(first_id), Some(second_id), Some(common_id), Some(club_id)) = (
            field_i32(record, 0),
            field_i32(record, 4),
            field_i32(record, 8),
            field_i32(record, 12),
            field_i32(record, 57),
        ) else {
            classifications.push(0);
            people.push(Player {
                id: index as u32,
                first_name: String::new(),
                second_name: String::new(),
                club_id: None,
                current_ability: None,
                potential_ability: None,
                squad_number: None,
                home_reputation: None,
                current_reputation: None,
                world_reputation: None,
                attributes: Vec::new(),
                birth_date: None,
                job: 0,
                is_player: false,
            });
            continue;
        };
        let lookup = |table: &[String], index: i32| -> Option<String> {
            usize::try_from(index)
                .ok()
                .and_then(|index| table.get(index).cloned())
                .filter(|name| !name.is_empty())
        };
        let second_name = lookup(second_names, second_id).unwrap_or_default();
        let first_name = if common_id > 0 {
            lookup(common_names, common_id)
                .or_else(|| lookup(first_names, first_id))
                .unwrap_or_default()
        } else {
            lookup(first_names, first_id).unwrap_or_default()
        };
        let birth_date = match (
            record
                .get(16..18)
                .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap())),
            record
                .get(18..20)
                .map(|bytes| i16::from_le_bytes(bytes.try_into().unwrap())),
        ) {
            (Some(day), Some(year)) => {
                cm0102_core::GameDate::from_day_of_year(i32::from(year), u32::from(day))
            }
            _ => None,
        };
        let job = record.get(61).copied().unwrap_or(0);
        classifications.push(record.get(95).copied().unwrap_or(0));
        people.push(Player {
            id: id.max(0) as u32,
            first_name,
            second_name,
            club_id: u32::try_from(club_id).ok(),
            current_ability: None,
            potential_ability: None,
            squad_number: None,
            home_reputation: None,
            current_reputation: None,
            world_reputation: None,
            attributes: Vec::new(),
            birth_date,
            job,
            is_player: job == JOB_PLAYER,
        });
    }

    // Second pass: merge the non-player and player sections by rank.
    let staff_bytes = people.len() * STAFF_RECORD_SIZE;
    let non_player_count = classifications
        .iter()
        .filter(|class| **class == 1 || **class == 3)
        .count();
    let player_start = staff_bytes + non_player_count * NON_PLAYER_RECORD_SIZE;
    let mut non_player_rank = 0usize;
    let mut player_rank = 0usize;
    for (person, class) in people.iter_mut().zip(classifications.iter()) {
        if *class == 1 || *class == 3 {
            let offset = staff_bytes + non_player_rank * NON_PLAYER_RECORD_SIZE;
            non_player_rank += 1;
            if let Some(record) = data.get(offset..offset + NON_PLAYER_RECORD_SIZE) {
                person.current_ability = field_i16(record, 4);
                person.potential_ability = field_i16(record, 6);
                person.home_reputation = field_i16(record, 8);
                person.current_reputation = field_i16(record, 10);
                person.world_reputation = field_i16(record, 12);
            }
        }
        if *class == 2 || *class == 3 {
            let offset = player_start + player_rank * PLAYER_RECORD_SIZE;
            player_rank += 1;
            if let Some(record) = data.get(offset..offset + PLAYER_RECORD_SIZE) {
                person.squad_number = record.get(4).copied();
                person.current_ability = field_i16(record, 5);
                person.potential_ability = field_i16(record, 7);
                person.home_reputation = field_i16(record, 9);
                person.current_reputation = field_i16(record, 11);
                person.world_reputation = field_i16(record, 13);
                if let Some(attributes) = record.get(15..15 + ATTRIBUTE_COUNT) {
                    person.attributes = attributes.to_vec();
                }
            }
        }
    }

    people
        .into_iter()
        .filter(|person| !person.first_name.is_empty() && !person.second_name.is_empty())
        .collect()
}

/// Read a little-endian `i16` at `offset`, if present.
fn field_i16(data: &[u8], offset: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(offset..offset + 2)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

/// Record size of `nation.dat`.
const NATION_RECORD_SIZE: usize = 290;
/// Record size of `city.dat`.
const CITY_RECORD_SIZE: usize = 56;
/// Record size of `stadium.dat`.
const STADIUM_RECORD_SIZE: usize = 78;
/// Record size of `continent.dat`.
const CONTINENT_RECORD_SIZE: usize = 198;
/// Record size of `colour.dat`.
const COLOUR_RECORD_SIZE: usize = 58;
/// Record size of the competition files (`club_comp.dat`,
/// `nation_comp.dat`, ids share one space across both).
const COMPETITION_RECORD_SIZE: usize = 107;
/// Record size of `staff_comp.dat`.
const STAFF_COMP_RECORD_SIZE: usize = 101;
/// Record size of `officials.dat`.
const OFFICIAL_RECORD_SIZE: usize = 43;
/// Record size of the competition history files.
const COMP_HISTORY_RECORD_SIZE: usize = 26;
/// Record size of `staff_history.dat`.
const STAFF_HISTORY_RECORD_SIZE: usize = 17;
/// Record size of `staff_comp_history.dat`.
const STAFF_COMP_HISTORY_RECORD_SIZE: usize = 58;

/// A nation from `nation.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nation {
    /// Nation id.
    pub id: u32,
    /// Full name, e.g. `Italy`.
    pub name: String,
    /// Short name.
    pub short_name: String,
    /// Three-letter code, e.g. `ITA`.
    pub three_letter: String,
    /// Continent id.
    pub continent_id: Option<u32>,
}

/// A city from `city.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct City {
    /// City id.
    pub id: u32,
    /// City name.
    pub name: String,
    /// Nation id.
    pub nation_id: Option<u32>,
}

/// A stadium from `stadium.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stadium {
    /// Stadium id.
    pub id: u32,
    /// Stadium name.
    pub name: String,
    /// City id.
    pub city_id: Option<u32>,
    /// Total capacity.
    pub capacity: Option<i32>,
    /// Seated capacity.
    pub seating_capacity: Option<i32>,
    /// Expansion capacity.
    pub expansion_capacity: Option<i32>,
}

/// A continent from `continent.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Continent {
    /// Continent id.
    pub id: u32,
    /// Continent name.
    pub name: String,
    /// Three-letter code.
    pub three_letter: String,
    /// Federation name.
    pub federation_name: String,
}

/// A colour from `colour.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colour {
    /// Colour id.
    pub id: u32,
    /// Colour name.
    pub name: String,
    /// Red intensity.
    pub red: u8,
    /// Green intensity.
    pub green: u8,
    /// Blue intensity.
    pub blue: u8,
}

/// A competition from `club_comp.dat`, `nation_comp.dat` or
/// `staff_comp.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Competition {
    /// Competition id (shared across club and nation competitions).
    pub id: u32,
    /// Full name.
    pub name: String,
    /// Short name.
    pub short_name: String,
    /// Three-letter code.
    pub three_letter: String,
    /// Nation id, when national in scope.
    pub nation_id: Option<u32>,
    /// Continent id.
    pub continent_id: Option<u32>,
    /// Reputation.
    pub reputation: Option<i16>,
}

/// An official (referee) from `officials.dat`, names resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Official {
    /// Official id.
    pub id: u32,
    /// First name.
    pub first_name: String,
    /// Second name.
    pub second_name: String,
    /// Nation id.
    pub nation_id: Option<u32>,
    /// City id.
    pub city_id: Option<u32>,
    /// Current ability.
    pub current_ability: Option<i16>,
    /// Potential ability.
    pub potential_ability: Option<i16>,
    /// Reputation.
    pub reputation: Option<i16>,
}

/// One row of a competition history file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitionHistory {
    /// Row id (shared across club and nation histories).
    pub id: u32,
    /// Competition id.
    pub competition_id: Option<u32>,
    /// Season year.
    pub year: Option<i16>,
    /// Winning club id.
    pub winners_club_id: Option<u32>,
    /// Runner-up club id.
    pub runners_up_club_id: Option<u32>,
    /// Third-placed club id.
    pub third_club_id: Option<u32>,
    /// Host club id.
    pub hosts_club_id: Option<u32>,
}

/// One row of `staff_history.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffHistory {
    /// Row id.
    pub id: u32,
    /// Staff (person) id.
    pub staff_id: Option<u32>,
    /// Season year.
    pub year: Option<i16>,
    /// Club id.
    pub club_id: Option<u32>,
    /// Whether the spell was a loan.
    pub on_loan: bool,
    /// Appearances.
    pub apps: u8,
    /// Goals.
    pub goals: u8,
}

/// One row of `staff_comp_history.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffCompHistory {
    /// Row id.
    pub id: u32,
    /// Staff competition id.
    pub competition_id: Option<u32>,
    /// Season year.
    pub year: Option<i16>,
    /// First-placed staff id.
    pub first_staff_id: Option<u32>,
    /// Second-placed staff id.
    pub second_staff_id: Option<u32>,
    /// Third-placed staff id.
    pub third_staff_id: Option<u32>,
}

/// Parse a file of fixed records whose id is the sequential record
/// index, returning each record's raw bytes.
fn sequential_records(data: &[u8], record_size: usize) -> Vec<&[u8]> {
    data.chunks_exact(record_size)
        .enumerate()
        .take_while(|(index, record)| field_i32(record, 0) == Some(*index as i32))
        .map(|(_, record)| record)
        .collect()
}

/// Parse `nation.dat`.
pub fn parse_nations(data: &[u8]) -> Vec<Nation> {
    sequential_records(data, NATION_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..55]);
            if name.is_empty() {
                return None;
            }
            Some(Nation {
                id: field_i32(record, 0)? as u32,
                name,
                short_name: field_string(&record[56..82]),
                three_letter: field_string(&record[83..87]),
                continent_id: field_i32(record, 113).and_then(|v| u32::try_from(v).ok()),
            })
        })
        .collect()
}

/// Parse `city.dat`.
pub fn parse_cities(data: &[u8]) -> Vec<City> {
    sequential_records(data, CITY_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..30]);
            if name.is_empty() {
                return None;
            }
            Some(City {
                id: field_i32(record, 0)? as u32,
                name,
                nation_id: field_i32(record, 31).and_then(|v| u32::try_from(v).ok()),
            })
        })
        .collect()
}

/// Parse `stadium.dat`.
pub fn parse_stadiums(data: &[u8]) -> Vec<Stadium> {
    sequential_records(data, STADIUM_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..55]);
            if name.is_empty() {
                return None;
            }
            Some(Stadium {
                id: field_i32(record, 0)? as u32,
                name,
                city_id: field_i32(record, 56).and_then(|v| u32::try_from(v).ok()),
                capacity: field_i32(record, 60),
                seating_capacity: field_i32(record, 64),
                expansion_capacity: field_i32(record, 68),
            })
        })
        .collect()
}

/// Parse `continent.dat`.
pub fn parse_continents(data: &[u8]) -> Vec<Continent> {
    sequential_records(data, CONTINENT_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..30]);
            if name.is_empty() {
                return None;
            }
            Some(Continent {
                id: field_i32(record, 0)? as u32,
                name,
                three_letter: field_string(&record[31..35]),
                federation_name: field_string(&record[61..162]),
            })
        })
        .collect()
}

/// Parse `colour.dat`.
pub fn parse_colours(data: &[u8]) -> Vec<Colour> {
    sequential_records(data, COLOUR_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..55]);
            if name.is_empty() {
                return None;
            }
            Some(Colour {
                id: field_i32(record, 0)? as u32,
                name,
                red: record[55],
                green: record[56],
                blue: record[57],
            })
        })
        .collect()
}

/// Parse one competition file (`club_comp.dat` or `nation_comp.dat`).
/// Nation competitions continue the club competition id space, so ids
/// do not start at zero in that file.
pub fn parse_competitions(data: &[u8]) -> Vec<Competition> {
    data.chunks_exact(COMPETITION_RECORD_SIZE)
        .filter_map(|record| {
            let name = field_string(&record[4..55]);
            if name.is_empty() {
                return None;
            }
            Some(Competition {
                id: field_i32(record, 0)? as u32,
                name,
                short_name: field_string(&record[56..82]),
                three_letter: field_string(&record[83..87]),
                nation_id: field_i32(record, 93).and_then(|v| u32::try_from(v).ok()),
                continent_id: field_i32(record, 89).and_then(|v| u32::try_from(v).ok()),
                reputation: field_i16(record, 105),
            })
        })
        .collect()
}

/// Parse `staff_comp.dat` (staff competitions and awards).
pub fn parse_staff_competitions(data: &[u8]) -> Vec<Competition> {
    sequential_records(data, STAFF_COMP_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let name = field_string(&record[4..55]);
            if name.is_empty() {
                return None;
            }
            Some(Competition {
                id: field_i32(record, 0)? as u32,
                name,
                short_name: field_string(&record[56..82]),
                three_letter: String::new(),
                nation_id: field_i32(record, 87).and_then(|v| u32::try_from(v).ok()),
                continent_id: field_i32(record, 83).and_then(|v| u32::try_from(v).ok()),
                reputation: field_i16(record, 99),
            })
        })
        .collect()
}

/// Parse `officials.dat`, resolving names through the name tables.
pub fn parse_officials(
    data: &[u8],
    first_names: &[String],
    second_names: &[String],
) -> Vec<Official> {
    sequential_records(data, OFFICIAL_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            let lookup = |table: &[String], index: i32| -> String {
                usize::try_from(index)
                    .ok()
                    .and_then(|index| table.get(index).cloned())
                    .unwrap_or_default()
            };
            let first_name = lookup(first_names, field_i32(record, 4)?);
            let second_name = lookup(second_names, field_i32(record, 8)?);
            if second_name.is_empty() {
                return None;
            }
            Some(Official {
                id: field_i32(record, 0)? as u32,
                first_name,
                second_name,
                nation_id: field_i32(record, 22).and_then(|v| u32::try_from(v).ok()),
                city_id: field_i32(record, 26).and_then(|v| u32::try_from(v).ok()),
                current_ability: field_i16(record, 30),
                potential_ability: field_i16(record, 32),
                reputation: field_i16(record, 34),
            })
        })
        .collect()
}

/// Parse one competition history file. Nation history rows continue
/// the club history id space, so ids do not start at zero there.
pub fn parse_competition_history(data: &[u8]) -> Vec<CompetitionHistory> {
    data.chunks_exact(COMP_HISTORY_RECORD_SIZE)
        .filter_map(|record| {
            Some(CompetitionHistory {
                id: field_i32(record, 0)? as u32,
                competition_id: field_i32(record, 4).and_then(|v| u32::try_from(v).ok()),
                year: field_i16(record, 8),
                winners_club_id: field_i32(record, 10).and_then(|v| u32::try_from(v).ok()),
                runners_up_club_id: field_i32(record, 14).and_then(|v| u32::try_from(v).ok()),
                third_club_id: field_i32(record, 18).and_then(|v| u32::try_from(v).ok()),
                hosts_club_id: field_i32(record, 22).and_then(|v| u32::try_from(v).ok()),
            })
        })
        .collect()
}

/// Parse `staff_history.dat`.
pub fn parse_staff_history(data: &[u8]) -> Vec<StaffHistory> {
    sequential_records(data, STAFF_HISTORY_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            Some(StaffHistory {
                id: field_i32(record, 0)? as u32,
                staff_id: field_i32(record, 4).and_then(|v| u32::try_from(v).ok()),
                year: field_i16(record, 8),
                club_id: field_i32(record, 10).and_then(|v| u32::try_from(v).ok()),
                on_loan: record[14] != 0,
                apps: record[15],
                goals: record[16],
            })
        })
        .collect()
}

/// Parse `staff_comp_history.dat`.
pub fn parse_staff_comp_history(data: &[u8]) -> Vec<StaffCompHistory> {
    sequential_records(data, STAFF_COMP_HISTORY_RECORD_SIZE)
        .into_iter()
        .filter_map(|record| {
            Some(StaffCompHistory {
                id: field_i32(record, 0)? as u32,
                competition_id: field_i32(record, 4).and_then(|v| u32::try_from(v).ok()),
                year: field_i16(record, 8),
                first_staff_id: field_i32(record, 18).and_then(|v| u32::try_from(v).ok()),
                second_staff_id: field_i32(record, 34).and_then(|v| u32::try_from(v).ok()),
                third_staff_id: field_i32(record, 50).and_then(|v| u32::try_from(v).ok()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build one 60-byte name record with the given text.
    fn name_record(text: &str) -> Vec<u8> {
        let mut record = vec![0u8; NAME_RECORD_SIZE];
        record[..text.len()].copy_from_slice(text.as_bytes());
        record
    }

    /// Nation records expose name, short name, code and continent.
    #[test]
    fn parses_nation_records() {
        let mut record = vec![0u8; NATION_RECORD_SIZE];
        record[4..9].copy_from_slice(b"Italy");
        record[56..61].copy_from_slice(b"Italy");
        record[83..86].copy_from_slice(b"ITA");
        record[113..117].copy_from_slice(&2i32.to_le_bytes());
        let nations = parse_nations(&record);
        assert_eq!(nations.len(), 1);
        assert_eq!(nations[0].name, "Italy");
        assert_eq!(nations[0].three_letter, "ITA");
        assert_eq!(nations[0].continent_id, Some(2));
    }

    /// Name files parse by record index, preserving the empty entry 0.
    #[test]
    fn parses_name_records() {
        let mut data = name_record("");
        data.extend(name_record("Thierry"));
        data.extend(name_record("Patrick"));
        let names = parse_names(&data);
        assert_eq!(names, ["", "Thierry", "Patrick"]);
    }

    /// Club records expose id and long name; nameless records are skipped.
    #[test]
    fn parses_club_records() {
        let mut record = vec![0u8; CLUB_RECORD_SIZE];
        record[0..4].copy_from_slice(&676i32.to_le_bytes());
        record[4..11].copy_from_slice(b"Arsenal");
        let clubs = parse_clubs(&record).unwrap();
        assert_eq!(clubs.len(), 1);
        assert_eq!(clubs[0].id, 676);
        assert_eq!(clubs[0].name, "Arsenal");
    }

    /// Staff records resolve name ids, the unaligned club id, the birth
    /// date (day-of-year + year) and the player job code. The record id
    /// must equal its index, as in the real staff section.
    #[test]
    fn parses_staff_records() {
        let first = vec!["".to_string(), "Thierry".to_string()];
        let second = vec!["".to_string(), "Henry".to_string()];
        let common = vec!["".to_string()];
        let mut record = vec![0u8; STAFF_RECORD_SIZE];
        record[0..4].copy_from_slice(&0i32.to_le_bytes());
        record[4..8].copy_from_slice(&1i32.to_le_bytes());
        record[8..12].copy_from_slice(&1i32.to_le_bytes());
        record[12..16].copy_from_slice(&0i32.to_le_bytes());
        record[16..18].copy_from_slice(&228u16.to_le_bytes());
        record[18..20].copy_from_slice(&1977i16.to_le_bytes());
        record[57..61].copy_from_slice(&676i32.to_le_bytes());
        record[61] = 11;
        let players = parse_staff(&record, &first, &second, &common);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].full_name(), "Thierry Henry");
        assert_eq!(players[0].club_id, Some(676));
        assert_eq!(
            players[0].birth_date,
            Some(cm0102_core::GameDate::new(1977, 8, 17))
        );
        assert!(players[0].is_player);
        assert_eq!(players[0].job, 11);
    }

    /// A player-classified person merges CA/PA, reputations, squad
    /// number and attributes from the 70-byte player section that
    /// follows the (here empty) non-player section.
    #[test]
    fn merges_player_section_by_rank() {
        let first = vec!["".to_string(), "Thierry".to_string()];
        let second = vec!["".to_string(), "Henry".to_string()];
        let common = vec!["".to_string()];
        let mut data = vec![0u8; STAFF_RECORD_SIZE + PLAYER_RECORD_SIZE];
        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        data[8..12].copy_from_slice(&1i32.to_le_bytes());
        data[57..61].copy_from_slice(&676i32.to_le_bytes());
        data[61] = 11;
        data[95] = 2; // classification: player
        let player = STAFF_RECORD_SIZE;
        data[player + 4] = 14; // squad number
        data[player + 5..player + 7].copy_from_slice(&175i16.to_le_bytes());
        data[player + 7..player + 9].copy_from_slice(&183i16.to_le_bytes());
        data[player + 9..player + 11].copy_from_slice(&170i16.to_le_bytes());
        data[player + 15] = 1; // Goalkeeper
        data[player + 15 + 6] = 20; // Attacker
        let players = parse_staff(&data, &first, &second, &common);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].current_ability, Some(175));
        assert_eq!(players[0].potential_ability, Some(183));
        assert_eq!(players[0].home_reputation, Some(170));
        assert_eq!(players[0].squad_number, Some(14));
        assert_eq!(players[0].attributes.len(), ATTRIBUTE_COUNT);
        assert_eq!(players[0].attributes[6], 20);
    }

    /// The staff section stops when the sequential id run breaks, so
    /// the following player section is never misread as staff.
    #[test]
    fn staff_section_stops_at_id_break() {
        let first = vec!["".to_string(), "A".to_string()];
        let second = vec!["".to_string(), "B".to_string()];
        let common = vec!["".to_string()];
        let mut data = vec![0u8; STAFF_RECORD_SIZE * 2];
        data[4..8].copy_from_slice(&1i32.to_le_bytes());
        data[8..12].copy_from_slice(&1i32.to_le_bytes());
        data[57..61].copy_from_slice(&(-1i32).to_le_bytes());
        // Second record has a non-sequential id: section must stop.
        let second_record = STAFF_RECORD_SIZE;
        data[second_record..second_record + 4].copy_from_slice(&999i32.to_le_bytes());
        data[second_record + 4..second_record + 8].copy_from_slice(&1i32.to_le_bytes());
        data[second_record + 8..second_record + 12].copy_from_slice(&1i32.to_le_bytes());
        let players = parse_staff(&data, &first, &second, &common);
        assert_eq!(players.len(), 1);
    }

    /// A common name replaces the first name for display.
    #[test]
    fn common_name_wins_over_first_name() {
        let first = vec!["".to_string(), " Ronaldo".trim().to_string()];
        let second = vec!["".to_string(), "Nazario".to_string()];
        let common = vec!["".to_string(), "Ronaldo".to_string()];
        let mut record = vec![0u8; STAFF_RECORD_SIZE];
        record[0..4].copy_from_slice(&0i32.to_le_bytes());
        record[4..8].copy_from_slice(&1i32.to_le_bytes());
        record[8..12].copy_from_slice(&1i32.to_le_bytes());
        record[12..16].copy_from_slice(&1i32.to_le_bytes());
        record[57..61].copy_from_slice(&(-1i32).to_le_bytes());
        let players = parse_staff(&record, &first, &second, &common);
        assert_eq!(players[0].full_name(), "Ronaldo Nazario");
        assert_eq!(players[0].club_id, None);
    }
}
