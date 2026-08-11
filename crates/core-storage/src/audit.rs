//! Hash-chained audit log on top of SQLite.
//!
//! Each entry stores an action, a payload, and `HMAC-SHA256(payload ‖ prev_hash, key)`.
//! Integrity is checked by [`AuditLog::verify`]: any tampering breaks the chain.
//! H_n = HMAC(Message_n ‖ H_{n-1}, K).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use crate::error::StorageError;
use mailgrit_core_security::{EncryptionKey, GENESIS_HASH, HMAC_LEN, chain_hash, verify_chain};
use rusqlite::{Connection, params};

/// An action recorded in the audit (`action` TEXT — extensible without a migration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    /// User creation.
    CreateUser,
    /// User edit.
    EditUser,
    /// User deletion.
    DeleteUser,
    /// Data export.
    Export,
    /// Domain creation.
    CreateDomain,
    /// Domain edit.
    EditDomain,
    /// Domain deletion.
    DeleteDomain,
    /// Administrator creation.
    CreateAdmin,
    /// Administrator edit.
    EditAdmin,
    /// Administrator deletion.
    DeleteAdmin,
}

impl AuditAction {
    /// String representation for the DB.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateUser => "CREATE_USER",
            Self::EditUser => "EDIT_USER",
            Self::DeleteUser => "DELETE_USER",
            Self::Export => "EXPORT",
            Self::CreateDomain => "CREATE_DOMAIN",
            Self::EditDomain => "EDIT_DOMAIN",
            Self::DeleteDomain => "DELETE_DOMAIN",
            Self::CreateAdmin => "CREATE_ADMIN",
            Self::EditAdmin => "EDIT_ADMIN",
            Self::DeleteAdmin => "DELETE_ADMIN",
        }
    }
}

/// An audit-log entry (for reading/verification).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Entry ID.
    pub id: i64,
    /// Timestamp (RFC3339).
    pub timestamp: String,
    /// Action.
    pub action: String,
    /// Payload the HMAC is computed over.
    pub payload: Vec<u8>,
    /// Entry hash (32 bytes).
    pub hash: [u8; HMAC_LEN],
}

/// Audit log with a cryptographic hash chain.
pub struct AuditLog {
    /// SQLite connection (exclusive writer).
    conn: Connection,
    /// Master key for HMAC (zeroed on Drop).
    key: EncryptionKey,
}

impl AuditLog {
    /// Opens (or creates) the audit log; `key` must be the same for the whole chain.
    ///
    /// # Errors
    ///
    /// - [`StorageError::Sqlite`] — DB initialization error.
    pub fn open(conn: Connection, key: EncryptionKey) -> Result<Self, StorageError> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                action      TEXT NOT NULL,
                payload     BLOB NOT NULL,
                hash        BLOB NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn, key })
    }

    /// Appends an entry, computing the hash chain.
    ///
    /// # Errors
    ///
    /// - [`StorageError::Sqlite`] — write error.
    pub fn append(
        &mut self,
        timestamp: &str,
        action: AuditAction,
        payload: &[u8],
    ) -> Result<i64, StorageError> {
        let prev_hash = self.last_hash()?;

        let new_hash = chain_hash(&self.key, &prev_hash, payload)?;

        self.conn.execute(
            "INSERT INTO audit_log (timestamp, action, payload, hash) VALUES (?1, ?2, ?3, ?4)",
            params![timestamp, action.as_str(), payload, new_hash.as_slice()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Returns the hash of the last entry, or GENESIS if the chain is empty.
    ///
    /// A corrupted blob (length ≠ `HMAC_LEN`) → [`StorageError::CorruptedEntry`]:
    /// it is NOT silently replaced with zeros, or verification would incorrectly
    /// point at the next entry (fail loud, not silent).
    fn last_hash(&self) -> Result<[u8; HMAC_LEN], StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1")?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => {
                let id: i64 = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                hash_from_blob(id, &blob)
            }
            None => Ok(GENESIS_HASH),
        }
    }

    /// Verifies the integrity of the whole hash chain.
    ///
    /// # Errors
    ///
    /// - [`StorageError::ChainBroken`] — the chain is broken (the log was tampered with).
    /// - [`StorageError::CorruptedEntry`] — corrupted entry structure.
    /// - [`StorageError::Sqlite`] — read error.
    pub fn verify(&self) -> Result<(), StorageError> {
        let entries = self.entries()?;
        let chain: Vec<(Vec<u8>, [u8; HMAC_LEN])> =
            entries.into_iter().map(|e| (e.payload, e.hash)).collect();
        verify_chain(&self.key, chain).map_err(StorageError::from)
    }

    /// Returns all entries in ascending `id` order (for UI/export and verification).
    ///
    /// A corrupted hash blob → [`StorageError::CorruptedEntry`] (fail loud, not silent).
    ///
    /// # Errors
    ///
    /// - [`StorageError::Sqlite`] — DB read error.
    /// - [`StorageError::CorruptedEntry`] — corrupted entry structure.
    pub fn entries(&self) -> Result<Vec<AuditEntry>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, action, payload, hash FROM audit_log ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RawEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                action: row.get(2)?,
                payload: row.get(3)?,
                hash_blob: row.get(4)?,
            })
        })?;
        let mut entries = Vec::new();
        for row in rows {
            let raw = row?;
            let hash = hash_from_blob(raw.id, &raw.hash_blob)?;
            entries.push(AuditEntry {
                id: raw.id,
                timestamp: raw.timestamp,
                action: raw.action,
                payload: raw.payload,
                hash,
            });
        }
        Ok(entries)
    }
}

/// A raw entry from the DB before hash-blob length validation.
struct RawEntry {
    id: i64,
    timestamp: String,
    action: String,
    payload: Vec<u8>,
    hash_blob: Vec<u8>,
}

/// Converts a hash blob into a fixed-size array, validating its length.
///
/// A length mismatch → [`StorageError::CorruptedEntry`], not a silent
/// zero-substitution: corruption must be diagnosed explicitly (fail loud).
const fn hash_from_blob(id: i64, blob: &[u8]) -> Result<[u8; HMAC_LEN], StorageError> {
    if blob.len() == HMAC_LEN {
        let mut arr = [0u8; HMAC_LEN];
        arr.copy_from_slice(blob);
        Ok(arr)
    } else {
        Err(StorageError::CorruptedEntry {
            id,
            expected: HMAC_LEN,
            actual: blob.len(),
        })
    }
}

#[cfg(test)]
#[path = "audit_tests.rs"]
mod tests;
