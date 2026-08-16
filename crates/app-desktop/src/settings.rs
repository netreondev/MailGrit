//! Application configuration without recompilation (a TOML file next to the binary).
//!
//! At startup it reads (or creates a sample) `config.toml` in the local data
//! folder next to the binary (portability). See [`config_path`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use mailgrit_core_domain::PasswordGenerator;
use std::path::PathBuf;

/// Settings loaded from TOML.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    /// Base URL of iRedAdmin (e.g. https://mail.example.com/iredadmin).
    #[serde(default)]
    pub base_url: String,
    /// Session cookie name. Default `webpy_session_id` — the actual iRedAdmin
    /// session name on web.py, NOT the Django-style `sessionid`.
    #[serde(default = "default_session_cookie")]
    pub session_cookie_name: String,
    /// UI theme: "dark" (default) or "light".
    #[serde(default = "default_theme")]
    pub theme: String,
    /// UI language (BCP-47 code). Default "en"; an unknown value falls back to English.
    #[serde(default = "default_language")]
    pub language: String,
    /// [password_policy] section: server-side policy for the strength indicator in the table.
    #[serde(default)]
    pub password_policy: PasswordPolicyConfig,
    /// [password_generator] section: password generator for the editable table.
    #[serde(default)]
    pub password_generator: PasswordGeneratorConfig,
}

fn default_theme() -> String {
    "dark".into()
}
fn default_language() -> String {
    "en".into()
}
fn default_session_cookie() -> String {
    // The actual iRedAdmin session name (web.py), NOT the Django-style `sessionid`.
    "webpy_session_id".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            base_url: String::from("https://mail.example.com/iredadmin"),
            session_cookie_name: default_session_cookie(),
            theme: default_theme(),
            language: default_language(),
            password_policy: PasswordPolicyConfig::default(),
            password_generator: PasswordGeneratorConfig::default(),
        }
    }
}

/// [password_policy] TOML section: server-side password policy for the strength
/// indicator in the editable table. Parsed by serde, converted into
/// [`PasswordPolicy`] via [`Self::to_policy`] (the parse-don't-validate boundary).
///
/// The four character-class requirement flags live in [`PolicyClasses`] (a single
/// sub-struct field) so this struct stays below clippy's `struct_excessive_bools`
/// threshold. The TOML schema is unchanged: the four flags still serialize as the
/// flat keys `require_uppercase`/`require_lowercase`/`require_number`/
/// `require_special` (derived serde + `#[serde(flatten)]`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PasswordPolicyConfig {
    /// Minimum password length (in characters). Default 8 (like iRedAdmin).
    #[serde(default = "default_pp_min_len")]
    pub min_len: usize,
    /// Required character classes (flattened into the TOML section).
    #[serde(flatten)]
    pub classes: PolicyClasses,
}

const fn default_pp_min_len() -> usize {
    8
}
const fn default_pp_true() -> bool {
    true
}

impl Default for PasswordPolicyConfig {
    fn default() -> Self {
        Self {
            min_len: default_pp_min_len(),
            classes: PolicyClasses::default(),
        }
    }
}

impl PasswordPolicyConfig {
    /// Converts the configuration into the domain [`PasswordPolicy`] (field mapping).
    #[must_use]
    pub const fn to_policy(&self) -> mailgrit_core_domain::PasswordPolicy {
        let (uppercase, lowercase, number, special) = self.classes.into_tuple();
        mailgrit_core_domain::PasswordPolicy {
            min_len: self.min_len,
            classes: mailgrit_core_domain::CharacterClasses::from_tuple((
                uppercase, lowercase, number, special,
            )),
        }
    }
}

/// [password_generator] TOML section: password generator for the editable table.
/// Parsed by serde, converted into [`PasswordGenerator`] via [`Self::to_generator`].
///
/// The four character-class toggles live in [`GeneratorClasses`] (a single
/// sub-struct field) so this struct stays below clippy's `struct_excessive_bools`
/// threshold. The TOML schema is unchanged: the flags still serialize as the flat
/// keys `use_uppercase`/`use_lowercase`/`use_digits`/`use_special`
/// (derived serde + `#[serde(flatten)]`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct PasswordGeneratorConfig {
    /// Target password length (in characters). Default 16.
    #[serde(default = "default_pg_length")]
    pub length: usize,
    /// Enabled character classes (flattened into the TOML section).
    #[serde(flatten)]
    pub classes: GeneratorClasses,
}

const fn default_pg_length() -> usize {
    16
}
const fn default_pg_true() -> bool {
    true
}

impl Default for PasswordGeneratorConfig {
    fn default() -> Self {
        Self {
            length: default_pg_length(),
            classes: GeneratorClasses::default(),
        }
    }
}

impl PasswordGeneratorConfig {
    /// Converts the configuration into the domain [`PasswordGenerator`].
    #[must_use]
    pub const fn to_generator(&self) -> PasswordGenerator {
        let (uppercase, lowercase, digits, special) = self.classes.into_tuple();
        PasswordGenerator {
            length: self.length,
            classes: mailgrit_core_domain::CharacterClasses::from_tuple((
                uppercase, lowercase, digits, special,
            )),
        }
    }
}

// ============================================================================
// Character-class flag sets for the password policy/generator config sections.
//
// Named-field structs with DERIVED serde (each key defaults to `true`).
// Previously ~145 lines of hand-rolled Serialize/Deserialize over a raw
// `[bool; 4]` (a Visitor, a serializer helper, four impls and index constants)
// existed "to keep the flat TOML keys" — derive + #[serde(flatten)] produces
// the identical schema (require_*/use_* keys, per-key default, unknown keys
// ignored) with none of the drift risk. The sub-struct also keeps the parent
// config below clippy's `struct_excessive_bools` threshold.
// ============================================================================

/// Required character classes for the server-side password policy
/// (`[password_policy]`). Flattens to the TOML keys `require_uppercase`/
/// `require_lowercase`/`require_number`/`require_special` (all default `true`).
// Four named bools are the SCHEMA (one per TOML key); the previous [bool; 4]
// wrapper dodged this lint while meaning exactly the same thing.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PolicyClasses {
    /// Require at least one uppercase letter (A–Z).
    #[serde(default = "default_pp_true")]
    pub require_uppercase: bool,
    /// Require at least one lowercase letter (a–z).
    #[serde(default = "default_pp_true")]
    pub require_lowercase: bool,
    /// Require at least one digit (0–9).
    #[serde(default = "default_pp_true")]
    pub require_number: bool,
    /// Require at least one special character.
    #[serde(default = "default_pp_true")]
    pub require_special: bool,
}

impl Default for PolicyClasses {
    fn default() -> Self {
        Self {
            require_uppercase: default_pp_true(),
            require_lowercase: default_pp_true(),
            require_number: default_pp_true(),
            require_special: default_pp_true(),
        }
    }
}

impl PolicyClasses {
    /// Returns the four flags as a tuple `(uppercase, lowercase, number, special)`.
    #[must_use]
    pub const fn into_tuple(self) -> (bool, bool, bool, bool) {
        (
            self.require_uppercase,
            self.require_lowercase,
            self.require_number,
            self.require_special,
        )
    }
}

/// Enabled character classes for the password generator
/// (`[password_generator]`). Flattens to the TOML keys `use_uppercase`/
/// `use_lowercase`/`use_digits`/`use_special` (all default `true`).
// See PolicyClasses for the struct_excessive_bools rationale.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeneratorClasses {
    /// Include uppercase letters (A–Z).
    #[serde(default = "default_pg_true")]
    pub use_uppercase: bool,
    /// Include lowercase letters (a–z).
    #[serde(default = "default_pg_true")]
    pub use_lowercase: bool,
    /// Include digits (0–9).
    #[serde(default = "default_pg_true")]
    pub use_digits: bool,
    /// Include special characters.
    #[serde(default = "default_pg_true")]
    pub use_special: bool,
}

impl Default for GeneratorClasses {
    fn default() -> Self {
        Self {
            use_uppercase: default_pg_true(),
            use_lowercase: default_pg_true(),
            use_digits: default_pg_true(),
            use_special: default_pg_true(),
        }
    }
}

impl GeneratorClasses {
    /// Returns the four flags as a tuple `(uppercase, lowercase, digits, special)`.
    #[must_use]
    pub const fn into_tuple(self) -> (bool, bool, bool, bool) {
        (
            self.use_uppercase,
            self.use_lowercase,
            self.use_digits,
            self.use_special,
        )
    }
}

/// Returns the path to config.toml — next to the binary (portability).
pub fn config_path() -> PathBuf {
    crate::app_data_dir().join("config.toml")
}

/// Saves settings to an explicit path (the only writer — production code
/// reaches it through the serialized [`save_field_at`] cycle). Errors are
/// logged, no panic.
fn save_to(path: &std::path::Path, settings: &Settings) {
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!("creating config.toml directory {}: {e}", parent.display());
    }
    match toml::to_string_pretty(settings) {
        Ok(toml_str) => {
            // NOTE: only list languages that `Language::from_config` actually
            // supports — a "ru" here used to fall back to English silently.
            let with_header = format!(
                "# MailGrit configuration. Edit for your iRedAdmin.\n\
                 # theme: \"dark\" or \"light\".\n\
                 # language: \"en\", \"de\", \"fr\", \"es\", \"it\", \"pt\", \"nl\", \
                 \"pl\", \"uk\".\n\n{toml_str}"
            );
            if let Err(e) = std::fs::write(path, with_header) {
                tracing::warn!("writing config.toml: {e}");
            }
        }
        Err(e) => tracing::warn!("serializing config.toml: {e}"),
    }
}

/// Serializes concurrent read-modify-write cycles of config.toml.
///
/// Two detached field saves (e.g. the theme toggled and the language switched
/// in quick succession) used to interleave: the later writer loaded a STALE
/// snapshot before the earlier writer saved, silently reverting its field.
/// The guard spans the whole load→modify→save, so every cycle starts from the
/// previous cycle's result.
static CONFIG_WRITE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs one serialized load→modify→save cycle on config.toml (the sync core of
/// [`save_field`]; separate so tests can drive it without the tokio runtime).
/// The lock recovers from poisoning: a panic mid-cycle leaves the file either
/// fully written or untouched (`fs::write` is a single call), so the next
/// cycle may proceed.
fn save_field_blocking(apply: impl FnOnce(&mut Settings)) {
    save_field_at(&config_path(), apply);
}

/// The path-parameterized core of [`save_field_blocking`] — the REAL
/// config.toml is only ever touched through `config_path()`, tests drive a
/// temp path. The lock serializes cycles regardless of the target path: the
/// production writers all target the same file.
fn save_field_at(path: &std::path::Path, apply: impl FnOnce(&mut Settings)) {
    let _guard = CONFIG_WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut settings = load_from(path);
    apply(&mut settings);
    save_to(path, &settings);
}

/// Updates one field of config.toml in place, preserving the other settings.
///
/// The read-modify-write runs via `spawn_blocking`: `block_in_place` PANICS on
/// a current-thread runtime (the fallback infra.rs may build), and would still
/// stall the event loop on a slow disk. Fire-and-forget: the UI applies the
/// change immediately, config.toml follows (serialized by
/// [`CONFIG_WRITE_LOCK`]).
fn save_field(apply: impl FnOnce(&mut Settings) + Send + 'static) {
    // Detached: dropping the handle leaves the task running (fire-and-forget).
    let join = crate::tokio_runtime().spawn_blocking(move || save_field_blocking(apply));
    drop(join);
}

/// Updates only the theme field in config.toml, preserving the other settings.
/// See [`save_field`] for the concurrency/spawn_blocking rationale.
pub fn save_theme(theme: &str) {
    let theme = theme.to_string();
    save_field(move |s| s.theme = theme);
}

/// Updates only the language field in config.toml, preserving the other settings.
/// See [`save_field`] for the concurrency/spawn_blocking rationale.
pub fn save_language(language: &str) {
    let language = language.to_string();
    save_field(move |s| s.language = language);
}

/// Loads settings from TOML. If the file is missing, it creates a sample and
/// returns the defaults. Parse errors yield the defaults with a warning (no panic).
pub fn load_or_create() -> Settings {
    load_from(&config_path())
}

/// Loads settings from an explicit path (the testable core of
/// [`load_or_create`]).
fn load_from(path: &std::path::Path) -> Settings {
    if !path.exists() {
        // Create the directory and a sample config (the user will edit it as needed).
        // Errors are logged, no panic — in-memory settings are the defaults and valid.
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!("creating config.toml directory {}: {e}", parent.display());
        }
        let sample = Settings::default();
        if let Ok(toml_str) = toml::to_string_pretty(&sample) {
            let with_header =
                format!("# MailGrit configuration. Edit for your iRedAdmin.\n\n{toml_str}");
            if let Err(e) = std::fs::write(path, with_header) {
                tracing::warn!("writing sample config.toml {}: {e}", path.display());
            }
        }
        return sample;
    }
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let mut settings: Settings = toml::from_str(&contents).unwrap_or_else(|e| {
                tracing::warn!("config.toml failed to parse ({e}), using defaults");
                Settings::default()
            });
            // Migrate a stale/empty cookie name to the current default.
            if settings.session_cookie_name == "sessionid"
                || settings.session_cookie_name.is_empty()
            {
                settings.session_cookie_name = default_session_cookie();
            }
            settings
        }
        Err(e) => {
            tracing::warn!("reading config.toml: {e}");
            Settings::default()
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
