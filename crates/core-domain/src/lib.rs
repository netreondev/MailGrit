//! `core-domain` — domain types, errors, and limit constants. Newtype/Typestate
//! wrappers that cannot be constructed except through the canonical parser-sanitizer.
//! The crate has no network/system dependencies (only `thiserror` and `rand`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// In tests, unwrap/panic are allowed (a failing test is an intentional panic).
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

/// Crate version (for smoke tests and diagnostics).
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

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

pub use editable::{EditableField, EditableFieldError, EditableUserRow};
pub use error::{
    CsvRowError, DisplayNameError, DomainError, PasswordError, QuotaError, UsernameError,
};
pub use limits::*;
pub use operation::{BulkOperationKind, OperationTarget};
pub use password_gen::PasswordGenerator;
pub use password_policy::{PasswordPolicy, PasswordWarning};
pub use profile::{FieldSpec, OperationProfile};
pub use types::{
    SanitizedDisplayName, SanitizedUsername, ValidatedDomain, ValidatedPassword, ValidatedQuota,
};
pub use typestate::{EXPECTED_CSV_COLUMNS, RawCsvRow, Sanitized, SanitizedUserRow, Unverified};
