//! UI internationalization layer.
//!
//! Strings live in `locales/app.<lang>.yml`, are embedded into the binary by the
//! `rust_i18n::i18n!` macro in `main.rs`, and the current language is set globally
//! via [`rust_i18n::set_locale`] in `use_effect` (see `main.rs`).
//!
//! ## Macros
//!
//! - `t!` (from `rust_i18n`) returns a `Cow<str>`. Suitable for ordinary code
//!   where the result is immediately wrapped (`.to_string()`, etc.).
//! - `tr!` (defined in `main.rs`) is a thin wrapper over `t!` returning a `String`.
//!   Needed for RSX: Dioxus `IntoDynNode` is implemented for `String`/`&str` but
//!   not for `Cow<str>`. Usage in RSX: `{ tr!("key") }`.
//!
//! ## Dioxus re-render pattern
//!
//! `rust_i18n::t!`/`tr!` read the **global** locale, not a Dioxus signal. So every
//! component that renders localized text must **additionally** read
//! `state.read().language`, otherwise Dioxus will not re-subscribe it on language
//! change. In practice this is a single line `let _ = state.read().language;` at
//! the top of the component (similar to reading `theme`).

#![forbid(unsafe_code)]

/// Helper for the `tr!` macro (main.rs): converts the `Cow<str>` from
/// `rust_i18n::t!` into a `String` accepted by Dioxus `IntoDynNode`.
/// `#[doc(hidden)]` marks it as a macro implementation detail.
#[doc(hidden)]
#[must_use]
pub fn __cow_to_string(cow: std::borrow::Cow<'_, str>) -> String {
    cow.into_owned()
}

#[cfg(test)]
pub mod tests {

    use rust_i18n::t;

    // Tests that mutate the global rust_i18n locale are serialized through a
    // shared Mutex: parallel runs caused non-deterministic failures (a race on
    // global state). This lock is also reused by the error_i18n tests module.
    pub static LOCALE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// English (the default fallback): keys resolve to a non-empty string
    /// different from the literal key. Uses an explicit `locale =` without
    /// touching the global locale.
    #[test]
    fn en_keys_resolve() {
        let keys = [
            "nav.operations",
            "nav.audit",
            "action.create",
            "action.delete",
            "session.active",
            "csv.not_loaded",
            "op_label.user.create",
            "err_code.NO_SUCH_ACCOUNT",
            "login.tagline",
            "lang.uk",
        ];
        for k in keys {
            let s = t!(k, locale = "en").to_string();
            assert!(!s.is_empty(), "empty en translation for key {k}");
            assert_ne!(s, k, "key {k} not found in catalog (returned as literal)");
        }
    }

    /// Switching the global locale changes the returned text (uk != en).
    #[test]
    fn locale_switch_changes_text() {
        let _guard = LOCALE_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rust_i18n::set_locale("en");
        let en = t!("nav.operations").to_string();
        rust_i18n::set_locale("uk");
        let uk = t!("nav.operations").to_string();
        assert_ne!(en, uk, "text did not change when switching locale");
        assert_eq!(uk, "Операції");
        rust_i18n::set_locale("en");
    }

    /// Language endonyms (`lang.<code>`) are present for every supported language.
    #[test]
    fn all_languages_have_labels() {
        for code in ["en", "de", "fr", "es", "it", "pt", "nl", "pl", "uk"] {
            let key = format!("lang.{code}");
            let label = t!(key.as_str(), locale = code).to_string();
            assert!(!label.is_empty(), "empty endonym for {code}");
        }
    }

    /// All translated locales are loaded: `nav.operations` differs from the
    /// en-fallback and from the literal key. Uses an explicit `locale =`.
    #[test]
    fn all_translated_locales_loaded() {
        let cases: &[(&str, &str)] = &[
            ("de", "Vorgänge"),
            ("fr", "Opérations"),
            ("es", "Operaciones"),
            ("it", "Operazioni"),
            ("pt", "Operações"),
            ("nl", "Bewerkingen"),
            ("pl", "Operacje"),
            ("uk", "Операції"),
        ];
        for (code, expected) in cases {
            let s = t!("nav.operations", locale = code).to_string();
            assert_ne!(
                s, "Operations",
                "locale {code} returned the en-fallback — catalog not loaded"
            );
            assert_ne!(
                s, "nav.operations",
                "locale {code} returned the literal key — entry is missing"
            );
            assert_eq!(
                s, *expected,
                "locale {code}: nav.operations does not match the expected translation"
            );
        }
    }

    /// All interpolation placeholders resolve in every language (no `%{x}` left
    /// in the output): a guard against variable-name drift between en and the
    /// translations.
    #[test]
    fn interpolation_placeholders_resolve_in_all_locales() {
        for code in ["en", "de", "fr", "es", "it", "pt", "nl", "pl", "uk"] {
            let samples: [String; 4] = [
                t!("result.succeeded", count = 7u64, locale = code).to_string(),
                t!(
                    "csv.loaded_summary",
                    valid = 5u64,
                    failed = 2u64,
                    locale = code
                )
                .to_string(),
                t!("pw.length_label", n = 16u64, locale = code).to_string(),
                t!(
                    "err.domain.invalid_char",
                    char = 'x',
                    pos = 3u64,
                    locale = code
                )
                .to_string(),
            ];
            for s in samples {
                assert!(
                    !s.contains("%{"),
                    "locale {code}: unresolved placeholder in \"{s}\" (name drift)"
                );
                assert!(!s.is_empty(), "locale {code}: empty interpolation result");
            }
        }
    }
}
