//! Master password entry modal (audit-log unlock and export-key protection).
//!
//! The master password protects the audit key via the Argon2id KDF (see
//! `core-security/kdf`). On first run (no key file) it is in create mode: the
//! password is entered twice (confirmation). On subsequent runs it is in unlock
//! mode: a single entry, verified against the stored token. The password is
//! NOT persisted — it lives only in memory for the duration of the session
//! (`AppState::master_password`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use crate::components::button::{Button, ButtonKind};
use crate::components::icon::Icon;
use crate::components::modal::{Modal, ModalFooter};
use crate::state::AppState;
use dioxus::prelude::*;

/// Minimum master password length.
const MIN_PASSWORD_LEN: usize = 8;

/// Master password entry/creation modal. Shown when
/// `pending_master_password == true`. Automatically selects the mode (create
/// vs unlock) based on whether the audit key file exists.
pub fn master_password_modal(mut state: Signal<AppState>) -> Element {
    // Subscribe to the language: tr!/t! read the global locale rather than a
    // Dioxus signal, so without this line a language change would not re-render
    // the modal (see the pattern in i18n.rs:16-21).
    let _ = state.read().language;
    let pending = state.read().modals.pending_master_password;
    if !pending {
        return rsx! {};
    }
    let is_create = !audit_key_exists();
    let input = state.read().master_password_input.clone();
    let confirm = state.read().master_password_confirm.clone();

    rsx! {
        Modal {
            title: tr!("master_password.title"),
            icon: Some(Icon::Lock),
            icon_class: "modal-icon-info".to_string(),
            on_close: move |()| {
                let mut s = state.write();
                s.modals.pending_master_password = false;
                s.export.pending_export_after_unlock = false;
                s.master_password_input.clear();
                s.master_password_confirm.clear();
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
                    onclick: move |_| {
                        let mut s = state.write();
                        s.modals.pending_master_password = false;
                        s.export.pending_export_after_unlock = false;
                        s.master_password_input.clear();
                        s.master_password_confirm.clear();
                    },
                    {tr!("master_password.cancel")}
                }
            }
        }
    }
}

/// Master password confirmation handler: validation (length, match on create)
/// → audit unlock → on a deferred export, resumes `do_export`. Extracted from
/// `master_password_modal` so the render function stays within the pedantic
/// 100-line limit and for the sake of testable logic.
fn confirm_master_password(state: &mut Signal<AppState>, is_create: bool) {
    let pw;
    let resume;
    {
        let mut s = state.write();
        pw = s.master_password_input.clone();
        // Validate length in CHARACTERS (not bytes): an 8-character Cyrillic
        // password is 16 bytes in UTF-8, and a byte-based threshold would pass it
        // incorrectly. Same convention as in password_policy.rs.
        if pw.chars().count() < MIN_PASSWORD_LEN {
            s.error_msg = Some(t!("master_password.too_short").to_string());
            return;
        }
        // On create — check that the passwords match.
        if is_create && pw != s.master_password_confirm {
            s.error_msg = Some(t!("master_password.mismatch").to_string());
            return;
        }
        // Audit unlock (key creation on first run).
        match s.unlock_audit(&pw) {
            Ok(()) => {
                s.error_msg = None;
                // Resume the deferred encrypted export: `do_export` set
                // `pending_export_after_unlock` when there was no master password
                // yet. Now there is — launch the export (encrypt=true, since that
                // encrypted mode is exactly what the password was needed for).
                resume = s.export.pending_export_after_unlock;
                s.export.pending_export_after_unlock = false;
            }
            Err(e) => {
                s.error_msg = Some(e);
                return;
            }
        }
    }
    // Release the write-scope BEFORE re-entering do_export — otherwise there would
    // be a reentrant borrow of the signal in the dialog/KDF.
    if resume {
        crate::ops_export::do_export(state, true);
    }
}

/// Checks whether an audit key file of the correct length exists
/// (→ unlock vs create mode). Delegates the length check to audit_ui, where the
/// `AUDIT_KEY_FILE_LEN` constant lives.
fn audit_key_exists() -> bool {
    crate::audit_ui::audit_key_file_is_valid()
}
