// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Transfers: work permits and scripted `player_setup.cfg` events.
//!
//! Behaviour from `docs/transfers.md`: a permit is required exactly
//! when the destination club's nation is flagged as permit-requiring
//! and the player holds no exempt (EU/second) nationality. Permits
//! are 14-byte records (player, two counters, expiry season, nation)
//! in a sorted array; a periodic processor grants when the player's
//! qualifying date is within five days of today and otherwise marks
//! a refusal, renewals refresh the expiry and nation and clear the
//! counters, and removal swaps in the last record. Scripted setup
//! events (`FUTURE_TRANSFER`, `LOAN`) apply on their dates.

use std::collections::HashMap;

use crate::calendar::GameDate;

/// One work-permit record: the persisted 14-byte shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermitRecord {
    /// Player id.
    pub player_id: u32,
    /// First counter (cleared on renewal).
    pub counter_a: u16,
    /// Second counter (cleared on renewal).
    pub counter_b: u16,
    /// Season the permit expires in.
    pub expiry_season: u16,
    /// Nation that issued the permit.
    pub nation_id: u32,
}

/// True when `player_id` currently holds a permit.
pub fn holds_permit(store: &[PermitRecord], player_id: u32) -> bool {
    store.iter().any(|record| record.player_id == player_id)
}

/// A permit is required exactly when the destination nation requires
/// one and none of the player's nationalities is exempt.
pub fn permit_required(
    destination_requires_permit: bool,
    player_nationalities: &[u32],
    exempt_nationalities: &[u32],
) -> bool {
    destination_requires_permit
        && !player_nationalities
            .iter()
            .any(|nation| exempt_nationalities.contains(nation))
}

/// Outcome of the periodic permit processor for one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitOutcome {
    /// Qualifying date within five days: granted.
    Granted,
    /// Otherwise: refused.
    Refused,
}

/// Periodic processor: for each `(player, qualifying date)` entry,
/// grant when the qualifying date is within five days of `today`
/// (inclusive), else refuse. Grants insert (or renew: refresh expiry
/// and nation, clear counters) the sorted record; refusals leave the
/// store untouched. Returns the outcome per player.
pub fn process_permits(
    store: &mut Vec<PermitRecord>,
    today: GameDate,
    qualifying: &HashMap<u32, GameDate>,
    expiry_season: u16,
    nation_id: u32,
) -> HashMap<u32, PermitOutcome> {
    let mut outcomes = HashMap::new();
    let mut players: Vec<u32> = qualifying.keys().copied().collect();
    players.sort_unstable();
    for player_id in players {
        let date = qualifying[&player_id];
        let outcome = if within_days(today, date, 5) {
            upsert_permit(store, player_id, expiry_season, nation_id);
            PermitOutcome::Granted
        } else {
            PermitOutcome::Refused
        };
        outcomes.insert(player_id, outcome);
    }
    outcomes
}

/// Insert a fresh record or renew the existing one (refresh expiry
/// and nation, clear both counters), keeping the array sorted by
/// player id.
fn upsert_permit(
    store: &mut Vec<PermitRecord>,
    player_id: u32,
    expiry_season: u16,
    nation_id: u32,
) {
    match store.binary_search_by_key(&player_id, |record| record.player_id) {
        Ok(index) => {
            let record = &mut store[index];
            record.counter_a = 0;
            record.counter_b = 0;
            record.expiry_season = expiry_season;
            record.nation_id = nation_id;
        }
        Err(index) => store.insert(
            index,
            PermitRecord {
                player_id,
                counter_a: 0,
                counter_b: 0,
                expiry_season,
                nation_id,
            },
        ),
    }
}

/// Remove a permit by swapping in the last record. Returns false when
/// the player held no permit.
pub fn remove_permit(store: &mut Vec<PermitRecord>, player_id: u32) -> bool {
    if let Ok(index) = store.binary_search_by_key(&player_id, |record| record.player_id) {
        store.swap_remove(index);
        if store.len() > 1 {
            store.sort_by_key(|record| record.player_id);
        }
        true
    } else {
        false
    }
}

/// True when `date` is `today` or at most `days` days after it.
fn within_days(today: GameDate, date: GameDate, days: u32) -> bool {
    (0..=days).any(|offset| today.add_days(offset) == date)
}

/// One scripted setup event from `player_setup.cfg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupEvent {
    /// A future transfer applying on `date`.
    FutureTransfer {
        /// Name parts identifying the player.
        player: [String; 3],
        /// Club the player moves from.
        from_club: String,
        /// Club the player moves to.
        to_club: String,
        /// Date the move applies on.
        date: GameDate,
    },
    /// A loan from `start` to `end`.
    Loan {
        /// Name parts identifying the player.
        player: [String; 3],
        /// Club loaning the player out.
        from_club: String,
        /// Club receiving the player.
        to_club: String,
        /// First day of the loan.
        start: GameDate,
        /// Last day of the loan.
        end: GameDate,
    },
}

impl SetupEvent {
    /// Date the event applies on (loans: their start).
    pub fn applies_on(&self) -> GameDate {
        match self {
            SetupEvent::FutureTransfer { date, .. } => *date,
            SetupEvent::Loan { start, .. } => *start,
        }
    }
}

/// Parse the quoted-command script lines of `player_setup.cfg`,
/// skipping blank lines and anything that is not a recognised
/// command. Trailing numeric arguments after the dates are accepted
/// and ignored.
pub fn parse_setup_events(text: &str) -> Vec<SetupEvent> {
    text.lines().filter_map(parse_setup_line).collect()
}

/// Events applying exactly on `today`, in file order.
pub fn due_events(events: &[SetupEvent], today: GameDate) -> Vec<&SetupEvent> {
    events
        .iter()
        .filter(|event| event.applies_on() == today)
        .collect()
}

/// Parse one script line, or return `None` for blanks and unknown
/// commands.
fn parse_setup_line(line: &str) -> Option<SetupEvent> {
    let tokens = tokenize(line);
    let (command, rest) = tokens.split_first()?;
    match command.as_str() {
        "FUTURE_TRANSFER" => {
            // player(3) from to day month year [trailing...]
            let [p0, p1, p2, from, to, day, month, year, ..] = rest else {
                return None;
            };
            Some(SetupEvent::FutureTransfer {
                player: [p0.clone(), p1.clone(), p2.clone()],
                from_club: from.clone(),
                to_club: to.clone(),
                date: GameDate::new(
                    year.parse().ok()?,
                    month.parse().ok()?,
                    day.parse().ok()?,
                ),
            })
        }
        "LOAN" => {
            // player(3) from to d1 m1 y1 d2 m2 y2
            let [p0, p1, p2, from, to, d1, m1, y1, d2, m2, y2, ..] = rest else {
                return None;
            };
            Some(SetupEvent::Loan {
                player: [p0.clone(), p1.clone(), p2.clone()],
                from_club: from.clone(),
                to_club: to.clone(),
                start: GameDate::new(y1.parse().ok()?, m1.parse().ok()?, d1.parse().ok()?),
                end: GameDate::new(y2.parse().ok()?, m2.parse().ok()?, d2.parse().ok()?),
            })
        }
        _ => None,
    }
}

/// Split a line into quoted strings and bare numbers.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&char) = chars.peek() {
        if char.is_whitespace() {
            chars.next();
        } else if char == '"' {
            chars.next();
            let mut token = String::new();
            for char in chars.by_ref() {
                if char == '"' {
                    break;
                }
                token.push(char);
            }
            tokens.push(token);
        } else {
            let mut token = String::new();
            while let Some(&char) = chars.peek() {
                if char.is_whitespace() {
                    break;
                }
                token.push(char);
                chars.next();
            }
            tokens.push(token);
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Permits are required only for flagged destinations without an
    /// exempt nationality.
    #[test]
    fn permit_predicate_matches_spec() {
        assert!(permit_required(true, &[7], &[1, 2]));
        assert!(!permit_required(true, &[1], &[1, 2]));
        assert!(!permit_required(false, &[7], &[1, 2]));
        assert!(!permit_required(true, &[7, 2], &[1, 2]));
    }

    /// Grants insert sorted records; renewals refresh and clear.
    #[test]
    fn grants_insert_and_renewals_refresh() {
        let mut store = Vec::new();
        let today = GameDate::new(2001, 7, 1);
        let qualifying = HashMap::from([
            (20u32, GameDate::new(2001, 7, 3)),
            (10u32, GameDate::new(2001, 7, 6)),
        ]);
        let outcomes = process_permits(&mut store, today, &qualifying, 2002, 50);
        assert_eq!(outcomes[&10], PermitOutcome::Granted);
        assert_eq!(outcomes[&20], PermitOutcome::Granted);
        assert_eq!(
            store.iter().map(|record| record.player_id).collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert!(holds_permit(&store, 10));

        store[0].counter_a = 4;
        store[0].counter_b = 9;
        let renewal = HashMap::from([(10u32, GameDate::new(2001, 7, 2))]);
        process_permits(&mut store, today, &renewal, 2003, 51);
        assert_eq!(store[0].counter_a, 0);
        assert_eq!(store[0].counter_b, 0);
        assert_eq!(store[0].expiry_season, 2003);
        assert_eq!(store[0].nation_id, 51);
    }

    /// Qualifying dates beyond five days refuse and store nothing.
    #[test]
    fn distant_qualifying_dates_refuse() {
        let mut store = Vec::new();
        let today = GameDate::new(2001, 7, 1);
        let qualifying = HashMap::from([(5u32, GameDate::new(2001, 7, 20))]);
        let outcomes = process_permits(&mut store, today, &qualifying, 2002, 50);
        assert_eq!(outcomes[&5], PermitOutcome::Refused);
        assert!(store.is_empty());
        assert!(!holds_permit(&store, 5));
    }

    /// Removal swaps in the last record; unknown players fail.
    #[test]
    fn removal_swaps_last_record() {
        let mut store = vec![
            PermitRecord {
                player_id: 1,
                counter_a: 0,
                counter_b: 0,
                expiry_season: 2002,
                nation_id: 50,
            },
            PermitRecord {
                player_id: 2,
                counter_a: 0,
                counter_b: 0,
                expiry_season: 2002,
                nation_id: 50,
            },
            PermitRecord {
                player_id: 3,
                counter_a: 0,
                counter_b: 0,
                expiry_season: 2002,
                nation_id: 50,
            },
        ];
        assert!(remove_permit(&mut store, 1));
        assert!(!holds_permit(&store, 1));
        assert_eq!(store.len(), 2);
        assert!(!remove_permit(&mut store, 99));
    }

    /// Both documented script shapes parse with trailing numbers
    /// ignored, and due filtering keeps file order.
    #[test]
    fn setup_scripts_parse_and_filter_by_date() {
        let text = concat!(
            "\"FUTURE_TRANSFER\" \"Steinar\" \"\" \"Pedersen\" \"IFK G\u{00f6}teborg\" \"IK Start\" 01 12 2001 0 -1 4\n",
            "\"LOAN\" \"Andrey\" \"Demkine\" \"Demkin\" \"Germinal Beerschot Antwerpen\" \"KSK Beveren\" 01 07 2001 30 06 2002\n",
            "\n",
            "\"RETIREMENT\" \"Someone\" \"X\" \"Y\" \"Club\" 23 7 2002\n",
        );
        let events = parse_setup_events(text);
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].applies_on(),
            GameDate::new(2001, 12, 1),
        );
        let july = due_events(&events, GameDate::new(2001, 7, 1));
        assert_eq!(july.len(), 1);
        assert!(matches!(july[0], SetupEvent::Loan { .. }));
        let december = due_events(&events, GameDate::new(2001, 12, 1));
        assert_eq!(december.len(), 1);
        assert!(matches!(december[0], SetupEvent::FutureTransfer { .. }));
    }
}
