//! Application configuration without recompilation (a TOML file next to the binary).
//!
//! At startup it reads (or creates a sample) `config.toml` in the local data
//! folder next to the binary (portability). See [`config_path`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use mailgrit_core_domain::PasswordGenerator;
use std::path::PathBuf;

/// Settings loaded from TOML.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
/// `require_special` (see [`PolicyClasses`]'s manual serde impl + `#[serde(flatten)]`).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    pub fn to_policy(&self) -> mailgrit_core_domain::PasswordPolicy {
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
/// (see [`GeneratorClasses`]'s manual serde impl + `#[serde(flatten)]`).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    pub fn to_generator(&self) -> PasswordGenerator {
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
// Stored as a fixed `[bool; 4]` array (a single field — keeps the parent config
// struct below clippy's `struct_excessive_bools` limit). The flags are accessed
// via destructuring (never raw indexing, to satisfy `indexing_slicing`). Each
// type implements serde manually so the TOML schema stays flat with the original
// named keys (require_*/use_*), via `#[serde(flatten)]` on the parent.
// ============================================================================

/// Indexes into the `[bool; 4]` array: `[uppercase, lowercase, digit, special]`.
const CLS_UPPER: usize = 0;
const CLS_LOWER: usize = 1;
const CLS_DIGIT: usize = 2;
const CLS_SPECIAL: usize = 3;

/// Required character classes for the server-side password policy
/// (`[password_policy]`). Serializes to the flat TOML keys `require_uppercase`/
/// `require_lowercase`/`require_number`/`require_special` (all default `true`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyClasses([bool; 4]);

impl Default for PolicyClasses {
    fn default() -> Self {
        // Mirrors the original per-field serde default (`default_pp_true`).
        Self([true, true, true, true])
    }
}

impl PolicyClasses {
    /// Returns the four flags as a tuple `(uppercase, lowercase, number, special)`.
    #[must_use]
    pub fn into_tuple(self) -> (bool, bool, bool, bool) {
        self.0.into()
    }
}

/// Enabled character classes for the password generator (`[password_generator]`).
/// Serializes to the flat TOML keys `use_uppercase`/`use_lowercase`/`use_digits`/
/// `use_special` (all default `true`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorClasses([bool; 4]);

impl Default for GeneratorClasses {
    fn default() -> Self {
        // Mirrors the original per-field serde default (`default_pg_true`).
        Self([true, true, true, true])
    }
}

impl GeneratorClasses {
    /// Returns the four flags as a tuple `(uppercase, lowercase, digits, special)`.
    #[must_use]
    pub fn into_tuple(self) -> (bool, bool, bool, bool) {
        self.0.into()
    }
}

/// Serializes a `[bool; 4]` class set as four named TOML/map keys (in declaration order).
fn serialize_classes<S>(
    serializer: S,
    classes: [bool; 4],
    keys: [&'static str; 4],
    type_name: &'static str,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeStruct;
    let [upper, lower, digit, special] = classes;
    let mut state = serializer.serialize_struct(type_name, 4)?;
    state.serialize_field(keys[CLS_UPPER], &upper)?;
    state.serialize_field(keys[CLS_LOWER], &lower)?;
    state.serialize_field(keys[CLS_DIGIT], &digit)?;
    state.serialize_field(keys[CLS_SPECIAL], &special)?;
    state.end()
}

/// Deserializes a `[bool; 4]` class set from four named map keys, defaulting each
/// missing key to `default_value`. Unknown keys are ignored (forward-compatible).
fn deserialize_classes<'de, D>(
    deserializer: D,
    keys: [&'static str; 4],
    default_value: bool,
) -> Result<[bool; 4], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Visitor;

    struct ClassVisitor {
        keys: [&'static str; 4],
        default_value: bool,
    }

    impl<'de> Visitor<'de> for ClassVisitor {
        type Value = [bool; 4];

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a character-class mapping of boolean flags")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut flags = [self.default_value; 4];
            while let Some(key) = map.next_key::<std::borrow::Cow<'de, str>>()? {
                match key.as_ref() {
                    k if k == self.keys[CLS_UPPER] => flags[CLS_UPPER] = map.next_value()?,
                    k if k == self.keys[CLS_LOWER] => flags[CLS_LOWER] = map.next_value()?,
                    k if k == self.keys[CLS_DIGIT] => flags[CLS_DIGIT] = map.next_value()?,
                    k if k == self.keys[CLS_SPECIAL] => flags[CLS_SPECIAL] = map.next_value()?,
                    // Unknown key: ignore (forward-compatible with future config keys).
                    _ => {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
            }
            Ok(flags)
        }
    }

    deserializer.deserialize_map(ClassVisitor {
        keys,
        default_value,
    })
}

impl serde::Serialize for PolicyClasses {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_classes(
            serializer,
            self.0,
            [
                "require_uppercase",
                "require_lowercase",
                "require_number",
                "require_special",
            ],
            "PasswordPolicyConfig",
        )
    }
}

impl<'de> serde::Deserialize<'de> for PolicyClasses {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_classes(
            deserializer,
            [
                "require_uppercase",
                "require_lowercase",
                "require_number",
                "require_special",
            ],
            default_pp_true(),
        )
        .map(Self)
    }
}

impl serde::Serialize for GeneratorClasses {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serialize_classes(
            serializer,
            self.0,
            [
                "use_uppercase",
                "use_lowercase",
                "use_digits",
                "use_special",
            ],
            "PasswordGeneratorConfig",
        )
    }
}

impl<'de> serde::Deserialize<'de> for GeneratorClasses {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_classes(
            deserializer,
            [
                "use_uppercase",
                "use_lowercase",
                "use_digits",
                "use_special",
            ],
            default_pg_true(),
        )
        .map(Self)
    }
}

/// Returns the path to config.toml — next to the binary (portability).
pub fn config_path() -> PathBuf {
    crate::app_data_dir().join("config.toml")
}

/// Saves the current settings back to config.toml. Errors are logged, no panic.
pub fn save(settings: &Settings) {
    let path = config_path();
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!("creating config.toml directory {}: {e}", parent.display());
    }
    match toml::to_string_pretty(settings) {
        Ok(toml_str) => {
            let with_header = format!(
                "# MailGrit configuration. Edit for your iRedAdmin.\n\
                 # theme: \"dark\" or \"light\".\n\
                 # language: \"en\", \"de\", \"fr\", \"es\", \"it\", \"pt\", \"nl\", \
                 \"pl\", \"uk\", \"ru\".\n\n{toml_str}"
            );
            if let Err(e) = std::fs::write(&path, with_header) {
                tracing::warn!("writing config.toml: {e}");
            }
        }
        Err(e) => tracing::warn!("serializing config.toml: {e}"),
    }
}

/// Updates only the theme field in config.toml, preserving the other settings.
/// Reading and writing TOML happen in `block_in_place`, so as not to block the event loop.
pub fn save_theme(theme: &str) {
    let theme = theme.to_string();
    tokio::task::block_in_place(|| {
        let mut settings = load_or_create();
        settings.theme = theme;
        save(&settings);
    });
}

/// Updates only the language field in config.toml, preserving the other settings.
/// Reading and writing TOML happen in `block_in_place`, so as not to block the event loop.
pub fn save_language(language: &str) {
    let language = language.to_string();
    tokio::task::block_in_place(|| {
        let mut settings = load_or_create();
        settings.language = language;
        save(&settings);
    });
}

/// Loads settings from TOML. If the file is missing, it creates a sample and
/// returns the defaults. Parse errors yield the defaults with a warning (no panic).
pub fn load_or_create() -> Settings {
    let path = config_path();
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
            if let Err(e) = std::fs::write(&path, with_header) {
                tracing::warn!("writing sample config.toml {}: {e}", path.display());
            }
        }
        return sample;
    }
    match std::fs::read_to_string(&path) {
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
