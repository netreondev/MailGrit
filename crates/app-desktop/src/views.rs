//! Auxiliary views (sub-views) for the application screens.
//!
//! Some of them are plain `-> Element` functions called as functions
//! (`preview_csv_rows(&state)`); two (`batch_result_view`, `audit_view`) are
//! `#[component]` and used as RSX tags in `dashboard_screen`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::icon::{Icon, IconSize, IconView};
use crate::csv_summary::CsvSummary;
use crate::state::{AppState, OpStatus};
use dioxus::prelude::*;
use mailgrit_core_csv::detect_mapping;
use std::sync::Arc;

/// Preview of CSV rows for delete confirmation (in a Modal).
pub fn preview_csv_rows(state: &Signal<AppState>) -> Element {
    let read = state.read();
    let Some(csv) = read.csv.rows.as_ref() else {
        return rsx! { p { class: "muted", {tr!("csv.preview_empty")} } };
    };
    let preview: Vec<String> = csv
        .rows
        .iter()
        .take(15)
        .map(|r| format!("{}@{}", r.username.as_str(), r.domain.as_str()))
        .collect();
    let more = csv.rows.len().saturating_sub(preview.len());
    let more_text = if more > 0 {
        Some(tr!("csv.preview_more", count = more))
    } else {
        None
    };
    rsx! {
        div { class: "preview-rows",
            for email in &preview {
                div { class: "mono preview-row", "{email}" }
            }
        }
        if let Some(m) = &more_text {
            p { class: "muted", "{m}" }
        }
    }
}

/// Collapsible cookie diagnostics panel (hidden by default — does not break the
/// premium clean look of the screen; expands for debugging login behind a WAF).
pub fn cookies_disclosure(state: &Signal<AppState>) -> Element {
    let cookies = state.read().last_cookies.clone();
    if cookies.is_empty() {
        return rsx! {};
    }
    let session_name = state.read().session_cookie_name.clone();
    // Read the language to re-render the localized strings.
    crate::i18n::subscribe_to_language(*state);
    // Prepare the strings for display (avoiding temporary borrows in RSX).
    let yes = tr!("cookies.yes");
    let no = tr!("cookies.no");
    let rows: Vec<(String, String, String, String, usize)> = cookies
        .iter()
        .map(|c| {
            (
                c.name.clone(),
                c.domain.clone().unwrap_or_else(|| "-".into()),
                c.path.clone().unwrap_or_else(|| "-".into()),
                if c.http_only { yes.clone() } else { no.clone() },
                c.value_len,
            )
        })
        .collect();
    let count = cookies.len();
    rsx! {
        details { class: "cookies-disclosure",
            summary { class: "disclosure-trigger",
                IconView { icon: Icon::ChevronRight, size: IconSize::Small, class: "disclosure-chevron".to_string() }
                {tr!("cookies.trigger", count = count)}
            }
            p { class: "muted",
                {tr!("cookies.hint", name = session_name)}
            }
            table { class: "table table-fixed",
                thead { tr {
                    th { {tr!("cookies.col_name")} }
                    th { {tr!("cookies.col_domain")} }
                    th { {tr!("cookies.col_path")} }
                    th { {tr!("cookies.col_httponly")} }
                    th { {tr!("cookies.col_length")} }
                } }
                tbody {
                    for (name, domain, path, http_only, vlen) in &rows {
                        tr {
                            td { class: "mono", "{name}" }
                            td { "{domain}" }
                            td { "{path}" }
                            td { "{http_only}" }
                            td { "{vlen}" }
                        }
                    }
                }
            }
        }
    }
}

/// View of rejected CSV rows (ParsedCsv::failed) — a table with the reason.
pub fn failed_csv_rows_view(state: &Signal<AppState>) -> Element {
    // Read the language to re-render the localized strings.
    crate::i18n::subscribe_to_language(*state);
    // Collect the data to display under the read-guard, releasing it before rendering.
    let (total, rows): (usize, Vec<(usize, String, String)>) = {
        let read = state.read();
        read.csv.rows.as_ref().map_or_else(
            || (0, Vec::new()),
            |csv| {
                let rows = csv
                    .failed
                    .iter()
                    .take(20)
                    // Localized reason via error_i18n (the core crate has no i18n).
                    .map(|f| {
                        (
                            f.line_no,
                            f.fields.join(", "),
                            crate::error_i18n::csv_parse_error(&f.error),
                        )
                    })
                    .collect();
                (csv.failed.len(), rows)
            },
        )
    };
    if total == 0 {
        return rsx! {};
    }
    let more = total.saturating_sub(rows.len());
    let more_text = if more > 0 {
        Some(tr!("csv.more_rejected", count = more))
    } else {
        None
    };
    rsx! {
        h3 { {tr!("csv.failed_title", count = total)} }
        table { class: "table table-fixed",
            thead { tr {
                th { {tr!("csv.col_line")} }
                th { {tr!("csv.col_content")} }
                th { {tr!("csv.col_reason")} }
            } }
            tbody {
                for (line_no, fields, error) in &rows {
                    tr {
                        td { "{line_no}" }
                        td { "{fields}" }
                        td { "{error}" }
                    }
                }
            }
        }
        if let Some(m) = &more_text {
            p { class: "muted", "{m}" }
        }
    }
}

/// View of the last bulk operation result.
#[component]
pub fn batch_result_view() -> Element {
    let state = use_context::<Signal<AppState>>();
    // Read the language to re-render the localized strings.
    crate::i18n::subscribe_to_language(state);
    let result = state.read().csv.batch_result.clone();

    let Some(result) = result else {
        return rsx! { p { class: "muted", {tr!("result.none_yet")} } };
    };
    let failures = result.failures.clone();
    let succeeded = tr!("result.succeeded", count = result.succeeded);
    let failed = tr!("result.failed", count = result.failed);
    let reason_title = tr!("result.reason_title");
    rsx! {
        p { "{succeeded}" }
        p { "{failed}" }
        if !failures.is_empty() {
            h3 { {tr!("result.rejected_title")} }
            table { class: "table table-fixed",
                thead { tr {
                    th { {tr!("result.col_user")} }
                    th { {tr!("result.col_domain")} }
                    th { {tr!("result.col_reason")} }
                } }
                tbody {
                    for f in &failures {
                        tr {
                            td { "{f.username}" }
                            td { "{f.domain}" }
                            // E1: the reason is now human-readable ("Account does not
                            // exist"); we highlight it with the danger color from the tokens.
                            td {
                                class: "text-danger",
                                title: "{reason_title}",
                                "{f.reason}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// View of the recent audit-log entries.
#[component]
pub fn audit_view() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let entries = state.read().audit_entries.clone();
    let has_audit = state.read().audit.is_some();
    // Read the language to re-render the localized strings.
    crate::i18n::subscribe_to_language(state);

    if !has_audit {
        // The audit is locked by the master password. Offer to unlock it.
        return rsx! {
            div { class: "audit-locked",
                p { class: "muted", {tr!("audit.unavailable")} }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| {
                        state.write().modals.pending_master_password = true;
                    },
                    {tr!("master_password.unlock")}
                }
            }
        };
    }
    if entries.is_empty() {
        return rsx! { p { class: "muted", {tr!("audit.empty")} } };
    }
    rsx! {
        button {
            class: "btn-secondary",
            onclick: move |_| {
                // Verify the audit hash chain. Clone the Arc<AuditWriter> in a
                // short read-scope (so as not to hold the signal borrow during the
                // blocking SQLite read of the whole chain) and move verify into
                // spawn_blocking — the UI does not freeze on large logs.
                let audit = { state.read().audit.clone() };
                let mut state_clone = state;
                spawn(async move {
                    let result = match audit {
                        Some(audit) => {
                            let join = crate::tokio_runtime().spawn_blocking(move || {
                                audit.verify()
                            });
                            join.await.map_or(
                                Some(Err(crate::audit_ui::AuditError::PoisonedLock)),
                                Some,
                            )
                        }
                        None => None,
                    };
                    // Distinguish a real tampering (ChainBroken) from a transient
                    // database read error, so we do not lie to the user about a
                    // "tampered log" on any SQLite error.
                    let msg = match result {
                        Some(Ok(())) => tr!("audit.chain_ok"),
                        Some(Err(crate::audit_ui::AuditError::Tampered(_))) => {
                            tr!("audit.chain_broken")
                        }
                        Some(Err(crate::audit_ui::AuditError::Storage(_))) => {
                            tr!("audit.verify_storage_error")
                        }
                        Some(Err(crate::audit_ui::AuditError::PoisonedLock)) => {
                            tr!("audit.verify_lock_error")
                        }
                        // WrongMasterPassword does not occur during verify (the audit is already open).
                        Some(Err(crate::audit_ui::AuditError::WrongMasterPassword)) => {
                            tr!("master_password.wrong")
                        }
                        // CorruptedKeyFile occurs only on open(), not on verify(),
                        // but for exhaustiveness we use the same key as on unlock.
                        Some(Err(crate::audit_ui::AuditError::CorruptedKeyFile { .. })) => {
                            tr!("master_password.corrupt_key")
                        }
                        // KDF/crypto failures cannot occur during verify (the key
                        // is already derived), but stay distinguishable if that
                        // ever changes — reported as a technical error, not
                        // "tampered".
                        Some(Err(
                            crate::audit_ui::AuditError::Kdf(_)
                            | crate::audit_ui::AuditError::Crypto(_),
                        )) => tr!("audit.verify_storage_error"),
                        None => tr!("audit.unavailable"),
                    };
                    state_clone.write().error_msg = Some(msg);
                });
            },
            {tr!("audit.verify_btn")}
        }
        table { class: "table table-fixed",
            thead { tr {
                th { {tr!("audit.col_time")} }
                th { {tr!("audit.col_action")} }
                th { {tr!("audit.col_detail")} }
            } }
            tbody {
                for e in &entries {
                    tr {
                        td { "{e.timestamp}" }
                        td { "{e.action}" }
                        td { "{e.detail}" }
                    }
                }
            }
        }
    }
}

/// Flexible CSV column mapping panel (Phase 1.3): shows the current operation
/// profile, an auto-detect mapping button, and a summary of the matched columns.
pub fn mapping_panel_view(
    mut state: Signal<AppState>,
    csv_summary: Option<&CsvSummary>,
) -> Element {
    let op_status = state.read().op_status;
    let mapping_info = state
        .read()
        .csv
        .column_mapping
        .as_ref()
        .map(|m| (m.bindings.len(), m.profile.fields.len(), m.header.clone()));
    // Profile label via op_label (target+kind are taken from the active profile,
    // so the label correctly reflects the chosen target: User/Domain/Admin).
    let active_profile = state.read().effective_profile();
    let profile_label =
        crate::op_label::operation_label(active_profile.target, active_profile.kind);
    rsx! {
        div { class: "op-row",
            span { class: "muted mono", {tr!("mapping.profile", label = profile_label)} }
            Button {
                kind: ButtonKind::Ghost,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Search),
                disabled: csv_summary.is_none() || op_status == OpStatus::Running,
                onclick: move |_| {
                    // Re-detect the mapping from the loaded CSV header.
                    let s = state.read();
                    let profile = s.effective_profile();
                    if let Some(m) = &s.csv.column_mapping {
                        let header = m.header.clone();
                        let mapping = detect_mapping(&header, &profile);
                        drop(s);
                        state.write().csv.column_mapping = Some(Arc::new(mapping));
                    }
                },
                {tr!("mapping.auto_detect")}
            }
        }
        {if let Some((bound, total, header)) = &mapping_info {
            let detected = if *bound == *total {
                tr!("mapping.all_detected")
            } else {
                tr!("mapping.partial_detected", bound = bound, total = total)
            };
            let summary = if header.is_empty() {
                detected
            } else {
                format!("{detected} ({header:?})")
            };
            rsx! {
                p { class: "muted mt-1",
                    {tr!("mapping.summary", info = summary)}
                }
            }
        } else {
            rsx! {}
        }}
    }
}
