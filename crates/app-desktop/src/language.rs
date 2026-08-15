//! UI language persisted to `config.toml`.
//!
//! Mirror of [`crate::theme`]: the same pattern (enum + `as_str`/`from_config`),
//! but for language selection. The locale is stored in `AppState.language` and
//! applied globally via [`rust_i18n::set_locale`] in `use_effect` (see `main.rs`).
//!
//! Translations live in `locales/app.<lang>.yml` and are embedded into the binary
//! by the `rust_i18n::i18n!` macro (compile-time, with no runtime files — matching
//! the project convention of `include_str!`/`const`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// Available UI languages.
///
/// Variant order = order in the UI selector. English is the default (`#[default]`)
/// and the fallback for unfilled keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// English (default, fallback for unfilled keys).
    #[default]
    En,
    /// Deutsch.
    De,
    /// Français.
    Fr,
    /// Español.
    Es,
    /// Italiano.
    It,
    /// Português.
    Pt,
    /// Nederlands.
    Nl,
    /// Polski.
    Pl,
    /// Українська.
    Uk,
}

impl Language {
    /// BCP-47 code for `rust_i18n::set_locale` and the TOML config (`config.toml`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::It => "it",
            Self::Pt => "pt",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Uk => "uk",
        }
    }

    /// Parse from a config string. An unknown value falls back to English (the
    /// default and fallback language), so a typo in `config.toml` cannot break
    /// the UI.
    #[must_use]
    pub fn from_config(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "de" => Self::De,
            "fr" => Self::Fr,
            "es" => Self::Es,
            "it" => Self::It,
            "pt" => Self::Pt,
            "nl" => Self::Nl,
            "pl" => Self::Pl,
            "uk" => Self::Uk,
            _ => Self::En,
        }
    }

    /// All supported languages (for the UI selector). Order is fixed.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::En,
            Self::De,
            Self::Fr,
            Self::Es,
            Self::It,
            Self::Pt,
            Self::Nl,
            Self::Pl,
            Self::Uk,
        ]
    }

    /// Emoji flag of the language (regional-indicator symbols). Kept as a stable
    /// Unicode label; note that on Windows/WebView2 these emoji do not render as
    /// flag glyphs (Segoe UI Emoji lacks them) and fall back to letters. For the
    /// visual flag in the UI selector use [`country_code`](Self::country_code)
    /// with the `.flag` CSS class family, which renders an inline SVG flag that
    /// is identical on every platform.
    /// ISO-3166-1 alpha-2 country code used to render the language's flag in the
    /// UI selector. The code becomes the CSS modifier `flag-<country_code>`
    /// (e.g. `flag-gb`, `flag-ua`), which the stylesheet maps to an inline SVG
    /// flag. Unlike the emoji [`flag`](Self::flag), the SVG renders identically
    /// on Windows, Linux, and macOS (WebView2 has no flag-glyph font support).
    /// English → gb; português → pt (European, not BR).
    #[must_use]
    pub const fn country_code(self) -> &'static str {
        match self {
            Self::En => "gb",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::It => "it",
            Self::Pt => "pt",
            Self::Nl => "nl",
            Self::Pl => "pl",
            Self::Uk => "ua",
        }
    }

    /// Localized endonym of the language (for the UI selector options). Reads the
    /// `lang.<code>` key from the translation catalog — the endonym is always in
    /// its own language (e.g. `uk` -> "Українська"), regardless of the current
    /// locale.
    ///
    /// The key is built dynamically (`format!`) because the `tr!` macro with the
    /// literal `"lang.{code}"` would look up that literal key verbatim (with the
    /// braces) rather than substituting `code` into the name. The dynamic key is
    /// passed through a slice.
    #[must_use]
    pub fn label(self) -> String {
        // format! outside the macro builds "lang.en" etc.; tr! accepts a &str key.
        let key = format!("lang.{}", self.as_str());
        tr!(key.as_str())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    /// Language codes are non-empty and unique (guard against accidental duplicates).
    #[test]
    fn codes_nonempty_and_distinct() {
        let all = Language::all();
        let codes: Vec<&str> = all.iter().map(|l| l.as_str()).collect();
        for c in &codes {
            assert!(!c.is_empty(), "empty language code");
        }
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "duplicate language code");
    }

    /// Round-trip: from_config(as_str) recovers the language.
    #[test]
    fn from_config_roundtrip() {
        for lang in Language::all() {
            assert_eq!(Language::from_config(lang.as_str()), *lang);
        }
    }

    /// Unknown value falls back to English (default + fallback).
    #[test]
    fn unknown_falls_back_to_english() {
        assert_eq!(Language::from_config("klingon"), Language::En);
        assert_eq!(Language::from_config(""), Language::En);
        // Case-insensitivity.
        assert_eq!(Language::from_config("EN"), Language::En);
        assert_eq!(Language::from_config("  Uk "), Language::Uk);
    }

    /// Default is English.
    #[test]
    fn default_is_english() {
        assert_eq!(Language::default(), Language::En);
    }

    /// label() returns the endonym (not the literal key, not empty) for every
    /// language. Regression: previously `tr!("lang.{code}")` looked up the literal
    /// key with the braces and fell back to the key itself.
    #[test]
    fn label_returns_endonym_not_literal() {
        for lang in Language::all() {
            let label = lang.label();
            assert!(!label.is_empty(), "empty endonym for {lang:?}");
            assert!(
                !label.contains('{') && !label.contains('}'),
                "endonym for {lang:?} contains braces (literal-key bug): {label}"
            );
        }
        // Spot-check specific values.
        assert_eq!(Language::En.label(), "English");
        assert_eq!(Language::Uk.label(), "Українська");
        assert_eq!(Language::De.label(), "Deutsch");
    }

    /// flag() returns a non-empty emoji flag for every language.
    /// country_code() returns a non-empty, distinct, lowercase ISO-2 code for
    /// every language (the CSS modifier for the inline SVG flag).
    #[test]
    fn country_code_nonempty_distinct_lowercase() {
        let codes: Vec<&str> = Language::all().iter().map(|l| l.country_code()).collect();
        for c in &codes {
            assert!(!c.is_empty(), "empty country code");
            assert!(
                c.chars().all(|ch| ch.is_ascii_lowercase()),
                "country code not lowercase ascii: {c}"
            );
        }
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "duplicate country code");
        // Spot-check.
        assert_eq!(Language::En.country_code(), "gb");
        assert_eq!(Language::Uk.country_code(), "ua");
        assert_eq!(Language::Pt.country_code(), "pt");
    }
}
