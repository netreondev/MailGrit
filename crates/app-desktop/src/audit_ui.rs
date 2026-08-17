//! Wiring of the hash-chained audit log (`core-storage`) to the UI.
//!
//! The audit log lives in a `SQLite` file in the application's data directory. The
//! HMAC key is generated in memory at startup (for chain verification within a
//! session). Each bulk operation is recorded via
//! [`AuditWriter::append_op`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::batch::BatchResult;
use mailgrit_core_security::EncryptionKey;
use mailgrit_core_storage::{AuditAction, AuditEntry, AuditLog};
use std::path::Path;
use std::sync::Mutex;

/// An error working with the audit log.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// Failed to open/create the DB or a read error (transient, not tampering).
    #[error("audit DB error: {0}")]
    Storage(String),
    /// Argon2id key derivation failed (distinct from DB errors — a KDF
    /// parameter/input problem, not a corrupted log).
    #[error("audit KDF error: {0}")]
    Kdf(String),
    /// HMAC/AEAD failure while verifying or deriving the audit key.
    #[error("audit crypto error: {0}")]
    Crypto(String),
    /// A hash-chain integrity violation (chain mismatch — possible tampering
    /// or corruption). Distinct
    /// from [`Storage`](Self::Storage) so the UI does not falsely report
    /// "tampering" on any `SQLite` error.
    #[error("audit log integrity violation: {0}")]
    Tampered(String),
    /// The audit-log mutex is poisoned (a panic while locked).
    #[error("audit log mutex is poisoned")]
    PoisonedLock,
    /// Wrong master password (the derived key does not match the stored one).
    #[error("wrong audit master password")]
    WrongMasterPassword,
    /// The audit key file exists but is damaged (wrong length). Unlike a missing
    /// file (first run → key creation), damage means the saved audit history can
    /// no longer be verified: a new key would create a new chain, and legitimate
    /// records would look forged. Therefore we do NOT silently recreate the key,
    /// but report an error.
    #[error("audit key file is damaged (wrong length): {actual} bytes")]
    CorruptedKeyFile { actual: usize },
}

/// A writer to the audit log (inside a `Mutex`, because `AuditLog` is not `Sync`
/// over `Connection`).
pub struct AuditWriter {
    log: Mutex<AuditLog>,
}

impl AuditWriter {
    /// Opens (or creates) the audit log in the `mailgrit-audit.sqlite` file
    /// in the application's local data directory.
    ///
    /// The master password protects the audit key via the Argon2id KDF (see
    /// [`load_or_create_persistent_key`]): without it, the key (and chain
    /// integrity verification across runs) is unavailable. On the first run, a
    /// new salt and key are created; on subsequent runs, the password is
    /// verified.
    ///
    /// # Errors
    ///
    /// - [`AuditError::Storage`] — a DB/FS error.
    /// - [`AuditError::WrongMasterPassword`] — the password does not match the
    ///   stored one.
    pub fn open(master_password: &str) -> Result<Self, AuditError> {
        let dir = crate::app_data_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return Err(AuditError::Storage(format!(
                "failed to create directory {}: {e}",
                dir.display()
            )));
        }
        let path = dir.join("mailgrit-audit.sqlite");

        // The audit key is protected by the master password (Argon2id). The file
        // holds the salt and a verify-token; the key itself is derived from the
        // password on each open. Without the password (or with a wrong one), the
        // key cannot be recovered → the audit is unavailable.
        let key = load_or_create_persistent_key(&dir, master_password.as_bytes())?;
        // open_path creates the SQLite connection internally: rusqlite stays an
        // implementation detail of core-storage, not a dependency of this crate.
        let log =
            AuditLog::open_path(&path, key).map_err(|e| AuditError::Storage(e.to_string()))?;
        Ok(Self {
            log: Mutex::new(log),
        })
    }

    /// Records the result of a bulk operation in the audit log.
    ///
    /// # Errors
    ///
    /// - [`AuditError`] — on a write error.
    pub fn append_op(
        &self,
        action: AuditAction,
        result: &BatchResult,
        timestamp: &str,
    ) -> Result<(), AuditError> {
        let payload = format!(
            "action={:?} succeeded={} failed={} failures={}",
            action.as_str(),
            result.succeeded,
            result.failed,
            result.failures.len()
        );
        self.append_payload(action, timestamp, payload.as_bytes())
    }

    /// Records an arbitrary action with a text payload (for export/settings,
    /// where there is no `BatchResult`). Does not forge "N succeeded" as before.
    ///
    /// # Errors
    ///
    /// - [`AuditError`] — on a write error.
    pub fn append_simple(
        &self,
        action: AuditAction,
        detail: &str,
        timestamp: &str,
    ) -> Result<(), AuditError> {
        self.append_payload(action, timestamp, detail.as_bytes())
    }

    /// Common helper: locks the mutex, appends a record, maps the error.
    /// The guard is released inside the block-scope (before `map_err`) so the lock
    /// is not held longer than necessary.
    fn append_payload(
        &self,
        action: AuditAction,
        timestamp: &str,
        payload: &[u8],
    ) -> Result<(), AuditError> {
        let outcome = {
            let mut log = self.log.lock().map_err(|_| AuditError::PoisonedLock)?;
            log.append(timestamp, action, payload)
        };
        outcome
            .map(|_| ())
            .map_err(|e| AuditError::Storage(e.to_string()))
    }

    /// Returns the last `limit` audit entries (newest first). Pushed down to
    /// SQL (`ORDER BY id DESC LIMIT ?`): the previous version materialized the
    /// whole table and re-sorted it in memory on every call — and this runs
    /// after every operation.
    ///
    /// # Errors
    ///
    /// - [`AuditError`] — on a read error.
    pub fn recent(&self, limit: usize) -> Result<Vec<AuditEntry>, AuditError> {
        // The audit table cannot outlive u32 rows (ids are SQLite integers);
        // saturate defensively rather than erroring on an absurd limit.
        let limit = u32::try_from(limit).unwrap_or(u32::MAX);
        let log = self.log.lock().map_err(|_| AuditError::PoisonedLock)?;
        log.recent(limit)
            .map_err(|e| AuditError::Storage(e.to_string()))
    }

    /// Verifies the integrity of the audit hash-chain.
    ///
    /// Distinguishes genuine tampering ([`AuditError::Tampered`]) from transient
    /// DB read errors ([`AuditError::Storage`]), so the UI does not falsely
    /// report "tampering" on any `SQLite` error.
    ///
    /// # Errors
    ///
    /// - [`AuditError::Tampered`] — the chain is broken (possible tampering
    ///   or corruption).
    /// - [`AuditError::Storage`] — a DB read error (not tampering).
    /// - [`AuditError::PoisonedLock`] — the mutex is poisoned.
    pub fn verify(&self) -> Result<(), AuditError> {
        let log = self.log.lock().map_err(|_| AuditError::PoisonedLock)?;
        log.verify().map_err(|e| match e {
            mailgrit_core_storage::StorageError::ChainBroken(sec) => {
                AuditError::Tampered(sec.to_string())
            }
            // Damage to the record structure (a hash-blob of wrong length) is
            // log tampering/damage, not a transient SQLite error.
            mailgrit_core_storage::StorageError::CorruptedEntry {
                id,
                expected,
                actual,
            } => AuditError::Tampered(format!(
                "record #{id} is damaged: hash-blob of length {actual} bytes (expected {expected})"
            )),
            mailgrit_core_storage::StorageError::Sqlite(sql) => {
                AuditError::Storage(sql.to_string())
            }
        })
    }
}

/// Loads the master-password-protected audit HMAC key, or creates a new one (on
/// the first run).
///
/// Format of the `.mailgrit-audit-key` file: `salt(16) || verify_token(32)`.
/// The key is derived from the master password and the salt via Argon2id
/// (`derive_key`). `verify_token = HMAC-SHA256(derived_key, CONST_TAG)` — lets
/// you check the master password without storing the key itself. The key itself
/// is NOT stored in the file.
///
/// On the first run (no file), a salt is generated, the key is derived from the
/// password, a verify-token is computed, and everything is saved. On subsequent
/// runs, the password is verified against the token; on a mismatch,
/// [`AuditError::WrongMasterPassword`] is returned.
///
/// The tag for the verify-token: distinguishes the HMAC purpose (do not confuse
/// with the audit hash-chain).
const VERIFY_TAG: &[u8] = b"mailgrit-audit-key-v1";
/// The expected key-file length: salt (16) + verify-token (32).
const AUDIT_KEY_FILE_LEN: usize =
    mailgrit_core_security::SALT_LEN + mailgrit_core_security::HMAC_LEN;

/// Checks whether an audit key file of the correct length exists.
/// Used by the UI to choose the mode (creation vs. master-password unlock).
#[must_use]
pub fn audit_key_file_is_valid() -> bool {
    let key_path = crate::app_data_dir().join(".mailgrit-audit-key");
    std::fs::metadata(&key_path)
        .is_ok_and(|m| m.len() == u64::try_from(AUDIT_KEY_FILE_LEN).unwrap_or(0))
}

fn load_or_create_persistent_key(
    dir: &Path,
    master_password: &[u8],
) -> Result<EncryptionKey, AuditError> {
    let key_path = dir.join(".mailgrit-audit-key");

    // Read first, decide from the result — the old `exists()` probe had a
    // TOCTOU gap (the file could appear/change between the check and the read).
    match std::fs::read(&key_path) {
        Ok(data) => validate_key_file(&data, master_password),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let key = create_new_key(&key_path, master_password, VERIFY_TAG)?;
            // First-run race guard: a second instance may have written ITS key
            // between our NotFound and our rename. Re-read and validate against
            // what is actually on disk — with the same master password this
            // converges to the winner's file; with a different one it correctly
            // reports WrongMasterPassword. (Residual window between this re-read
            // and the other instance's rename would require file locking, which
            // the std API does not expose; a single instance per data directory
            // is the supported configuration.)
            std::fs::read(&key_path).map_or_else(
                |_| Ok(key),
                |data| validate_key_file(&data, master_password),
            )
        }
        Err(e) => Err(AuditError::Storage(format!("reading the audit key: {e}"))),
    }
}

/// Validates the key file (`salt || verify_token`) against the password and
/// derives the audit key. A wrong file length is damage, not a wrong password.
fn validate_key_file(data: &[u8], master_password: &[u8]) -> Result<EncryptionKey, AuditError> {
    if data.len() != AUDIT_KEY_FILE_LEN {
        // The file exists but has the wrong length → damage (truncation, a
        // bad sector, third-party editing). Silently recreating the key would
        // make the entire legitimate audit history indistinguishable from a
        // forgery (verify() would report Tampered for every old record).
        // Report it as an error distinct from log tampering and from a wrong
        // password.
        tracing::error!(
            "audit key file is damaged: {} bytes (expected {AUDIT_KEY_FILE_LEN})",
            data.len()
        );
        return Err(AuditError::CorruptedKeyFile { actual: data.len() });
    }
    let (salt, stored_token) = data.split_at(mailgrit_core_security::SALT_LEN);
    let derived = mailgrit_core_security::derive_key(master_password, salt)
        .map_err(|e| AuditError::Kdf(e.to_string()))?;
    let derived_key = EncryptionKey::from_bytes(derived.as_slice())
        .map_err(|e| AuditError::Crypto(e.to_string()))?;
    // Verify-token: an HMAC with the derived key over the tag. Compared with
    // the stored one constant-time: the token is cryptographic; a classic
    // `!=` would reveal the position of the first mismatch via timing (a
    // weak but real channel).
    let expected_token = compute_verify_token(&derived_key, VERIFY_TAG)?;
    if !mailgrit_core_security::constant_time_eq(&expected_token, stored_token) {
        return Err(AuditError::WrongMasterPassword);
    }
    Ok(derived_key)
}

/// Creates a new protected audit key (first run): salt + key + token.
fn create_new_key(
    key_path: &Path,
    master_password: &[u8],
    verify_tag: &[u8],
) -> Result<EncryptionKey, AuditError> {
    let salt = mailgrit_core_security::generate_salt();
    let derived = mailgrit_core_security::derive_key(master_password, &salt)
        .map_err(|e| AuditError::Kdf(e.to_string()))?;
    let derived_key = EncryptionKey::from_bytes(derived.as_slice())
        .map_err(|e| AuditError::Crypto(e.to_string()))?;
    let verify_token = compute_verify_token(&derived_key, verify_tag)?;
    // Assemble the file: salt || verify_token.
    let mut file_data = Vec::with_capacity(salt.len().saturating_add(verify_token.len()));
    file_data.extend_from_slice(&salt);
    file_data.extend_from_slice(&verify_token);
    // Atomic write (temp + fsync + rename): a crash mid-write must never
    // leave a truncated key file — that would brick the whole audit history
    // (every future open reports CorruptedKeyFile). 0600 on Unix (secret).
    crate::fs_util::atomic_write(key_path, &file_data)
        .map_err(|e| AuditError::Storage(format!("writing the audit key: {e}")))?;
    Ok(derived_key)
}

/// Computes the verify-token = HMAC-SHA256(key, tag) via the domain hash-chain
/// API.
fn compute_verify_token(key: &EncryptionKey, tag: &[u8]) -> Result<[u8; 32], AuditError> {
    mailgrit_core_security::chain_hash(key, &mailgrit_core_security::GENESIS_HASH, tag)
        .map_err(|e| AuditError::Crypto(e.to_string()))
}

#[cfg(test)]
#[path = "audit_ui_tests.rs"]
mod tests;
