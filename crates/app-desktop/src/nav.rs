//! Top-level panel navigation.
//!
//! MailGrit is focused on two sections: Operations (load CSV -> editable table
//! -> password generation -> execute -> result) and Audit (a hash-chained
//! operation log for accountability).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use crate::components::icon::Icon;
use crate::components::segmented::SegmentedOption;

/// Top-level panel section. Default `Operations` -> on entry the target picker
/// and the CSV/operations/result cards are immediately visible.
///
/// `Copy` + `PartialEq` so it can be used in `SegmentedControl`
/// (`Segmented<T: Clone + PartialEq + 'static>`) and stored in `AppState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DashboardSection {
    /// Targets (User/Domain/Admin), CSV, operations, result.
    #[default]
    Operations,
    /// Hash-chained audit log of operations.
    Audit,
}

impl DashboardSection {
    /// Section title translation key (the value is resolved via `t!` in the UI).
    /// Returns a key, not a ready string, since the language changes at runtime.
    #[must_use]
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::Operations => "nav.operations",
            Self::Audit => "nav.audit",
        }
    }

    /// Human-readable section title in the current language (for the card header).
    /// NOT `const`, since it reads the global `rust_i18n` locale.
    #[must_use]
    pub fn title(self) -> String {
        t!(self.title_key()).to_string()
    }

    /// Options for the top-level `SegmentedControl`. Labels come from the i18n
    /// dictionary (single source of truth). Returns a `Vec` (required by the
    /// `Segmented::options` signature).
    #[must_use]
    pub fn options() -> Vec<SegmentedOption<Self>> {
        vec![
            SegmentedOption {
                value: Self::Operations,
                label: t!("nav.operations").to_string(),
                icon: Some(Icon::Wrench),
            },
            SegmentedOption {
                value: Self::Audit,
                label: t!("nav.audit").to_string(),
                icon: Some(Icon::Check),
            },
        ]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    /// Default is Operations (the operations section opens on entry).
    #[test]
    fn default_is_operations() {
        let s = DashboardSection::default();
        assert_eq!(s, DashboardSection::Operations);
    }

    /// Each option has a non-empty label and a matching value/title.
    #[test]
    fn options_have_titles() {
        let opts = DashboardSection::options();
        assert_eq!(opts.len(), 2);
        for opt in &opts {
            assert!(!opt.label.is_empty());
            assert_eq!(opt.label, opt.value.title());
        }
    }

    /// All variants are represented.
    #[test]
    fn all_sections_present_in_options() {
        let opts = DashboardSection::options();
        let values: Vec<_> = opts.into_iter().map(|o| o.value).collect();
        assert!(values.contains(&DashboardSection::Operations));
        assert!(values.contains(&DashboardSection::Audit));
    }
}
