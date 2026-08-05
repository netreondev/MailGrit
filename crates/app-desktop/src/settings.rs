//! Application configuration without recompilation (a TOML file next to the binary).
//!
//! At startup it reads (or creates a sample) `config.toml` in the local data
//! folder next to the binary (portability). See [`config_path`].

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
#[allow(
    clippy::struct_excessive_bools,
    reason = "4 character-class flags mirror PasswordPolicy and the TOML on/off toggles"
)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PasswordPolicyConfig {
    /// Minimum password length (in characters). Default 8 (like iRedAdmin).
    #[serde(default = "default_pp_min_len")]
    pub min_len: usize,
    /// An uppercase letter is required. Default true.
    #[serde(default = "default_pp_true")]
    pub require_uppercase: bool,
    /// A lowercase letter is required. Default true.
    #[serde(default = "default_pp_true")]
    pub require_lowercase: bool,
    /// A digit is required. Default true.
    #[serde(default = "default_pp_true")]
    pub require_number: bool,
    /// A special character is required. Default true.
    #[serde(default = "default_pp_true")]
    pub require_special: bool,
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
            require_uppercase: default_pp_true(),
            require_lowercase: default_pp_true(),
            require_number: default_pp_true(),
            require_special: default_pp_true(),
        }
    }
}

impl PasswordPolicyConfig {
    /// Converts the configuration into the domain [`PasswordPolicy`] (field mapping).
    #[must_use]
    pub const fn to_policy(&self) -> mailgrit_core_domain::PasswordPolicy {
        mailgrit_core_domain::PasswordPolicy {
            min_len: self.min_len,
            require_uppercase: self.require_uppercase,
            require_lowercase: self.require_lowercase,
            require_number: self.require_number,
            require_special: self.require_special,
        }
    }
}

/// [password_generator] TOML section: password generator for the editable table.
/// Parsed by serde, converted into [`PasswordGenerator`] via [`Self::to_generator`].
#[allow(
    clippy::struct_excessive_bools,
    reason = "4 character-class flags mirror PasswordGenerator and the TOML on/off toggles"
)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PasswordGeneratorConfig {
    /// Target password length (in characters). Default 16.
    #[serde(default = "default_pg_length")]
    pub length: usize,
    /// Use uppercase letters. Default true.
    #[serde(default = "default_pg_true")]
    pub use_uppercase: bool,
    /// Use lowercase letters. Default true.
    #[serde(default = "default_pg_true")]
    pub use_lowercase: bool,
    /// Use digits. Default true.
    #[serde(default = "default_pg_true")]
    pub use_digits: bool,
    /// Use special characters. Default true.
    #[serde(default = "default_pg_true")]
    pub use_special: bool,
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
            use_uppercase: default_pg_true(),
            use_lowercase: default_pg_true(),
            use_digits: default_pg_true(),
            use_special: default_pg_true(),
        }
    }
}

impl PasswordGeneratorConfig {
    /// Converts the configuration into the domain [`PasswordGenerator`].
    #[must_use]
    pub const fn to_generator(&self) -> PasswordGenerator {
        PasswordGenerator {
            length: self.length,
            use_uppercase: self.use_uppercase,
            use_lowercase: self.use_lowercase,
            use_digits: self.use_digits,
            use_special: self.use_special,
        }
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
