//! `core-domain` — domain types, errors, and limit constants. Newtype/Typestate
//! wrappers that cannot be constructed except through the canonical parser-sanitizer.
//! The crate has no network/system dependencies (only `thiserror` and `rand`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

#![forbid(unsafe_code)]
// Lint policy (missing_docs/dead_code/unused/rust_2018_idioms deny) is set
// centrally in [workspace.lints.rust] of the root Cargo.toml. Test modules
// follow the same policy (no test-only suppressions).

/// Crate version (for smoke tests and diagnostics).
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod char_classes;
pub mod editable;
pub mod error;
pub mod limits;
pub mod operation;
pub mod password_gen;
pub mod password_policy;
pub mod profile;
pub mod types;
pub mod typestate;

/// Kani formal verification proof harnesses (only active under cfg(kani)).
#[cfg(kani)]
mod kani_harnesses;

pub use char_classes::CharacterClasses;
pub use editable::{EditableField, EditableFieldError, EditableUserRow};
pub use error::{
    CsvRowError, DisplayNameError, DomainError, PasswordError, QuotaError, UsernameError,
};
pub use limits::*;
pub use operation::{BulkOperationKind, OperationTarget};
pub use password_gen::PasswordGenerator;
pub use password_policy::{PasswordPolicy, PasswordWarning};
pub use profile::{CLASSICAL_FIELD_NAMES, FieldSpec, OperationProfile};
pub use types::{
    SanitizedDisplayName, SanitizedUsername, ValidatedDomain, ValidatedPassword, ValidatedQuota,
};
pub use typestate::{EXPECTED_CSV_COLUMNS, RawCsvRow, Sanitized, SanitizedUserRow, Unverified};
