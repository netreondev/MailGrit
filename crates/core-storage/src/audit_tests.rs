//! Audit-log tests ([`audit`](../audit.rs)).
//!
//! Factored into a separate file (via `#[path]`) to keep the main module within
//! the ≤400-line file-size limit. Included as the body of `mod tests`.

#![allow(clippy::indexing_slicing)] // tests intentionally use indexing

use super::*;
// Connection/params are available via the glob import `use super::*` (audit.rs
// imports `rusqlite::{Connection, params}` at module level).

fn in_memory_log() -> AuditLog {
    let conn = Connection::open_in_memory().unwrap();
    let key = EncryptionKey::generate();
    AuditLog::open(conn, key).unwrap()
}

#[test]
fn append_and_verify_clean_chain() {
    let mut log = in_memory_log();
    log.append(
        "2026-07-30T10:00:00Z",
        AuditAction::CreateUser,
        b"create u1",
    )
    .unwrap();
    log.append("2026-07-30T10:01:00Z", AuditAction::EditUser, b"edit u1")
        .unwrap();
    log.append(
        "2026-07-30T10:02:00Z",
        AuditAction::DeleteUser,
        b"delete u1",
    )
    .unwrap();
    assert!(log.verify().is_ok());
}

#[test]
fn verify_empty_chain() {
    let log = in_memory_log();
    assert!(log.verify().is_ok(), "empty chain is valid");
}

#[test]
fn detect_tampered_payload() {
    let mut log = in_memory_log();
    log.append("2026-07-30T10:00:00Z", AuditAction::CreateUser, b"original")
        .unwrap();
    // Tamper with the payload directly in the DB (simulating a break-in).
    log.conn
        .execute(
            "UPDATE audit_log SET payload = ?1 WHERE id = 1",
            params![b"tampered"],
        )
        .unwrap();
    assert!(matches!(
        log.verify(),
        Err(StorageError::ChainBroken { .. })
    ));
}

#[test]
fn detect_deleted_entry() {
    let mut log = in_memory_log();
    log.append("t1", AuditAction::CreateUser, b"e1").unwrap();
    log.append("t2", AuditAction::CreateUser, b"e2").unwrap();
    log.append("t3", AuditAction::CreateUser, b"e3").unwrap();
    log.conn
        .execute("DELETE FROM audit_log WHERE id = 2", [])
        .unwrap();
    assert!(matches!(
        log.verify(),
        Err(StorageError::ChainBroken { .. })
    ));
}

#[test]
fn different_keys_produce_different_chains() {
    let conn1 = Connection::open_in_memory().unwrap();
    let conn2 = Connection::open_in_memory().unwrap();
    let key1 = EncryptionKey::generate();
    let key2 = EncryptionKey::generate();
    let mut log1 = AuditLog::open(conn1, key1).unwrap();
    let mut log2 = AuditLog::open(conn2, key2).unwrap();

    log1.append("t", AuditAction::CreateUser, b"same-payload")
        .unwrap();
    log2.append("t", AuditAction::CreateUser, b"same-payload")
        .unwrap();

    let entries1 = log1.entries().unwrap();
    let entries2 = log2.entries().unwrap();
    assert_ne!(entries1[0].hash, entries2[0].hash);
}

#[test]
fn append_returns_monotonic_ids() {
    let mut log = in_memory_log();
    let id1 = log.append("t1", AuditAction::CreateUser, b"e1").unwrap();
    let id2 = log.append("t2", AuditAction::CreateUser, b"e2").unwrap();
    assert!(id2 > id1, "IDs increase monotonically");
}

// All AuditAction variants have correct string representations (DB contract).
#[test]
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
fn append_domain_admin_actions_keeps_chain_consistent() {
    let mut log = in_memory_log();
    log.append("t", AuditAction::CreateDomain, b"create-domain")
        .unwrap();
    log.append("t", AuditAction::EditAdmin, b"edit-admin")
        .unwrap();
    log.append("t", AuditAction::DeleteDomain, b"delete-domain")
        .unwrap();
    assert!(log.verify().is_ok(), "chain is valid");
    let entries = log.entries().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].action, "CREATE_DOMAIN");
    assert_eq!(entries[1].action, "EDIT_ADMIN");
    assert_eq!(entries[2].action, "DELETE_DOMAIN");
}

// Corruption of the hash-blob length (truncation) was previously silently
// replaced with zeros, so verify() incorrectly pointed at the NEXT entry rather
// than the corrupted one. Now a CorruptedEntry is returned with the id of the
// corrupted entry itself.
#[test]
fn corrupted_hash_length_is_reported_not_silenced() {
    let mut log = in_memory_log();
    log.append("t1", AuditAction::CreateUser, b"e1").unwrap();
    log.append("t2", AuditAction::CreateUser, b"e2").unwrap();
    // Truncate the hash of the second entry to 10 bytes (HMAC_LEN = 32).
    log.conn
        .execute(
            "UPDATE audit_log SET hash = ?1 WHERE id = 2",
            params![b"0123456789".as_slice()],
        )
        .unwrap();
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
        other => panic!("expected CorruptedEntry for id=2, got {other:?}"),
    }
    // verify() also fails (via entries), rather than reporting success.
    assert!(
        matches!(
            log.verify(),
            Err(StorageError::CorruptedEntry { id: 2, .. })
        ),
        "verify must point at the corrupted entry id=2"
    );
}

// last_hash (used during append) also diagnoses corruption of the chain tail
// instead of silently substituting zeros — otherwise the new entry would be
// built from a bogus prev_hash and break the chain further with no signal.
#[test]
fn last_hash_detects_corrupted_tail() {
    let mut log = in_memory_log();
    log.append("t1", AuditAction::CreateUser, b"e1").unwrap();
    // Corrupt the only entry (which is also the last one).
    log.conn
        .execute(
            "UPDATE audit_log SET hash = ?1 WHERE id = 1",
            params![b"short".as_slice()],
        )
        .unwrap();
    // The next append triggers last_hash → it must return an error, not zeros.
    let result = log.append("t2", AuditAction::CreateUser, b"e2");
    assert!(
        matches!(result, Err(StorageError::CorruptedEntry { id: 1, .. })),
        "append after tail corruption must fail with CorruptedEntry id=1"
    );
}
