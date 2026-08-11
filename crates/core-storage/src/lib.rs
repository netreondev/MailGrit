//! `core-storage` — local storage (SQLite): operation journal and audit log.
//! Integrity protection: a hash-chained audit log — each entry contains
//! `HMAC-SHA256(message ‖ prev_hash, key)`; any tampering with or deletion of a
//! row breaks the chain on verification. Access: a single exclusive writer.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny) is set
// centrally in [workspace.lints.rust] of the root Cargo.toml. Test modules
// follow the same policy (no test-only suppressions).

pub mod audit;
pub mod error;

pub use audit::{AuditAction, AuditEntry, AuditLog};
pub use error::{StorageError, StorageResult};
