//! Declarative operation profile: the set of fields for a
//! ([`OperationTarget`], [`BulkOperationKind`]) pair.
//!
//! A pure description of the field schema (canonical name, requiredness, default,
//! length limit) — with no value logic or validation. Provides `core-csv` with the
//! data to auto-detect column mapping.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::limits::{
    DEFAULT_QUOTA_MB_STR, MAX_DISPLAY_NAME_LEN, MAX_DOMAIN_LEN, MAX_PASSWORD_LEN, MAX_USERNAME_LEN,
};
use crate::operation::{BulkOperationKind, OperationTarget};

/// Canonical names of the classic 5 CSV fields (in `RawCsvRow` order).
const FIELD_DOMAIN: &str = "domain";
const FIELD_USERNAME: &str = "username";
const FIELD_PASSWORD: &str = "password";
const FIELD_DISPLAY_NAME: &str = "display_name";
const FIELD_QUOTA_MB: &str = "quota_mb";

/// The classic 5-column CSV schema, in [`RawCsvRow`](crate::RawCsvRow) order.
///
/// Single source of truth: `core-csv` derives its header constant and mapping
/// names from here (previously the same array was duplicated across crates).
#[rustfmt::skip]
pub const CLASSICAL_FIELD_NAMES: [&str; crate::EXPECTED_CSV_COLUMNS] = [
    FIELD_DOMAIN,
    FIELD_USERNAME,
    FIELD_PASSWORD,
    FIELD_DISPLAY_NAME,
    FIELD_QUOTA_MB,
];

/// Description of a single operation field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    /// Canonical field name (matches a CSV column or a REST parameter key).
    pub name: &'static str,
    /// Whether the field is required (has no default and cannot be empty).
    pub required: bool,
    /// Default value when the column is absent or empty.
    pub default: Option<&'static str>,
    /// Optional value length limit (in characters).
    pub max_len: Option<usize>,
}

impl FieldSpec {
    /// Creates a required field with no default value.
    #[must_use]
    pub const fn required(name: &'static str, max_len: usize) -> Self {
        Self {
            name,
            required: true,
            default: None,
            max_len: Some(max_len),
        }
    }

    /// Creates an optional field with a default value and a length limit.
    #[must_use]
    pub const fn optional(name: &'static str, default: &'static str, max_len: usize) -> Self {
        Self {
            name,
            required: false,
            default: Some(default),
            max_len: Some(max_len),
        }
    }
}

/// Declarative operation profile: a list of [`FieldSpec`] for a (target, kind) pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationProfile {
    /// Operation target (User, Domain, ...).
    pub target: OperationTarget,
    /// Operation kind (Create, Edit, Delete, ...).
    pub kind: BulkOperationKind,
    /// Fields in canonical order (important to match `RawCsvRow::fields`).
    pub fields: Vec<FieldSpec>,
}

impl OperationProfile {
    /// User creation profile: the classic 5-column CSV schema.
    /// Field order matches [`RawCsvRow`](crate::RawCsvRow).
    #[must_use]
    pub fn for_user_create() -> Self {
        Self {
            target: OperationTarget::User,
            kind: BulkOperationKind::Create,
            fields: vec![
                FieldSpec::required(FIELD_DOMAIN, MAX_DOMAIN_LEN),
                FieldSpec::required(FIELD_USERNAME, MAX_USERNAME_LEN),
                FieldSpec::required(FIELD_PASSWORD, MAX_PASSWORD_LEN),
                FieldSpec::optional(FIELD_DISPLAY_NAME, "", MAX_DISPLAY_NAME_LEN),
                FieldSpec::optional(FIELD_QUOTA_MB, DEFAULT_QUOTA_MB_STR, 16),
            ],
        }
    }

    // `SanitizedUserRow` is reused as the universal row carrier for both
    // Domain and Admin: the typestate validates all 5 classic fields, so the
    // domain quota flows through the classic `quota_mb` field.

    const MAX_TRANSPORT_LEN: usize = 64;
    const MAX_BACKUPMX_LEN: usize = 1;

    /// Domain creation profile (OSE).
    #[must_use]
    pub fn for_domain_create() -> Self {
        Self {
            target: OperationTarget::Domain,
            kind: BulkOperationKind::Create,
            fields: vec![
                FieldSpec::required(FIELD_DOMAIN, MAX_DOMAIN_LEN),
                FieldSpec::optional(FIELD_QUOTA_MB, DEFAULT_QUOTA_MB_STR, 16),
                FieldSpec::optional("transport", "dovecot", Self::MAX_TRANSPORT_LEN),
                FieldSpec::optional("is_backupmx", "0", Self::MAX_BACKUPMX_LEN),
            ],
        }
    }

    /// Administrator creation profile (OSE). Email = `username@domain`.
    #[must_use]
    pub fn for_admin_create() -> Self {
        Self {
            target: OperationTarget::Admin,
            kind: BulkOperationKind::Create,
            fields: vec![
                FieldSpec::required(FIELD_DOMAIN, MAX_DOMAIN_LEN),
                FieldSpec::required(FIELD_USERNAME, MAX_USERNAME_LEN),
                FieldSpec::required(FIELD_PASSWORD, MAX_PASSWORD_LEN),
                FieldSpec::optional(FIELD_DISPLAY_NAME, "", MAX_DISPLAY_NAME_LEN),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_create_has_exactly_five_fields_in_canonical_order() {
        let p = OperationProfile::for_user_create();
        assert_eq!(p.target, OperationTarget::User);
        assert_eq!(p.kind, BulkOperationKind::Create);
        assert_eq!(p.fields.len(), 5, "create profile = exactly 5 fields");
        let names: Vec<&str> = p.fields.iter().map(|f| f.name).collect();
        assert_eq!(names, CLASSICAL_FIELD_NAMES);
    }

    // CLASSICAL_FIELD_NAMES is the cross-crate contract (core-csv header and
    // mapping): pin that it stays the exact 5 canonical names in order.
    #[test]
    fn classical_field_names_are_the_canonical_five() {
        assert_eq!(
            CLASSICAL_FIELD_NAMES,
            ["domain", "username", "password", "display_name", "quota_mb"]
        );
    }

    #[test]
    fn user_create_required_flags_are_correct() -> Result<(), Box<dyn std::error::Error>> {
        let p = OperationProfile::for_user_create();
        let fields = p.fields.as_slice();
        let f0 = fields.first().ok_or("domain missing")?;
        let f1 = fields.get(1).ok_or("username missing")?;
        let f2 = fields.get(2).ok_or("password missing")?;
        let f3 = fields.get(3).ok_or("display_name missing")?;
        let f4 = fields.get(4).ok_or("quota_mb missing")?;
        assert!(f0.required, "domain is required");
        assert!(f1.required, "username is required");
        assert!(f2.required, "password is required");
        assert!(!f3.required, "display_name is optional");
        assert!(!f4.required, "quota_mb is optional");
        Ok(())
    }

    #[test]
    fn user_create_defaults_are_sane() -> Result<(), Box<dyn std::error::Error>> {
        let p = OperationProfile::for_user_create();
        let fields = p.fields.as_slice();
        let f0 = fields.first().ok_or("domain missing")?;
        let f1 = fields.get(1).ok_or("username missing")?;
        let f2 = fields.get(2).ok_or("password missing")?;
        let f3 = fields.get(3).ok_or("display_name missing")?;
        let f4 = fields.get(4).ok_or("quota_mb missing")?;
        assert_eq!(f0.default, None, "domain has no default");
        assert_eq!(f1.default, None, "username has no default");
        assert_eq!(f2.default, None, "password has no default");
        assert_eq!(f3.default, Some(""), "display_name default = ''");
        assert_eq!(
            f4.default,
            Some(DEFAULT_QUOTA_MB_STR),
            "quota_mb default = DEFAULT_QUOTA_MB_STR"
        );
        Ok(())
    }

    #[test]
    fn user_create_max_lens_match_limits_module() -> Result<(), Box<dyn std::error::Error>> {
        let p = OperationProfile::for_user_create();
        let fields = p.fields.as_slice();
        let f0 = fields.first().ok_or("domain missing")?;
        let f1 = fields.get(1).ok_or("username missing")?;
        let f2 = fields.get(2).ok_or("password missing")?;
        let f3 = fields.get(3).ok_or("display_name missing")?;
        let f4 = fields.get(4).ok_or("quota_mb missing")?;
        assert_eq!(f0.max_len, Some(MAX_DOMAIN_LEN));
        assert_eq!(f1.max_len, Some(MAX_USERNAME_LEN));
        assert_eq!(f2.max_len, Some(MAX_PASSWORD_LEN));
        assert_eq!(f3.max_len, Some(MAX_DISPLAY_NAME_LEN));
        assert!(f4.max_len.is_some());
        Ok(())
    }

    #[test]
    fn fieldspec_constructors_set_attributes() {
        let r = FieldSpec::required("x", 10);
        assert!(r.required);
        assert_eq!(r.default, None);
        assert_eq!(r.max_len, Some(10));
        let o = FieldSpec::optional("y", "def", 20);
        assert!(!o.required);
        assert_eq!(o.default, Some("def"));
        assert_eq!(o.max_len, Some(20));
    }

    fn names_of(p: &OperationProfile) -> Vec<&str> {
        p.fields.iter().map(|f| f.name).collect()
    }

    #[test]
    fn domain_create_has_correct_fields_and_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let p = OperationProfile::for_domain_create();
        assert_eq!(p.target, OperationTarget::Domain);
        assert_eq!(p.kind, BulkOperationKind::Create);
        assert_eq!(
            names_of(&p),
            vec!["domain", "quota_mb", "transport", "is_backupmx"]
        );
        let fields = p.fields.as_slice();
        assert!(
            fields.first().ok_or("domain missing")?.required,
            "domain is required"
        );
        assert_eq!(
            fields.get(1).ok_or("quota_mb missing")?.default,
            Some(DEFAULT_QUOTA_MB_STR)
        );
        assert_eq!(
            fields.get(2).ok_or("transport missing")?.default,
            Some("dovecot")
        );
        assert_eq!(
            fields.get(3).ok_or("is_backupmx missing")?.default,
            Some("0")
        );
        Ok(())
    }

    #[test]
    fn admin_create_has_correct_fields() -> Result<(), Box<dyn std::error::Error>> {
        let p = OperationProfile::for_admin_create();
        assert_eq!(p.target, OperationTarget::Admin);
        assert_eq!(p.kind, BulkOperationKind::Create);
        assert_eq!(
            names_of(&p),
            vec!["domain", "username", "password", "display_name"]
        );
        let fields = p.fields.as_slice();
        let f0 = fields.first().ok_or("domain missing")?;
        let f1 = fields.get(1).ok_or("username missing")?;
        let f2 = fields.get(2).ok_or("password missing")?;
        let f3 = fields.get(3).ok_or("display_name missing")?;
        assert!(f0.required && f1.required && f2.required);
        assert!(!f3.required);
        assert_eq!(f3.default, Some(""));
        Ok(())
    }
}
