//! Result of a bulk operation over a batch of CSV rows.
//!
//! These types previously lived in the `core-bulk` crate (orchestration via a
//! `BulkOperation` trait, `DashMap`, `Semaphore`), but that whole engine was
//! dead — the actual processing goes through webview JS (`fetch()` inside the
//! login webview), and the result is aggregated manually in
//! [`crate::ops::launch_op`]. The types were moved here as the live part; the
//! dead `core-bulk` crate was removed.

/// Result of bulk processing a batch of rows.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Number of successful processing.
    pub succeeded: u64,
    /// Number of rejections.
    pub failed: u64,
    /// Rejected rows with reasons: (username, domain, error reason).
    pub failures: Vec<RowFailure>,
    /// Credentials of successfully created accounts (domain, username, password,
    /// display name, quota). Needed for export: passwords live only in
    /// `editable_rows`, which is reset on a target (tab) change, after which the
    /// export would lose data. The snapshot in `BatchResult` captures the passwords
    /// current at the time of the operation (valid and confirmed by the server).
    pub created_credentials: Vec<CredentialRow>,
}

/// Credentials of a created account for export.
///
/// The values are taken from a valid `SanitizedUserRow` (typestate pipeline)
/// only for successfully executed operations, so the password is guaranteed to
/// match what was sent to the server.
#[derive(Debug, Clone)]
pub struct CredentialRow {
    /// Domain.
    pub domain: String,
    /// Username (without the domain).
    pub username: String,
    /// Password (in plaintext — as in the editable table/source CSV; encrypted
    /// for an encrypted export via XChaCha20-Poly1305).
    pub password: String,
    /// Display name.
    pub display_name: String,
    /// Quota (MiB).
    pub quota_mb: u32,
}

/// Record of a rejected row.
#[derive(Debug, Clone)]
pub struct RowFailure {
    /// Username (from the row).
    pub username: String,
    /// Domain (from the row).
    pub domain: String,
    /// Rejection reason.
    pub reason: String,
}
