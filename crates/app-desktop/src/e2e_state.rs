//! E2E hook to start directly in the dashboard state (bypassing the iRedAdmin login flow).
//!
//! Compiled ONLY with the `e2e` cargo feature (test builds; the CI e2e job
//! builds with `--features e2e`) AND activated only by the
//! `MAILGRIT_E2E_DASHBOARD` environment variable — release binaries do not
//! contain this module at all. Purpose: Playwright/CDP E2E tests cannot flip
//! the Dioxus `Signal<AppState>` to `Screen::Dashboard` from the outside (the
//! state lives in the Rust runtime, unreachable from JS/CDP), and a real
//! iRedAdmin login requires a live server. So when the flag is set, the
//! application starts directly on the dashboard with pre-filled valid test
//! table rows — this lets E2E evaluate the UI/UX of every screen (modals,
//! table, password controls, theme, i18n, a11y) without a network round-trip.
//!
//! Reuses the canonical CSV parser (`parse_csv_bytes_auto` + `detect_mapping`)
//! so the pre-filled data goes through the same sanitization/validation as
//! user data — with no separate test branch of the logic.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::state::{AppState, AuthStatus, Screen};
use dioxus::prelude::*;
use mailgrit_core_csv::{detect_mapping, parse_csv_bytes_auto};
use mailgrit_core_domain::EditableUserRow;
use std::sync::Arc;

/// Name of the env variable that activates starting in the dashboard (E2E mode).
pub const E2E_DASHBOARD_ENV: &str = "MAILGRIT_E2E_DASHBOARD";

/// Embedded test CSV: 2 valid rows for the `for_user_create` profile.
/// Fields in canonical order: `domain,username,password,display_name,quota_mb`.
/// The rows are valid per the typestate pipeline rules (see core-domain/typestate),
/// so the table renders without error highlighting — E2E can enter invalid
/// values itself to test validation.
const E2E_TEST_CSV: &str = "\
domain,username,password,display_name,quota_mb
example.com,alice.test,Str0ng!Pass1,Alice Test,1024
example.com,bob.demo,Str0ng!Pass2,Bob Demo,2048
";

/// Applies the E2E state overrides when `MAILGRIT_E2E_DASHBOARD` is set.
///
/// No-op in production: without the env flag the function returns immediately,
/// leaving `AppState` untouched. Called once from `app()` BEFORE the first
/// `state.read()`, so that subsequent subscriptions (`screen`, `language`, `theme`)
/// observe the already-overridden state.
///
/// Fail-soft: on a parse error of the test CSV a warning is logged, and the
/// state is NOT switched to the dashboard (the E2E test then times out waiting
/// for the dashboard sentinel — a clear signal that the embedded CSV is out of
/// sync with the validator). The warning lands in `app-stdio.log`, which the
/// Playwright fixture PRESERVES into its test-results output on any failure —
/// so this path is diagnosable post-mortem (previously the log was deleted by
/// teardown and the timeout was fully opaque).
pub fn apply_e2e_overrides(state: &Signal<AppState>) {
    if std::env::var(E2E_DASHBOARD_ENV).is_err() {
        return;
    }
    tracing::info!("E2E mode: starting in the dashboard with test rows (env {E2E_DASHBOARD_ENV})");

    let profile = state.read().effective_profile();
    let bytes = E2E_TEST_CSV.as_bytes();
    let parsed = match parse_csv_bytes_auto(bytes, &profile) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("E2E: embedded test CSV is invalid: {e}");
            return;
        }
    };
    // The embedded CSV header is known; detect_mapping yields correct bindings.
    let header: Vec<String> = E2E_TEST_CSV
        .lines()
        .next()
        .unwrap_or("")
        .split(',')
        .map(str::to_string)
        .collect();
    let mapping = detect_mapping(&header, &profile);
    let editable: Vec<EditableUserRow> = parsed.rows.iter().map(EditableUserRow::from).collect();

    // Signal implements Copy — copy the pointer to call write()
    // (requires mutable access in Dioxus 0.7).
    let mut state = *state;
    let mut s = state.write();
    s.screen = Screen::Dashboard;
    s.session_ok = true;
    s.auth_status = AuthStatus::Connected;
    s.base_url = "https://mail.example.com/iredadmin".to_string();
    s.csv.rows = Some(Arc::new(parsed));
    s.csv.column_mapping = Some(Arc::new(mapping));
    s.csv.editable_rows = Some(editable);
    tracing::debug!("E2E: dashboard state applied");
}
