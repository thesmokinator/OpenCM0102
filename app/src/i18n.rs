// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Michele Broggi
//
//! User-interface localization (Project Fluent via `i18n-embed`).
//!
//! Locale files live in `app/i18n/<lang>/opencm0102.ftl` and are
//! embedded in the binary. English (`en`) is the fallback: every
//! other locale falls back to it key by key. The initial language
//! is negotiated from the operating-system locale; egui re-renders
//! on every frame, so switching language applies immediately.

use std::collections::HashMap;
use std::sync::OnceLock;

use fluent::FluentValue;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::LanguageLoader;
use rust_embed::RustEmbed;
use unic_langid::LanguageIdentifier;

/// Embedded locale files from `app/i18n/`.
#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

/// Process-wide message loader, initialized once from the OS locale.
static LOADER: OnceLock<FluentLanguageLoader> = OnceLock::new();

/// Access the initialized loader, negotiating the initial language
/// from the operating-system locale with English as fallback.
fn loader() -> &'static FluentLanguageLoader {
    LOADER.get_or_init(|| {
        let loader = FluentLanguageLoader::new("opencm0102", "en".parse().expect("valid tag"));
        let requested: Vec<LanguageIdentifier> = sys_locale::get_locales()
            .filter_map(|tag| tag.parse().ok())
            .collect();
        if i18n_embed::select(&loader, &Localizations, &requested).is_err() {
            loader
                .load_fallback_language(&Localizations)
                .expect("embedded English locale");
        }
        // Plain interpolation (no Unicode isolating marks): must run
        // after every load, which recreates the bundles.
        loader.set_use_isolating(false);
        loader
    })
}

/// Look up a message by id in the current language (English fallback).
/// Missing keys resolve to the id itself, never to an empty string.
pub fn t(message_id: &str) -> String {
    if has(message_id) {
        loader().get(message_id)
    } else {
        message_id.to_string()
    }
}

/// Look up a message with `$name` arguments in the current language.
pub fn tr(message_id: &str, args: &[(&str, &str)]) -> String {
    let owned: HashMap<String, FluentValue> = args
        .iter()
        .map(|(name, value)| ((*name).to_string(), FluentValue::from(*value)))
        .collect();
    loader().get_args(message_id, owned)
}

/// True when the current language (or fallback) defines `message_id`.
pub fn has(message_id: &str) -> bool {
    loader().has(message_id)
}

/// Currently selected language.
pub fn current_language() -> LanguageIdentifier {
    loader().current_language()
}

/// Languages with an embedded locale directory.
pub fn available_languages() -> Vec<LanguageIdentifier> {
    loader().available_languages(&Localizations).unwrap_or_default()
}

/// Switch language by tag (e.g. `"en"`). False when the tag is
/// unknown or unavailable; the current language is kept then.
pub fn set_language(tag: &str) -> bool {
    let Ok(language) = tag.parse::<LanguageIdentifier>() else {
        return false;
    };
    if !available_languages().contains(&language) {
        return false;
    }
    let applied = loader().load_languages(&Localizations, &[language]).is_ok();
    loader().set_use_isolating(false);
    applied
}

/// Native display name for a language tag, for settings lists.
pub fn display_name(language: &LanguageIdentifier) -> String {
    match language.language.as_str() {
        "da" => "Dansk",
        "de" => "Deutsch",
        "en" => "English",
        "es" => "Español",
        "fr" => "Français",
        "it" => "Italiano",
        "nl" => "Nederlands",
        "no" => "Norsk",
        "pt" => "Português",
        "sv" => "Svenska",
        _ => return language.to_string(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_keys_resolve() {
        assert!(has("setup-title"));
        assert_eq!(t("setup-title"), "Setup Game");
        assert_eq!(t("section-squad"), "Squad");
    }

    #[test]
    fn missing_keys_fall_back_to_the_id() {
        assert!(!has("no-such-key"));
        assert_eq!(t("no-such-key"), "no-such-key");
    }

    #[test]
    fn message_arguments_interpolate() {
        assert_eq!(
            tr("sidebar-version", &[("version", "0.1.0")]),
            "Version 0.1.0"
        );
    }

    #[test]
    fn unknown_language_is_rejected() {
        assert!(!set_language("xx"));
        assert!(!set_language("not a tag!!!"));
    }

    #[test]
    fn english_is_available_and_selectable() {
        let available = available_languages();
        assert!(available.iter().any(|tag| tag.language.as_str() == "en"));
        assert!(set_language("en"));
        assert_eq!(current_language().language.as_str(), "en");
    }
}
