// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! Commentary packs: the `events_<lang>.cfg` format the original
//! game reads its match commentary from, plus a renderer.
//!
//! Format (one block per event, `#` comments, CRLF tolerated):
//! ```text
//! # EVENT_NAME
//! = id, priority, silent, follow_on, flashing, report, snd_centre, wait_snd, no_rollback
//! > probability, delay_ms, sound.wav
//! G in-game text with <tokens> and {s} plurals, `|` splits lines
//! R past-tense match-report text
//! ```
//! The `>`/`G`/`R` group repeats per text variant. Only the game
//! text, report text, probability and delay drive the renderer;
//! sounds are parsed and kept for completeness.
//!
//! The engine ships a built-in pack written in our own words and
//! loads the player's own file when one is configured: original
//! texts are never versioned, only this format reader.

use std::collections::HashMap;

use cm0102_core::{EventKind, MatchEvent};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Built-in English pack, our own words, same format and numbering.
pub const BUILTIN_PACK: &str = include_str!("../data/commentary_en.cfg");

/// One text variant of a commentary event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// Relative selection weight.
    pub probability: u32,
    /// Display hold in milliseconds.
    pub delay_ms: u32,
    /// Sound filename (parsed, not played).
    pub sound: String,
    /// In-game text, present tense.
    pub game: String,
    /// Match-report text, past tense.
    pub report: String,
}

/// One commentary event: id plus its text variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentaryEvent {
    /// Stable numeric id shared across packs.
    pub id: u16,
    /// Higher priority wins when several fire at once.
    pub priority: u8,
    /// Silent events never render a line.
    pub silent: bool,
    /// Part of a follow-on chain (parsed, chains resolved by caller).
    pub follow_on: bool,
    /// Text variants.
    pub variants: Vec<Variant>,
}

/// A parsed commentary pack: events by id.
#[derive(Debug, Clone, Default)]
pub struct CommentaryPack {
    events: HashMap<u16, CommentaryEvent>,
}

/// What went wrong parsing a pack, with a 1-based line number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackError {
    /// Offending line, 1-based.
    pub line: usize,
    /// What was expected.
    pub message: String,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for PackError {}

impl CommentaryPack {
    /// Parse a pack from cfg text.
    pub fn parse(text: &str) -> Result<Self, PackError> {
        let mut events = HashMap::new();
        let mut current: Option<CommentaryEvent> = None;
        let mut pending: Option<Variant> = None;
        let mut game_seen = false;
        for (index, raw) in text.lines().enumerate() {
            let line_no = index + 1;
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(body) = trimmed.strip_prefix('=') {
                flush_variant(&mut current, &mut pending, game_seen)?;
                if let Some(event) = current.take() {
                    events.insert(event.id, event);
                }
                game_seen = false;
                current = Some(parse_header(body, line_no)?);
            } else if let Some(body) = trimmed.strip_prefix('>') {
                if current.is_none() {
                    return Err(PackError {
                        line: line_no,
                        message: "variant outside an event block".to_string(),
                    });
                }
                // The report text is optional in the wild: a variant
                // carrying only its game text still closes here.
                flush_variant(&mut current, &mut pending, game_seen)?;
                pending = Some(parse_variant(body, line_no)?);
                game_seen = false;
            } else if let Some(body) = trimmed.strip_prefix('G') {
                let variant = pending.as_mut().ok_or(PackError {
                    line: line_no,
                    message: "G text without a variant line".to_string(),
                })?;
                if game_seen {
                    return Err(PackError {
                        line: line_no,
                        message: "duplicate G text for one variant".to_string(),
                    });
                }
                variant.game = body.trim().to_string();
                game_seen = true;
            } else if let Some(body) = trimmed.strip_prefix('R') {
                let event = current.as_mut().ok_or(PackError {
                    line: line_no,
                    message: "R text outside an event block".to_string(),
                })?;
                let mut variant = pending.take().ok_or(PackError {
                    line: line_no,
                    message: "R text without a variant line".to_string(),
                })?;
                if !game_seen {
                    return Err(PackError {
                        line: line_no,
                        message: "R text without its G text".to_string(),
                    });
                }
                variant.report = body.trim().to_string();
                event.variants.push(variant);
                game_seen = false;
            } else {
                return Err(PackError {
                    line: line_no,
                    message: format!("unexpected line: {trimmed}"),
                });
            }
        }
        flush_variant(&mut current, &mut pending, game_seen)?;
        if let Some(event) = current {
            events.insert(event.id, event);
        }
        Ok(Self { events })
    }

    /// Built-in pack.
    pub fn builtin() -> Result<Self, PackError> {
        Self::parse(BUILTIN_PACK)
    }

    /// Event by id.
    pub fn event(&self, id: u16) -> Option<&CommentaryEvent> {
        self.events.get(&id)
    }

    /// Number of parsed events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// True when no events parsed.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Close a pending variant into its event. A variant with its
/// game text but no report text still counts; one with neither is
/// malformed.
fn flush_variant(
    current: &mut Option<CommentaryEvent>,
    pending: &mut Option<Variant>,
    game_seen: bool,
) -> Result<(), PackError> {
    let Some(variant) = pending.take() else {
        return Ok(());
    };
    if !game_seen {
        return Err(PackError {
            line: 0,
            message: "variant without any text".to_string(),
        });
    }
    if let Some(event) = current {
        event.variants.push(variant);
    }
    Ok(())
}

/// Parse the `=` header line into an event shell.
fn parse_header(body: &str, line: usize) -> Result<CommentaryEvent, PackError> {
    let fields: Vec<&str> = body.split(',').map(str::trim).collect();
    if fields.len() != 9 {
        return Err(PackError {
            line,
            message: format!("header needs 9 fields, has {}", fields.len()),
        });
    }
    let number = |index: usize, what: &str| -> Result<u32, PackError> {
        fields[index].parse::<u32>().map_err(|_| PackError {
            line,
            message: format!("{what} is not a number: {}", fields[index]),
        })
    };
    Ok(CommentaryEvent {
        id: number(0, "id").and_then(|id| {
            u16::try_from(id).map_err(|_| PackError {
                line,
                message: format!("id out of range: {id}"),
            })
        })?,
        priority: number(1, "priority")? as u8,
        silent: number(2, "silent")? != 0,
        follow_on: number(3, "follow_on")? != 0,
        variants: Vec::new(),
    })
}

/// Parse the `>` variant line. The sound field may itself contain
/// commas, so only the first two fields split.
fn parse_variant(body: &str, line: usize) -> Result<Variant, PackError> {
    let mut parts = body.splitn(3, ',');
    let probability = parts
        .next()
        .unwrap_or("")
        .trim()
        .parse::<u32>()
        .map_err(|_| PackError {
            line,
            message: "variant probability is not a number".to_string(),
        })?;
    let delay_ms = parts
        .next()
        .unwrap_or("")
        .trim()
        .parse::<u32>()
        .map_err(|_| PackError {
            line,
            message: "variant delay is not a number".to_string(),
        })?;
    let sound = parts.next().unwrap_or("").trim().to_string();
    Ok(Variant {
        probability,
        delay_ms,
        sound,
        game: String::new(),
        report: String::new(),
    })
}

/// Tokens filling a rendered line.
#[derive(Debug, Clone, Default)]
pub struct CfgTokens<'a> {
    /// Primary protagonist.
    pub p1: &'a str,
    /// Secondary protagonist, when the incident has one.
    pub p2: Option<&'a str>,
    /// Protagonist's team.
    pub team: &'a str,
    /// Distance in yards, for shot lines.
    pub distance: Option<u16>,
    /// Referee name; falls back to "the referee".
    pub referee: Option<&'a str>,
    /// Running tally (e.g. scorer's goals including this one).
    pub tally: Option<u32>,
    /// Competition name, when known.
    pub competition: Option<&'a str>,
}

/// Which texts may render for incidents sharing one event id (the
/// original goal pool mixes open-play and own-goal lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextHint {
    /// Any variant.
    #[default]
    Any,
    /// Own-goal lines only.
    OwnGoal,
}

/// One rendered commentary line plus its display hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedLine {
    /// Finished text, tokens filled.
    pub text: String,
    /// Display hold in milliseconds.
    pub delay_ms: u32,
}

/// Render event `id` to finished lines (past tense for the report
/// when `report` is set). Returns `None` for unknown or silent
/// events, or when no variant fits the known tokens.
pub fn render(
    pack: &CommentaryPack,
    id: u16,
    tokens: &CfgTokens<'_>,
    hint: TextHint,
    report: bool,
    rng: &mut ChaCha8Rng,
) -> Option<Vec<RenderedLine>> {
    let event = pack.event(id)?;
    if event.silent || event.variants.is_empty() {
        return None;
    }
    let fitting: Vec<&Variant> = event
        .variants
        .iter()
        .filter(|variant| {
            let text = if report { &variant.report } else { &variant.game };
            variant_fits(text, tokens, hint)
        })
        .collect();
    if fitting.is_empty() {
        return None;
    }
    let total: u32 = fitting.iter().map(|variant| variant.probability.max(1)).sum();
    let mut draw = rng.random_range(0..total);
    let mut picked = fitting[0];
    for variant in fitting {
        let weight = variant.probability.max(1);
        if draw < weight {
            picked = variant;
            break;
        }
        draw -= weight;
    }
    let raw = if report { &picked.report } else { &picked.game };
    Some(
        raw.split('|')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| RenderedLine {
                text: fill_tokens(part, tokens),
                delay_ms: picked.delay_ms,
            })
            .collect(),
    )
}

/// Whole-report rendering: game lines per incident (falls back
/// to the one-line display when an event has no usable text),
/// per-incident display holds, and past-tense report prose.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchCommentary {
    /// Finished game lines per incident, aligned with the events.
    pub lines: Vec<Vec<String>>,
    /// Display hold per incident in milliseconds.
    pub delays: Vec<u32>,
    /// Past-tense prose for report-worthy incidents.
    pub prose: Vec<String>,
}

/// Fallback hold when an incident renders no cfg text.
const FALLBACK_DELAY_MS: u32 = 900;

/// Render every incident of a report deterministically from `seed`.
/// `teams` is (home, away); the referee falls back to "the referee".
pub fn render_match(
    pack: &CommentaryPack,
    events: &[MatchEvent],
    teams: (&str, &str),
    referee: Option<&str>,
    competition: Option<&str>,
    seed: u64,
) -> MatchCommentary {
    use rand::SeedableRng;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut out = MatchCommentary::default();
    let mut tallies: HashMap<(String, bool), u32> = HashMap::new();
    for event in events {
        if matches!(event.kind, EventKind::Goal | EventKind::Penalty) {
            *tallies
                .entry((event.player.clone(), event.home))
                .or_insert(0) += 1;
        }
        let team = if event.home { teams.0 } else { teams.1 };
        // Only substitutions carry a second protagonist in `other`
        // (the player off); injuries store the layoff there instead.
        // Saved shots store the keeper there and swap roles: keeper
        // lines address the keeper as `<p1>`, the shooter as `<p2>`.
        let (p1, p2) = match event.kind {
            EventKind::Sub => (event.player.as_str(), event.other.as_deref()),
            EventKind::ShotSaved => match event.other.as_deref() {
                Some(keeper) => (keeper, Some(event.player.as_str())),
                None => (event.player.as_str(), None),
            },
            _ => (event.player.as_str(), None),
        };
        let tokens = CfgTokens {
            p1,
            p2,
            team,
            distance: event.distance_yards,
            referee,
            tally: tallies.get(&(event.player.clone(), event.home)).copied(),
            competition,
        };
        let hint = if event.kind == EventKind::OwnGoal {
            TextHint::OwnGoal
        } else {
            TextHint::Any
        };
        let id = event.commentary_id;
        let game = id.and_then(|id| render(pack, id, &tokens, hint, false, &mut rng));
        match game {
            Some(rendered) if !rendered.is_empty() => {
                out.delays.push(rendered[0].delay_ms);
                out.lines.push(
                    rendered
                        .into_iter()
                        .map(|line| line.text)
                        .collect(),
                );
            }
            _ => {
                out.delays.push(FALLBACK_DELAY_MS);
                out.lines.push(vec![event.display()]);
            }
        }
        if is_report_worthy(event.kind) {
            // Open-play goals render game text from the one-line
            // pool but past-tense prose from the goal pool, which
            // carries the report lines.
            let prose_id = if event.kind == EventKind::Goal {
                Some(140)
            } else {
                id
            };
            if let Some(rendered) =
                prose_id.and_then(|id| render(pack, id, &tokens, hint, true, &mut rng))
            {
                out.prose.extend(rendered.into_iter().map(|line| line.text));
            }
        }
    }
    out
}

/// Incidents earning past-tense match-report prose.
fn is_report_worthy(kind: EventKind) -> bool {
    matches!(
        kind,
        EventKind::Goal
            | EventKind::Penalty
            | EventKind::MissedPenalty
            | EventKind::OwnGoal
            | EventKind::ShotSaved
            | EventKind::Yellow
            | EventKind::Red
            | EventKind::Injury
            | EventKind::Sub
            | EventKind::Shootout
    )
}

/// True when a variant's text only needs tokens we know.
fn variant_fits(text: &str, tokens: &CfgTokens<'_>, hint: TextHint) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    if tokens.p2.is_none() && text.contains("<p2>") {
        return false;
    }
    if tokens.distance.is_none() && text.contains("<d>") {
        return false;
    }
    if tokens.tally.is_none() && text.contains("<n>") {
        return false;
    }
    if tokens.competition.is_none() && (text.contains("<c>") || text.contains("<f>")) {
        return false;
    }
    if hint == TextHint::OwnGoal {
        return is_own_goal_line(text);
    }
    !is_own_goal_line(text)
}

/// True for lines describing an own-goal incident (as opposed to
/// passing mentions of one's own goal).
fn is_own_goal_line(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("scores an own goal")
        || lower.contains("scored an own goal")
        || lower.contains("into his own goal")
        || lower.contains("into her own goal")
        || lower.contains("disaster for")
}

/// Fill `<tokens>`, strip colour codes and unknown tags, expand
/// `{s}` plurals.
fn fill_tokens(text: &str, tokens: &CfgTokens<'_>) -> String {
    let referee_cap = tokens.referee.unwrap_or("The referee");
    let referee = tokens.referee.unwrap_or("the referee");
    let mut out = text.to_string();
    for (tag, value) in [
        ("<Rf>", referee_cap.to_string()),
        ("<rf>", referee.to_string()),
        ("<p1c>", tokens.p1.to_string()),
        ("<p1>", tokens.p1.to_string()),
        ("<p2>", tokens.p2.unwrap_or("").to_string()),
        ("<t1>", tokens.team.to_string()),
        ("<t2>", tokens.team.to_string()),
        ("<t>", tokens.team.to_string()),
        ("<tc>", tokens.team.to_string()),
        (
            "<d>",
            tokens.distance.map(|d| d.to_string()).unwrap_or_default(),
        ),
        (
            "<d1>",
            tokens.distance.map(|d| d.to_string()).unwrap_or_default(),
        ),
        ("<n>", tokens.tally.map(|n| n.to_string()).unwrap_or_default()),
        ("<N>", tokens.tally.map(|n| n.to_string()).unwrap_or_default()),
        (
            "<c>",
            tokens.competition.unwrap_or("").to_string(),
        ),
        (
            "<f>",
            tokens.competition.unwrap_or("").to_string(),
        ),
        ("<O>", String::new()),
        ("<R>", String::new()),
        ("<Y>", String::new()),
    ] {
        out = out.replace(tag, &value);
    }
    // Strip any leftover <tag>.
    while let Some(start) = out.find('<') {
        let Some(end) = out[start..].find('>') else {
            break;
        };
        let end = start + end;
        let tag = &out[start..=end];
        if tag.len() <= 5 && tag[1..tag.len() - 1].chars().all(|c| c.is_alphanumeric()) {
            out.replace_range(start..=end, "");
        } else {
            break;
        }
    }
    // `{s}` always trails a token as a possessive in the wild
    // ("<p1>{s} effort" -> "Henry's effort").
    out = out.replace("{s}", "'s").replace("{}", "");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    const SAMPLE: &str = "# EVENT_GOAL_SCORED\r\n= 191, 1, 0, 0, 1, 2, 0, 0, 0\r\n> 70, 900, cheerg1.wav\r\nG <p1> scores for <t>!|What a strike from <d> yards!\r\nR <p1> scored for <t>.\r\n> 30, 1500, cheerg2.wav\r\nG <O><p1> buries it!\r\nR <p1> buried it.\r\n";

    fn rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(7)
    }

    #[test]
    fn parses_headers_variants_and_texts() {
        let pack = CommentaryPack::parse(SAMPLE).expect("parses");
        assert_eq!(pack.len(), 1);
        let event = pack.event(191).expect("id 191");
        assert_eq!(event.priority, 1);
        assert!(!event.silent);
        assert_eq!(event.variants.len(), 2);
        assert_eq!(event.variants[0].delay_ms, 900);
        assert_eq!(event.variants[0].sound, "cheerg1.wav");
    }

    #[test]
    fn render_fills_tokens_and_splits_lines() {
        let pack = CommentaryPack::parse(SAMPLE).expect("parses");
        let tokens = CfgTokens {
            p1: "Henry",
            team: "Arsenal",
            distance: Some(18),
            ..CfgTokens::default()
        };
        // Both variants render cleanly across seeds: no leftover
        // markup, and the protagonist appears in one line.
        for seed in 0..10 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let lines = render(&pack, 191, &tokens, TextHint::Any, false, &mut rng)
                .expect("renders");
            assert!(!lines.is_empty());
            for line in &lines {
                assert!(!line.text.contains('<'), "{}", line.text);
                assert!(!line.text.contains("{s}"), "{}", line.text);
            }
            assert!(
                lines.iter().any(|line| line.text.contains("Henry")),
                "{lines:?}"
            );
        }
    }

    #[test]
    fn render_skips_variants_needing_unknown_tokens() {
        let pack = CommentaryPack::parse(SAMPLE).expect("parses");
        let tokens = CfgTokens {
            p1: "Henry",
            team: "Arsenal",
            distance: None,
            ..CfgTokens::default()
        };
        let mut rng = rng();
        let lines = render(&pack, 191, &tokens, TextHint::Any, false, &mut rng).expect("renders");
        // First variant needs <d>; only the second fits.
        assert_eq!(lines.len(), 1);
        assert!(lines[0].text.contains("buries"), "{}", lines[0].text);
    }

    #[test]
    fn render_reports_past_tense() {
        let pack = CommentaryPack::parse(SAMPLE).expect("parses");
        let tokens = CfgTokens {
            p1: "Henry",
            team: "Arsenal",
            distance: Some(18),
            ..CfgTokens::default()
        };
        let mut rng = rng();
        let lines = render(&pack, 191, &tokens, TextHint::Any, true, &mut rng).expect("renders");
        assert!(lines.iter().all(|line| !line.text.contains('<')));
    }

    #[test]
    fn malformed_packs_fail_with_lines() {
        assert!(CommentaryPack::parse("= 1, 2\n").is_err());
        assert!(CommentaryPack::parse("G orphan\n").is_err());
        assert!(CommentaryPack::parse("= 1, 0, 0, 0, 0, 0, 0, 0, 0\n> x, 900\n").is_err());
    }

    #[test]
    fn builtin_pack_covers_mapped_ids() {
        let pack = CommentaryPack::builtin().expect("builtin parses");
        for id in [
            22u16, 27, 37, 38, 52, 127, 131, 132, 134, 135, 138, 139, 140, 141, 142, 143, 164, 166,
            167, 183, 184, 185, 191, 193, 196, 407, 433, 565, 566,
        ] {
            assert!(pack.event(id).is_some(), "missing {id}");
        }
    }

    /// Whole-report rendering aligns lines and delays with the
    /// incidents, deterministically per seed.
    #[test]
    fn render_match_aligns_and_repeats() {
        use cm0102_core::{EventKind, MatchEvent};
        let pack = CommentaryPack::builtin().expect("builtin parses");
        let events = vec![
            MatchEvent {
                minute: 10,
                home: true,
                kind: EventKind::Goal,
                player: "Henry".to_string(),
                other: None,
                commentary_id: Some(191),
                distance_yards: Some(18),
            },
            MatchEvent {
                minute: 20,
                home: false,
                kind: EventKind::Yellow,
                player: "Keane".to_string(),
                other: None,
                commentary_id: Some(185),
                distance_yards: None,
            },
        ];
        let first = render_match(&pack, &events, ("Arsenal", "United"), None, None, 42);
        let second = render_match(&pack, &events, ("Arsenal", "United"), None, None, 42);
        assert_eq!(first, second);
        assert_eq!(first.lines.len(), 2);
        assert_eq!(first.delays.len(), 2);
        assert!(first.lines[0].iter().any(|line| line.contains("Henry")));
        assert!(first.lines[1].iter().any(|line| line.contains("Keane")));
        assert!(!first.prose.is_empty());
    }

    /// The player's own pack parses when `OPENCM0102_DATA_DIR`
    /// points at it; skipped otherwise. Asserts the mapped ids all
    /// resolve, so engine output never misses original text.
    #[test]
    fn original_pack_parses_when_present() {
        let Some(dir) = std::env::var_os("OPENCM0102_DATA_DIR") else {
            eprintln!("skipping: OPENCM0102_DATA_DIR unset");
            return;
        };
        let path = std::path::Path::new(&dir).join("events_eng.cfg");
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("skipping: {} unreadable", path.display());
            return;
        };
        let text = String::from_utf8_lossy(&bytes);
        eprintln!("parsing {} ({} bytes)", path.display(), bytes.len());
        let pack = CommentaryPack::parse(&text).expect("original parses");
        assert!(pack.len() > 600, "only {} events", pack.len());
        for id in [
            22u16, 27, 37, 38, 52, 127, 131, 132, 134, 135, 138, 139, 140, 141, 142, 143, 164, 166,
            167, 183, 184, 185, 191, 193, 196, 407, 433, 565, 566,
        ] {
            assert!(pack.event(id).is_some(), "original lacks {id}");
        }
    }

    #[test]
    fn own_goal_hint_filters_pool_texts() {
        let pack = CommentaryPack::parse(SAMPLE).expect("parses");
        let tokens = CfgTokens {
            p1: "Keown",
            team: "Arsenal",
            distance: Some(6),
            ..CfgTokens::default()
        };
        let mut rng = rng();
        // No own-goal line in the sample: filtered out entirely.
        assert!(render(&pack, 191, &tokens, TextHint::OwnGoal, false, &mut rng).is_none());
    }
}
