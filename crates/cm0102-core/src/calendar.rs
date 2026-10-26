// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! In-game calendar. A CM 01/02 season starts on 1 July 2001.

/// A date in the simulated world.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct GameDate {
    /// Four-digit year, e.g. `2001`.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Day of month, 1–31.
    pub day: u32,
}

impl GameDate {
    /// The first day of the 01/02 season: 1 July 2001.
    pub const SEASON_START: GameDate = GameDate {
        year: 2001,
        month: 7,
        day: 1,
    };

    /// Create a date. No validation is performed beyond the type ranges.
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// Number of days in `month` of `year` (proleptic Gregorian calendar).
    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    /// Build a date from a year and a zero-based day of year
    /// (0 = 1 January), as stored in CM 01/02 staff records.
    /// Returns `None` for out-of-range years or days.
    pub fn from_day_of_year(year: i32, zero_based_day: u32) -> Option<Self> {
        if !(1901..=2010).contains(&year) || zero_based_day >= 366 {
            return None;
        }
        let mut month = 1u32;
        let mut day = zero_based_day + 1;
        loop {
            let days = Self::days_in_month(year, month);
            if day <= days {
                return Some(Self { year, month, day });
            }
            day -= days;
            month += 1;
            if month > 12 {
                return None;
            }
        }
    }

    /// The date `days` later, rolling over month and year boundaries.
    /// Round spacing is small (weekly), so a plain loop is enough.
    pub fn add_days(self, days: u32) -> Self {
        let mut date = self;
        for _ in 0..days {
            date = date.next_day();
        }
        date
    }

    /// The following day, rolling over month and year boundaries.
    pub fn next_day(self) -> Self {
        if self.day < Self::days_in_month(self.year, self.month) {
            Self {
                day: self.day + 1,
                ..self
            }
        } else if self.month < 12 {
            Self {
                month: self.month + 1,
                day: 1,
                ..self
            }
        } else {
            Self {
                year: self.year + 1,
                month: 1,
                day: 1,
            }
        }
    }
}

impl std::fmt::Display for GameDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}/{:02}/{}", self.day, self.month, self.year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Season start is 1 July 2001.
    #[test]
    fn season_start_is_first_of_july_2001() {
        assert_eq!(GameDate::SEASON_START, GameDate::new(2001, 7, 1));
    }

    /// `next_day` rolls over the year boundary.
    #[test]
    fn next_day_rolls_over_year() {
        assert_eq!(
            GameDate::new(2001, 12, 31).next_day(),
            GameDate::new(2002, 1, 1)
        );
    }

    /// `add_days` crosses month and year boundaries.
    #[test]
    fn add_days_crosses_boundaries() {
        assert_eq!(
            GameDate::new(2001, 7, 28).add_days(7),
            GameDate::new(2001, 8, 4)
        );
        assert_eq!(
            GameDate::new(2001, 12, 30).add_days(3),
            GameDate::new(2002, 1, 2)
        );
        assert_eq!(
            GameDate::new(2001, 7, 1).add_days(0),
            GameDate::new(2001, 7, 1)
        );
    }

    /// Day-of-year conversion lands on the right calendar date
    /// (day 228 of 1977 is 17 August, Thierry Henry's birthday).
    #[test]
    fn from_day_of_year_converts() {
        assert_eq!(
            GameDate::from_day_of_year(1977, 228),
            Some(GameDate::new(1977, 8, 17))
        );
        assert_eq!(GameDate::from_day_of_year(1900, 31), None);
        assert_eq!(GameDate::from_day_of_year(2001, 366), None);
    }

    /// `next_day` respects February length in a non-leap year.
    #[test]
    fn next_day_handles_february() {
        assert_eq!(
            GameDate::new(2002, 2, 28).next_day(),
            GameDate::new(2002, 3, 1)
        );
    }
}
