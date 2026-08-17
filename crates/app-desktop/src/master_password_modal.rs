//! Master password entry modal (audit-log unlock and export-key encryption).
//!
//! The master password protects the audit key via the Argon2id KDF (see
//! `core-security/kdf`). On first run (no key file) it is in create mode: the
//! password is entered twice (confirmation). On subsequent runs it is in unlock
//! mode: a single entry, verified against the stored token. The password is
//! NOT persisted — it lives only in memory for the duration of the session
//! (`AppState::master_password`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind};
use crate::components::icon::Icon;
use crate::components::modal::{Modal, ModalFooter};
use crate::state::AppState;
use dioxus::prelude::*;
use zeroize::{Zeroize, Zeroizing};

/// Minimum master password length.
const MIN_PASSWORD_LEN: usize = 8;

/// Master password entry/creation modal. Shown when
/// `pending_master_password == true`. Automatically selects the mode (create
/// vs unlock) based on whether the audit key file exists.
pub fn master_password_modal(mut state: Signal<AppState>) -> Element {
    // Subscribe to the language: tr!/t! read the global locale rather than a
    // Dioxus signal, so without this line a language change would not re-render
    // the modal (see the pattern in i18n.rs:16-21).
    crate::i18n::subscribe_to_language(state);
    let pending = state.read().modals.pending_master_password;
    if !pending {
        return rsx! {};
    }
    // Create-vs-unlock was snapshotted at modal OPEN time
    // (`AppState::open_master_password_modal`) — reading it here avoids a
    // filesystem probe on every re-render (each keystroke re-renders).
    let is_create = matches!(
        state.read().master_password_mode,
        crate::state::MasterPasswordMode::Create
    );
    let input = state.read().master_password_input.clone();
    let confirm = state.read().master_password_confirm.clone();
    // While the Argon2id unlock task is running, both footer buttons are
    // disabled (a second submission would race the task).
    let pending_unlock = state.read().unlock_pending;

    rsx! {
        Modal {
            title: tr!("master_password.title"),
            icon: Some(Icon::Lock),
            icon_class: "modal-icon-info".to_string(),
            on_close: move |()| {
                let mut s = state.write();
                s.modals.pending_master_password = false;
                s.export.pending_export_after_unlock = false;
                s.master_password_input.zeroize();
                s.master_password_confirm.zeroize();
            },
            p {
                if is_create {
                    {tr!("master_password.create")}
                } else {
                    {tr!("master_password.enter")}
                }
            }
            p { class: "muted", {tr!("master_password.hint")} }

            div { class: "field",
                label { class: "field-label", {tr!("master_password.title")} }
                input {
                    class: "input",
                    r#type: "password",
                    value: "{input}",
                    autocomplete: "off",
                    oninput: move |e| {
                        state.write().master_password_input = e.value();
                    },
                }
            }

            if is_create {
                div { class: "field",
                    label { class: "field-label", {tr!("master_password.confirm")} }
                    input {
                        class: "input",
                        r#type: "password",
                        value: "{confirm}",
                        autocomplete: "off",
                        oninput: move |e| {
                            state.write().master_password_confirm = e.value();
                        },
                    }
                }
            }

            ModalFooter {
                Button {
                    kind: ButtonKind::Primary,
                    icon_left: Some(Icon::Lock),
                    disabled: pending_unlock,
                    onclick: move |_| {
                        confirm_master_password(&mut state, is_create);
                    },
                    if is_create {
                        {tr!("master_password.create_btn")}
                    } else {
                        {tr!("master_password.unlock")}
                    }
                }
                Button {
                    kind: ButtonKind::Ghost,
                    disabled: pending_unlock,
                    onclick: move |_| {
                        let mut s = state.write();
                        s.modals.pending_master_password = false;
                        s.export.pending_export_after_unlock = false;
                        s.master_password_input.zeroize();
                        s.master_password_confirm.zeroize();
                    },
                    {tr!("master_password.cancel")}
                }
            }
        }
    }
}

/// Master password confirmation handler: validation (length, match on create) →
/// audit unlock in a background task → on success applies the result and, for a
/// deferred export, resumes `do_export`. Extracted from `master_password_modal`
/// so the render function stays within the pedantic 100-line limit.
///
/// The Argon2id KDF inside `AuditWriter::open` is memory-hard (64 MiB, t=3) —
/// hundreds of milliseconds. It runs in `spawn_blocking` on the tokio runtime
/// so the UI event loop never freezes (the same pattern as the export pipeline
/// in `ops_export`); `unlock_pending` blocks a repeated submission while the
/// task is in flight.
fn confirm_master_password(state: &mut Signal<AppState>, is_create: bool) {
    let pw;
    {
        let mut s = state.write();
        if s.unlock_pending {
            return;
        }
        pw = Zeroizing::new(s.master_password_input.clone());
        // Validate length in CHARACTERS (not bytes): an 8-character Cyrillic
        // password is 16 bytes in UTF-8, and a byte-based threshold would pass it
        // incorrectly. Same convention as in password_policy.rs.
        if pw.chars().count() < MIN_PASSWORD_LEN {
            s.error_msg = Some(t!("master_password.too_short").to_string());
            return;
        }
        // On create — check that the passwords match.
        if is_create && pw.as_str() != s.master_password_confirm {
            s.error_msg = Some(t!("master_password.mismatch").to_string());
            return;
        }
        s.unlock_pending = true;
    }
    // A second zeroized copy travels into the task result and is stored in
    // AppState for the encrypted export (complete_unlock).
    let pw_for_state = pw.clone();
    let mut state_clone = *state;
    spawn(async move {
        let join = crate::tokio_runtime()
            .spawn_blocking(move || crate::audit_ui::AuditWriter::open(pw.as_str()));
        let opened = match join.await {
            Ok(opened) => opened,
            Err(e) => Err(crate::audit_ui::AuditError::Storage(format!(
                "unlock task failed: {e}"
            ))),
        };
        match opened {
            Ok(audit) => {
                let resume = {
                    let mut s = state_clone.write();
                    s.complete_unlock(pw_for_state, audit);
                    // Resume the deferred encrypted export: `do_export` set
                    // `pending_export_after_unlock` when there was no master password
                    // yet. Now there is — launch the export (encrypt=true, since that
                    // encrypted mode is exactly what the password was needed for).
                    let resume = s.export.pending_export_after_unlock;
                    s.export.pending_export_after_unlock = false;
                    resume
                };
                // Release the write-scope BEFORE re-entering do_export — otherwise
                // there would be a reentrant borrow of the signal in the dialog.
                if resume {
                    crate::ops_export::do_export(&mut state_clone, true);
                }
            }
            Err(e) => {
                let mut s = state_clone.write();
                s.unlock_pending = false;
                s.error_msg = Some(crate::error::AppError::Audit(e).user_message());
            }
        }
    });
}
