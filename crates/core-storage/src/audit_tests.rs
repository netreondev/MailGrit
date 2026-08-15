//! Audit-log tests ([`audit`](../audit.rs)).
//!
//! Factored into a separate file (via `#[path]`) to keep the main module within
//! the ≤400-line file-size limit. Included as the body of `mod tests`.
//!
//! Every test below drives a real in-memory SQLite database via `rusqlite`,
//! which calls into libsqlite3 through C FFI. Miri cannot interpret foreign
//! functions, so these tests are skipped under `cfg(miri)` (Miri reports
//! "unsupported operation: can't call foreign function `sqlite3_threadsafe`").
//! The audit-log *logic* — the hash-chain (HMAC-SHA256) computation, tamper
//! detection, and entry framing — lives in `core-security::hashchain` and is
//! fully covered by Miri there (no FFI); `core-storage` is the SQLite
//! persistence layer on top.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;
// Connection/params are available via the glob import `use super::*` (audit.rs
// imports `rusqlite::{Connection, params}` at module level).

fn in_memory_log() -> Result<AuditLog, Box<dyn std::error::Error>> {
    let conn = Connection::open_in_memory()?;
    let key = EncryptionKey::generate();
    Ok(AuditLog::open(conn, key)?)
}

#[test]
#[cfg_attr(miri, ignore)]
fn append_and_verify_clean_chain() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append(
        "2026-07-30T10:00:00Z",
        AuditAction::CreateUser,
        b"create u1",
    )?;
    log.append("2026-07-30T10:01:00Z", AuditAction::EditUser, b"edit u1")?;
    log.append(
        "2026-07-30T10:02:00Z",
        AuditAction::DeleteUser,
        b"delete u1",
    )?;
    assert!(log.verify().is_ok());
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn verify_empty_chain() -> Result<(), Box<dyn std::error::Error>> {
    let log = in_memory_log()?;
    assert!(log.verify().is_ok(), "empty chain is valid");
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn detect_tampered_payload() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append("2026-07-30T10:00:00Z", AuditAction::CreateUser, b"original")?;
    // Tamper with the payload directly in the DB (simulating a break-in).
    log.conn.execute(
        "UPDATE audit_log SET payload = ?1 WHERE id = 1",
        params![b"tampered"],
    )?;
    assert!(matches!(
        log.verify(),
        Err(StorageError::ChainBroken { .. })
    ));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn detect_deleted_entry() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append("t1", AuditAction::CreateUser, b"e1")?;
    log.append("t2", AuditAction::CreateUser, b"e2")?;
    log.append("t3", AuditAction::CreateUser, b"e3")?;
    log.conn.execute("DELETE FROM audit_log WHERE id = 2", [])?;
    assert!(matches!(
        log.verify(),
        Err(StorageError::ChainBroken { .. })
    ));
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn different_keys_produce_different_chains() -> Result<(), Box<dyn std::error::Error>> {
    let conn1 = Connection::open_in_memory()?;
    let conn2 = Connection::open_in_memory()?;
    let key1 = EncryptionKey::generate();
    let key2 = EncryptionKey::generate();
    let mut log1 = AuditLog::open(conn1, key1)?;
    let mut log2 = AuditLog::open(conn2, key2)?;

    log1.append("t", AuditAction::CreateUser, b"same-payload")?;
    log2.append("t", AuditAction::CreateUser, b"same-payload")?;

    let entries1 = log1.entries()?;
    let entries2 = log2.entries()?;
    assert_ne!(
        entries1.first().ok_or("missing entry 1")?.hash,
        entries2.first().ok_or("missing entry 2")?.hash
    );
    Ok(())
}

#[test]
#[cfg_attr(miri, ignore)]
fn append_returns_monotonic_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    let id1 = log.append("t1", AuditAction::CreateUser, b"e1")?;
    let id2 = log.append("t2", AuditAction::CreateUser, b"e2")?;
    assert!(id2 > id1, "IDs increase monotonically");
    Ok(())
}

// recent(limit) returns the NEWEST `limit` entries in descending-id order —
// the UI contract (previously emulated by sorting the full table in memory).
#[test]
#[cfg_attr(miri, ignore)]
fn recent_returns_newest_first_and_respects_limit() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    for i in 0..5 {
        log.append("t", AuditAction::CreateUser, format!("e{i}").as_bytes())?;
    }
    let recent = log.recent(3)?;
    assert_eq!(recent.len(), 3, "LIMIT is pushed down to SQL");
    let ids: Vec<i64> = recent.iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![5, 4, 3], "newest first, ascending ids reversed");
    // limit larger than the table → everything, still newest first.
    let all = log.recent(100)?;
    assert_eq!(all.len(), 5);
    assert_eq!(all.first().ok_or("empty")?.id, 5);
    Ok(())
}

// The path-based constructor (used by app-desktop) creates the DB file and
// keeps the same chain semantics as the in-memory constructor.
#[test]
#[cfg_attr(miri, ignore)]
fn open_path_creates_file_and_appends() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!(
        "mailgrit-storage-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir)?;
    let result = (|| {
        let key = mailgrit_core_security::EncryptionKey::generate();
        let db = dir.join("audit.sqlite");
        let mut log = AuditLog::open_path(&db, key)?;
        log.append("t", AuditAction::Export, b"via-open-path")?;
        assert!(log.verify().is_ok());
        assert_eq!(log.recent(10)?.len(), 1);
        Ok::<(), Box<dyn std::error::Error>>(())
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

// All AuditAction variants have correct string representations (DB contract).
#[test]
#[cfg_attr(miri, ignore)]
fn action_strings() {
    assert_eq!(AuditAction::CreateUser.as_str(), "CREATE_USER");
    assert_eq!(AuditAction::EditUser.as_str(), "EDIT_USER");
    assert_eq!(AuditAction::DeleteUser.as_str(), "DELETE_USER");
    assert_eq!(AuditAction::Export.as_str(), "EXPORT");
    assert_eq!(AuditAction::CreateDomain.as_str(), "CREATE_DOMAIN");
    assert_eq!(AuditAction::EditDomain.as_str(), "EDIT_DOMAIN");
    assert_eq!(AuditAction::DeleteDomain.as_str(), "DELETE_DOMAIN");
    assert_eq!(AuditAction::CreateAdmin.as_str(), "CREATE_ADMIN");
    assert_eq!(AuditAction::EditAdmin.as_str(), "EDIT_ADMIN");
    assert_eq!(AuditAction::DeleteAdmin.as_str(), "DELETE_ADMIN");
}

#[test]
#[cfg_attr(miri, ignore)]
fn action_strings_are_distinct() {
    let all = [
        AuditAction::CreateUser.as_str(),
        AuditAction::EditUser.as_str(),
        AuditAction::DeleteUser.as_str(),
        AuditAction::Export.as_str(),
        AuditAction::CreateDomain.as_str(),
        AuditAction::EditDomain.as_str(),
        AuditAction::DeleteDomain.as_str(),
        AuditAction::CreateAdmin.as_str(),
        AuditAction::EditAdmin.as_str(),
        AuditAction::DeleteAdmin.as_str(),
    ];
    let unique: std::collections::HashSet<&str> = all.iter().copied().collect();
    assert_eq!(unique.len(), all.len(), "duplicate AuditAction strings");
}

#[test]
#[cfg_attr(miri, ignore)]
fn append_domain_admin_actions_keeps_chain_consistent() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append("t", AuditAction::CreateDomain, b"create-domain")?;
    log.append("t", AuditAction::EditAdmin, b"edit-admin")?;
    log.append("t", AuditAction::DeleteDomain, b"delete-domain")?;
    assert!(log.verify().is_ok(), "chain is valid");
    let entries = log.entries()?;
    assert_eq!(entries.len(), 3);
    assert_eq!(
        entries.first().ok_or("missing entry 0")?.action,
        "CREATE_DOMAIN"
    );
    assert_eq!(
        entries.get(1).ok_or("missing entry 1")?.action,
        "EDIT_ADMIN"
    );
    assert_eq!(
        entries.get(2).ok_or("missing entry 2")?.action,
        "DELETE_DOMAIN"
    );
    Ok(())
}

// Corruption of the hash-blob length (truncation) was previously silently
// replaced with zeros, so verify() incorrectly pointed at the NEXT entry rather
// than the corrupted one. Now a CorruptedEntry is returned with the id of the
// corrupted entry itself.
#[test]
#[cfg_attr(miri, ignore)]
fn corrupted_hash_length_is_reported_not_silenced() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append("t1", AuditAction::CreateUser, b"e1")?;
    log.append("t2", AuditAction::CreateUser, b"e2")?;
    // Truncate the hash of the second entry to 10 bytes (HMAC_LEN = 32).
    log.conn.execute(
        "UPDATE audit_log SET hash = ?1 WHERE id = 2",
        params![b"0123456789".as_slice()],
    )?;
    // entries() must point at the corrupted entry (id=2), not stay silent.
    match log.entries() {
        Err(StorageError::CorruptedEntry {
            id,
            expected,
            actual,
        }) => {
            assert_eq!(id, 2, "the corrupted entry is exactly id=2");
            assert_eq!(actual, 10);
            assert_eq!(expected, HMAC_LEN);
        }
        Ok(_) => return Err("expected CorruptedEntry, got Ok".into()),
        Err(other) => return Err(format!("expected CorruptedEntry for id=2, got {other:?}").into()),
    }
    // verify() also fails (via entries), rather than reporting success.
    assert!(
        matches!(
            log.verify(),
            Err(StorageError::CorruptedEntry { id: 2, .. })
        ),
        "verify must point at the corrupted entry id=2"
    );
    Ok(())
}

// last_hash (used during append) also diagnoses corruption of the chain tail
// instead of silently substituting zeros — otherwise the new entry would be
// built from a bogus prev_hash and break the chain further with no signal.
#[test]
#[cfg_attr(miri, ignore)]
fn last_hash_detects_corrupted_tail() -> Result<(), Box<dyn std::error::Error>> {
    let mut log = in_memory_log()?;
    log.append("t1", AuditAction::CreateUser, b"e1")?;
    // Corrupt the only entry (which is also the last one).
    log.conn.execute(
        "UPDATE audit_log SET hash = ?1 WHERE id = 1",
        params![b"short".as_slice()],
    )?;
    // The next append triggers last_hash → it must return an error, not zeros.
    let result = log.append("t2", AuditAction::CreateUser, b"e2");
    assert!(
        matches!(result, Err(StorageError::CorruptedEntry { id: 1, .. })),
        "append after tail corruption must fail with CorruptedEntry id=1"
    );
    Ok(())
}
