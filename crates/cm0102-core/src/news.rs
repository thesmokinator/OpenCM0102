// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Manager inbox: news items produced by the daily pipeline.
//!
//! Every engine pushes dated items as its stage runs (match reports,
//! injuries, bans, board messages, results roundups); the UI presents
//! fresh items after each Continue press and keeps the rest in the
//! News section until read.

use serde::{Deserialize, Serialize};

use crate::calendar::GameDate;

/// What kind of news an item carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NewsCategory {
    /// Full-time report of the managed club's match.
    MatchReport,
    /// Other results played the same day.
    ResultsRoundup,
    /// A managed player was injured.
    Injury,
    /// A managed player was banned.
    Suspension,
    /// Board confidence and expectations.
    Board,
    /// A fixture was postponed (unplayable pitch).
    Postponed,
}

/// One inbox item addressed to the manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewsItem {
    /// Stable id within the career, ascending.
    pub id: u64,
    /// In-game date the item was produced.
    pub date: GameDate,
    /// What kind of news this is.
    pub category: NewsCategory,
    /// One-line headline.
    pub headline: String,
    /// Longer body, possibly multi-line.
    pub body: String,
    /// True once the manager has seen it.
    pub read: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn news_item_round_trips_as_data() {
        let item = NewsItem {
            id: 7,
            date: GameDate::SEASON_START,
            category: NewsCategory::Board,
            headline: "Board pleased".to_string(),
            body: "Keep it up.".to_string(),
            read: false,
        };
        let json = serde_json::to_string(&item).expect("serializable");
        let restored: NewsItem = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(restored, item);
    }
}
