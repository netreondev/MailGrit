//! The "Operations" section of the panel — extracted from `dashboard.rs`.
//!
//! Contains the CSV-load + editable-table card, the bulk-operations card (a
//! User/Domain/Admin target switch, Create/Edit/Delete/export/diagnostics
//! buttons, a delete-confirmation Modal) and the result of the last operation.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::card::Card;
use crate::components::icon::{Icon, IconView};
use crate::components::modal::{Modal, ModalFooter};
use crate::components::progress::Progress;
use crate::components::segmented::{Segmented, SegmentedOption};
use crate::components::spinner::Spinner;
use crate::csv_summary::CsvSummary;
use crate::editable_table_view::{editable_table_view, password_controls_view};
use crate::ops::{do_export, launch_op, open_export_choice, run_diag};
use crate::screens::csv_load::load_csv_file;
use crate::state::{AppState, OpStatus};
use crate::views::{batch_result_view, failed_csv_rows_view, mapping_panel_view, preview_csv_rows};
use dioxus::prelude::*;
use mailgrit_core_domain::{BulkOperationKind, OperationTarget};

/// Options for the operation-target segmented control. Labels come from the
/// translation catalog.
fn target_options() -> Vec<SegmentedOption<OperationTarget>> {
    vec![
        SegmentedOption {
            value: OperationTarget::User,
            label: tr!("target.user"),
            icon: None,
        },
        SegmentedOption {
            value: OperationTarget::Domain,
            label: tr!("target.domain"),
            icon: None,
        },
        SegmentedOption {
            value: OperationTarget::Admin,
            label: tr!("target.admin"),
            icon: None,
        },
    ]
}

/// The "Operations" section: CSV + bulk operations + result.
///
/// Reads `state` via context (like `dashboard_screen`). The default section
/// `Operations` → behavior identical to Phase 14 on entry.
pub fn operations_section(state: Signal<AppState>) -> Element {
    // Read the language so Dioxus re-renders localized strings on language change.
    crate::i18n::subscribe_to_language(state);
    rsx! {
        div { class: "dash-grid",
            // Card 1: CSV upload.
            {csv_card(state)}

            // Card 2: Bulk operations.
            {ops_card(state)}

            // Card 3: Result of the last operation (full width).
            Card { class: "span-all".to_string(),
                h2 { IconView { icon: Icon::Check } {tr!("result.card_title")} }
                batch_result_view {}
            }
        }
    }
}

/// Card 1: CSV upload + mapping panel + editable table + password controls +
/// rejected rows.
fn csv_card(state: Signal<AppState>) -> Element {
    let op_status = state.read().op_status;
    // Read the language for re-rendering localized strings.
    crate::i18n::subscribe_to_language(state);
    let csv_summary = state
        .read()
        .csv
        .as_ref()
        .map(|c| CsvSummary::from_parsed(c));
    let rejected_text = csv_summary.as_ref().and_then(|summary| {
        (summary.failed > 0).then(|| format!("{} {}", summary.failed, tr!("csv.rejected")))
    });
    rsx! {
        Card {
            h2 { IconView { icon: Icon::Upload } {tr!("csv.card_title")} }
            p { class: "muted", {tr!("csv.format_hint")} }

            div { class: "op-row",
                Button {
                    kind: ButtonKind::Secondary,
                    size: ButtonSize::Small,
                    icon_left: Some(Icon::Upload),
                    disabled: op_status == OpStatus::Running,
                    onclick: move |_| {
                        // Native file-selection dialog via AsyncFileDialog:
                        // the blocking part runs on a separate thread (rfd on
                        // Windows spawns it itself), so the UI thread does not
                        // reenter the Dioxus runtime — just like in export.
                        // Parsing and state update happen after the path
                        // returns.
                        let mut s = state;
                        spawn(async move {
                            let title = tr!("csv.file_dialog_title");
                            let handle = rfd::AsyncFileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .set_title(title)
                                .pick_file()
                                .await;
                            if let Some(handle) = handle {
                                let path = handle.path().to_path_buf();
                                load_csv_file(&mut s, &path);
                            }
                        });
                    },
                    {tr!("csv.choose_file")}
                }
            }

            // Flexible column-mapping panel.
            {mapping_panel_view(state, csv_summary.as_ref())}

            if let Some(summary) = &csv_summary {
                div { class: "dash-stat-row",
                    span { class: "dash-stat", "{summary.valid}" }
                    span { class: "dash-stat-label", {tr!("csv.valid_rows")} }
                }
                if let Some(rej) = &rejected_text {
                    p { class: "muted", "{rej}" }
                }
            } else {
                p { class: "muted mt-3", {tr!("csv.not_loaded")} }
            }

            // Password-generation controls + the editable row table.
            {password_controls_view(state)}
            {editable_table_view(state)}

            // The rejected-CSV-rows table.
            {failed_csv_rows_view(&state)}
        }
    }
}

/// Card 2: bulk operations (target, buttons, Modal).
/// Reads all state inside (1 parameter → no argument/bool explosion).
fn ops_card(mut state: Signal<AppState>) -> Element {
    let op_status = state.read().op_status;
    let current_target = state.read().current_target;
    let has_session = state.read().session_ok;
    let has_csv = state.read().csv.is_some();
    // Read the language for re-rendering localized strings.
    crate::i18n::subscribe_to_language(state);
    let can_op = has_session && has_csv && op_status != OpStatus::Running;
    rsx! {
        Card {
            h2 { IconView { icon: Icon::Wrench } {tr!("ops.card_title")} }
            p {
                class: "muted",
                if op_status == OpStatus::Running {
                    {tr!("ops.status_running")}
                } else if has_session {
                    {tr!("ops.status_ready")}
                } else {
                    {tr!("ops.status_no_session")}
                }
            }

            // Operation progress.
            {if op_status == OpStatus::Running {
                rsx! {
                    div { class: "op-running",
                        Spinner {}
                        {tr!("ops.running_hint")}
                    }
                    Progress { indeterminate: true }
                }
            } else {
                rsx! {}
            }}
            // The target switch (changes current_target/profile, resets the CSV).
            div { class: "op-row",
                Segmented {
                    current: current_target,
                    options: target_options(),
                    disabled: op_status == OpStatus::Running,
                    onchange: move |t| {
                        state.write().set_current_target(t);
                    },
                }
            }

            {ops_buttons(state, op_status, current_target, can_op, has_csv, has_session)}
        }
    }
}

/// Bulk-operation buttons (Create/Edit/Delete/export/diagnostics) + the Modal.
/// Extracted from `ops_card` to comply with the clippy line limit
/// (too_many_lines).
fn ops_buttons(
    mut state: Signal<AppState>,
    op_status: OpStatus,
    current_target: OperationTarget,
    can_op: bool,
    has_csv: bool,
    has_session: bool,
) -> Element {
    let pending_delete = state.read().modals.pending_delete;
    let export_in_progress = state.read().export.export_in_progress;
    let has_result = state.read().batch_result.is_some();
    // Read the language for re-rendering localized button labels.
    crate::i18n::subscribe_to_language(state);
    rsx! {
        div { class: "op-row",
            Button {
                kind: ButtonKind::Primary,
                icon_left: Some(Icon::Plus),
                loading: op_status == OpStatus::Running,
                disabled: !can_op,
                onclick: move |_| {
                    launch_op(&mut state, current_target, BulkOperationKind::Create);
                },
                {tr!("action.create")}
            }
            Button {
                kind: ButtonKind::Secondary,
                icon_left: Some(Icon::Edit),
                loading: op_status == OpStatus::Running,
                disabled: !can_op,
                onclick: move |_| {
                    launch_op(&mut state, current_target, BulkOperationKind::Edit);
                },
                {tr!("action.edit")}
            }
        }
        div { class: "op-row",
            Button {
                kind: ButtonKind::Danger,
                icon_left: Some(Icon::Trash),
                disabled: !can_op || pending_delete,
                onclick: move |_| {
                    state.write().modals.pending_delete = true;
                },
                {tr!("action.delete")}
            }
        }
        div { class: "op-row",
            Button {
                kind: ButtonKind::Secondary,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Download),
                // Synchronized with the `open_export_choice` guard: export is
                // available if there is a CSV (editable table) OR a result of the
                // last operation (it holds a snapshot of the created credentials,
                // even if the table was already cleared by switching the tab).
                disabled: (!has_csv && !has_result)
                    || op_status == OpStatus::Running
                    || export_in_progress,
                onclick: move |_| open_export_choice(&mut state),
                {tr!("ops.save_csv")}
            }
        }
        div { class: "op-row",
            Button {
                kind: ButtonKind::Ghost,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Wrench),
                disabled: !has_session || !has_csv || op_status == OpStatus::Running,
                onclick: move |_| run_diag(&mut state),
                {tr!("ops.diag_forms")}
            }
        }

        // Fail-closed delete confirmation — a premium Modal.
        {delete_modal(state, current_target, pending_delete)}
        // The export-format selection modal (encrypted/plain).
        {export_choice_modal(state)}
    }
}

/// The delete-confirmation Modal (fail-closed). Extracted into a separate
/// function for the compactness of `ops_card`.
fn delete_modal(
    mut state: Signal<AppState>,
    current_target: OperationTarget,
    pending_delete: bool,
) -> Element {
    if !pending_delete {
        return rsx! {};
    }
    rsx! {
        Modal {
            title: tr!("ops.delete_modal_title"),
            icon: Some(Icon::Alert),
            icon_class: "modal-icon-danger".to_string(),
            on_close: move |()| {
                state.write().modals.pending_delete = false;
            },
            p { {tr!("ops.delete_modal_body")} }
            {preview_csv_rows(&state)}
            ModalFooter {
                Button {
                    kind: ButtonKind::Danger,
                    icon_left: Some(Icon::Trash),
                    onclick: move |_| {
                        state.write().modals.pending_delete = false;
                        launch_op(
                            &mut state,
                            current_target,
                            BulkOperationKind::Delete,
                        );
                    },
                    {tr!("ops.delete_confirm")}
                }
                Button {
                    kind: ButtonKind::Ghost,
                    onclick: move |_| {
                        state.write().modals.pending_delete = false;
                    },
                    {tr!("action.cancel")}
                }
            }
        }
    }
}

/// The export-format selection Modal: encrypted (default) or plain CSV.
/// Each option has an explanation so the mode's purpose is obvious.
fn export_choice_modal(mut state: Signal<AppState>) -> Element {
    // Subscribe to the language: a language change must re-render the localized
    // strings.
    crate::i18n::subscribe_to_language(state);
    let pending = state.read().export.pending_export_choice;
    if !pending {
        return rsx! {};
    }
    let encrypt = state.read().export_encrypt;
    let export_in_progress = state.read().export.export_in_progress;
    let encrypt_class = if encrypt {
        "export-option export-option-active"
    } else {
        "export-option"
    };
    let plain_class = if encrypt {
        "export-option"
    } else {
        "export-option export-option-active"
    };
    rsx! {
        Modal {
            title: tr!("export.choice_title"),
            icon: Some(Icon::Download),
            icon_class: "modal-icon-info".to_string(),
            on_close: move |()| {
                state.write().export.pending_export_choice = false;
            },
            p { class: "muted", {tr!("export.choice_body")} }

            button {
                class: "{encrypt_class}",
                onclick: move |_| {
                    state.write().export_encrypt = true;
                },
                div { class: "export-option-head",
                    IconView { icon: Icon::Shield, size: crate::components::icon::IconSize::Regular }
                    strong { {tr!("export.option_encrypted")} }
                }
                p { class: "muted", {tr!("export.option_encrypted_desc")} }
            }
            button {
                class: "{plain_class}",
                onclick: move |_| {
                    state.write().export_encrypt = false;
                },
                div { class: "export-option-head",
                    IconView { icon: Icon::Download, size: crate::components::icon::IconSize::Regular }
                    strong { {tr!("export.option_plain")} }
                }
                p { class: "muted", {tr!("export.option_plain_desc")} }
            }

            ModalFooter {
                Button {
                    kind: ButtonKind::Primary,
                    icon_left: Some(Icon::Download),
                    disabled: export_in_progress,
                    onclick: move |_| {
                        let encrypt = state.read().export_encrypt;
                        state.write().export.pending_export_choice = false;
                        do_export(&mut state, encrypt);
                    },
                    {tr!("export.confirm")}
                }
                Button {
                    kind: ButtonKind::Ghost,
                    onclick: move |_| {
                        state.write().export.pending_export_choice = false;
                    },
                    {tr!("action.cancel")}
                }
            }
        }
    }
}
