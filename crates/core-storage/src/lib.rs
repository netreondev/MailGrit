//! `core-storage` — local storage (SQLite): operation journal and audit log.
//! Integrity protection: a hash-chained audit log — each entry contains
//! `HMAC-SHA256(message ‖ prev_hash, key)`; any tampering with or deletion of a
//! row breaks the chain on verification. Access: a single exclusive writer.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// In tests, unwrap/panic are permitted (a test failure is an intentional panic).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic
    )
)]

pub mod audit;
pub mod error;

pub use audit::{AuditAction, AuditEntry, AuditLog};
pub use error::{StorageError, StorageResult};
