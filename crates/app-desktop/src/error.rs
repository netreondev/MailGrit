//! Typed application errors (replaces `Result<_, String>` at module boundaries).
//!
//! The core crates carry strict `thiserror` enums; the app layer used to
//! flatten them into `String` at every boundary, losing the KDF/AEAD/SQLite
//! distinction and forcing substring matching on message text. Errors are data
//! here; the user-facing localized text is produced at the display boundary via
//! [`error_i18n`](crate::error_i18n) / [`AppError::user_message`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::audit_ui::AuditError;

/// Errors of the base-URL validation (`util::validate_base_url`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlError {
    /// Not a parseable URL at all.
    #[error("invalid URL")]
    Invalid,
    /// The scheme is not `https` (a session over HTTP would leak the cookie).
    #[error("https is required (scheme: {scheme})")]
    NotHttps {
        /// The scheme that was found.
        scheme: String,
    },
    /// The URL has no host part.
    #[error("no host")]
    NoHost,
}

/// Application-layer error: wraps the core-crate errors plus the app-specific
/// failure modes, preserving the source type down to the display boundary.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Audit-log open/read/write and master-password problems.
    #[error("audit: {0}")]
    Audit(#[from] AuditError),
    /// URL validation.
    #[error("url: {0}")]
    Url(#[from] UrlError),
    /// Base-URL string is not a usable URL at all (cookie extraction).
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),
    /// The login webview is not open (cookie extraction requested too early).
    #[error("login window is not open")]
    LoginWindowClosed,
    /// A webview/cookie API failure.
    #[error("webview: {0}")]
    WebView(String),
    /// Cryptography (core-security).
    #[error("crypto: {0}")]
    Crypto(#[from] mailgrit_core_security::SecurityError),
    /// CSV parsing (core-csv).
    #[error("csv: {0}")]
    Csv(#[from] mailgrit_core_csv::CsvParseError),
    /// Local storage (core-storage).
    #[error("storage: {0}")]
    Storage(#[from] mailgrit_core_storage::StorageError),
    /// Filesystem I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Anything else (window creation, JS injection) — kept as context text.
    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Localized, user-facing message (via the translation catalog). This is
    /// the ONLY place where an [`AppError`] becomes display text.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Url(e) => crate::error_i18n::url_error(e),
            Self::Audit(AuditError::WrongMasterPassword) => t!("master_password.wrong").to_string(),
            Self::Audit(AuditError::CorruptedKeyFile { .. }) => {
                t!("master_password.corrupt_key").to_string()
            }
            Self::Audit(e) => e.to_string(),
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The KDF/AEAD/SQLite distinction must survive to this layer (audit
    // finding: everything used to collapse into AuditError::Storage(String)).
    #[test]
    fn audit_error_kinds_stay_distinct() {
        let wrong = AppError::Audit(AuditError::WrongMasterPassword);
        let corrupted = AppError::Audit(AuditError::CorruptedKeyFile { actual: 7 });
        assert_ne!(
            wrong.user_message(),
            corrupted.user_message(),
            "wrong-password and damaged-key must remain distinguishable"
        );
    }

    #[test]
    fn url_error_is_typed() {
        let e = UrlError::NotHttps {
            scheme: "http".into(),
        };
        assert!(e.to_string().contains("http"));
        let app = AppError::from(UrlError::NoHost);
        assert!(matches!(app, AppError::Url(UrlError::NoHost)));
    }
}
