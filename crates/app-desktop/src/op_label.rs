//! Single source of UI labels for the (target x operation-kind) pair.
//!
//! core-domain stores the technical `as_str` ("CREATE"/"EDIT"/...) for the audit
//! log, while here live the human-readable localized labels for the UI. This is
//! the single UI-label source for the whole pipeline (logs, badges, headings),
//! preventing label drift across modules.
//!
//! Labels are taken from the translation catalog (`locales/app.<lang>.yml`, keys
//! `op_label.<target>.<kind>`), so they change together with the UI language.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use mailgrit_core_domain::{BulkOperationKind, OperationTarget};

/// Returns the translation key for the (target, operation-kind) pair.
///
/// Full Cartesian table of 3 targets x 3 kinds = 9 cells. All cells are filled
/// explicitly (parse, don't validate): adding a new target/kind will require
/// explicitly filling all pairs, ruling out a "forgotten" label (the compiler
/// won't let the enum build without an exhaustive match).
#[must_use]
pub const fn operation_label_key(target: OperationTarget, kind: BulkOperationKind) -> &'static str {
    match (target, kind) {
        // —— User ———————————————————————————————————————————————
        (OperationTarget::User, BulkOperationKind::Create) => "op_label.user.create",
        (OperationTarget::User, BulkOperationKind::Edit) => "op_label.user.edit",
        (OperationTarget::User, BulkOperationKind::Delete) => "op_label.user.delete",
        // —— Domain ——————————————————————————————————————————————
        (OperationTarget::Domain, BulkOperationKind::Create) => "op_label.domain.create",
        (OperationTarget::Domain, BulkOperationKind::Edit) => "op_label.domain.edit",
        (OperationTarget::Domain, BulkOperationKind::Delete) => "op_label.domain.delete",
        // —— Admin ——————————————————————————————————————————————
        (OperationTarget::Admin, BulkOperationKind::Create) => "op_label.admin.create",
        (OperationTarget::Admin, BulkOperationKind::Edit) => "op_label.admin.edit",
        (OperationTarget::Admin, BulkOperationKind::Delete) => "op_label.admin.delete",
    }
}

/// Returns the localized UI label for the (target, operation-kind) pair in the
/// current language. Not `const` because it reads the global `rust_i18n` locale.
#[must_use]
pub fn operation_label(target: OperationTarget, kind: BulkOperationKind) -> String {
    t!(operation_label_key(target, kind)).to_string()
}

#[cfg(test)]
mod tests {

    use super::*;

    // User-target labels in Ukrainian (a non-fallback locale).
    #[test]
    fn user_labels_uk() {
        rust_i18n::set_locale("uk");
        assert_eq!(
            operation_label(OperationTarget::User, BulkOperationKind::Create),
            "Створення користувача"
        );
        assert_eq!(
            operation_label(OperationTarget::User, BulkOperationKind::Edit),
            "Редагування користувача"
        );
        assert_eq!(
            operation_label(OperationTarget::User, BulkOperationKind::Delete),
            "Видалення користувача"
        );
        // Restore the global locale to the default.
        rust_i18n::set_locale("en");
    }

    // Labels for non-user targets in Ukrainian.
    #[test]
    fn non_user_target_labels_uk() {
        rust_i18n::set_locale("uk");
        assert_eq!(
            operation_label(OperationTarget::Domain, BulkOperationKind::Create),
            "Створення домену"
        );
        assert_eq!(
            operation_label(OperationTarget::Admin, BulkOperationKind::Delete),
            "Видалення адміністратора"
        );
        rust_i18n::set_locale("en");
    }

    // Table completeness: no (target x kind) pair may return a duplicate of
    // another pair for the same target (the action must be distinguishable).
    #[test]
    fn all_kinds_distinct_for_user() {
        let labels: Vec<String> = vec![
            operation_label(OperationTarget::User, BulkOperationKind::Create),
            operation_label(OperationTarget::User, BulkOperationKind::Edit),
            operation_label(OperationTarget::User, BulkOperationKind::Delete),
        ];
        let unique = {
            let mut v = labels.clone();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        assert_eq!(unique, labels.len(), "duplicate UI labels for User");
    }

    // The table keys cover all 9 pairs (compile-time exhaustiveness via a const fn match).
    #[test]
    fn all_nine_pairs_have_keys() {
        for target in [
            OperationTarget::User,
            OperationTarget::Domain,
            OperationTarget::Admin,
        ] {
            for kind in [
                BulkOperationKind::Create,
                BulkOperationKind::Edit,
                BulkOperationKind::Delete,
            ] {
                let key = operation_label_key(target, kind);
                assert!(!key.is_empty());
            }
        }
    }
}
