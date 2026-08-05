//! Storage domain errors (spec §19).

use mailgrit_core_security::SecurityError;

/// Error of a local-storage operation.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// SQLite error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Audit hash-chain integrity violation (the log was tampered with).
    #[error("audit log integrity violation: {0}")]
    ChainBroken(#[from] SecurityError),
    /// The audit entry itself is corrupted: the hash blob has the wrong length
    /// (truncated or extended). Unlike [`Self::ChainBroken`] (payload tampering
    /// with a correct structure), here the storage structure is broken — such
    /// data must not be silently turned into zeros and continued, or
    /// verification would incorrectly point at the next entry.
    #[error("audit entry #{id} corrupted: hash blob is {actual} bytes (expected {expected})")]
    CorruptedEntry {
        /// ID of the corrupted entry (matches `audit_log.id`).
        id: i64,
        /// Expected hash-blob length (`HMAC_LEN`).
        expected: usize,
        /// Actual hash-blob length in the DB.
        actual: usize,
    },
}

/// Result of a storage operation.
pub type StorageResult<T> = Result<T, StorageError>;
