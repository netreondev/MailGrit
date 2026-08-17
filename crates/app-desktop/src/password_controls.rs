//! The password-generation control panel: length slider, character-class
//! checkboxes (policy-locked ones disabled), fill-empty / regenerate-all
//! actions, and the regeneration confirmation modal.
//!
//! Extracted from `editable_table_view.rs` to keep each file under the
//! 400-line spec.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::icon::{Icon, IconView};
use crate::components::modal::{Modal, ModalFooter};
use crate::state::AppState;
use crate::state::OpStatus;
use dioxus::prelude::*;

/// The password-generation control panel: length, character classes, buttons.
///
/// Character classes required by the server-side `[password_policy]`
/// (`require_uppercase/lowercase/number/special`) are locked — they cannot be
/// disabled, otherwise the generator would produce passwords failing the
/// strength check. The slider's minimum length is also no lower than the
/// policy's `min_len`.
pub fn password_controls_view(mut state: Signal<AppState>) -> Element {
    let has_rows = state
        .read()
        .csv
        .editable_rows
        .as_ref()
        .is_some_and(|r| !r.is_empty());
    let op_running = state.read().op_status == OpStatus::Running;
    let ui_max = mailgrit_core_domain::UI_MAX_LENGTH;
    // Read the language for re-rendering localized strings.
    crate::i18n::subscribe_to_language(state);
    let pw_gen = state.read().password_generator.clone();
    let policy = state.read().password_policy.clone();
    // Slider lower bound: no lower than the policy's min_len and no lower than 8
    // (the UI limit).
    let length_min = policy.min_len.max(8);
    let length_label = tr!("pw.length_label", n = pw_gen.clamped_for_label());
    let regenerate_confirm = state.read().modals.pending_password_regenerate;

    rsx! {
        if has_rows {
            div { class: "pw-controls",
                h3 { IconView { icon: Icon::Lock } {tr!("pw.card_title")} }
                div { class: "pw-controls-row",
                    label { class: "pw-length",
                        span { class: "muted", "{length_label}" }
                        input {
                            class: "pw-length-slider",
                            r#type: "range",
                            min: "{length_min}",
                            max: "{ui_max}",
                            step: "1",
                            value: "{pw_gen.length}",
                            disabled: op_running,
                            oninput: move |ev: FormEvent| {
                                // Range slider: clamp to [length_min, ui_max]
                                // (UI_MAX_LENGTH — the single source shared with
                                // the domain clamp) instead of a silent
                                // substitution on a parse failure.
                                let v = ev
                                    .value()
                                    .parse::<usize>()
                                    .ok()
                                    .map_or(state.read().password_generator.length, |n| n.clamp(length_min, ui_max));
                                state.write().password_generator.length = v;
                            },
                        }
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.classes.uppercase(),
                            disabled: op_running || policy.classes.uppercase(),
                            onchange: move |ev| state.write().password_generator.classes.set_uppercase(ev.checked()),
                        }
                        {tr!("pw.class_upper")}
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.classes.lowercase(),
                            disabled: op_running || policy.classes.lowercase(),
                            onchange: move |ev| state.write().password_generator.classes.set_lowercase(ev.checked()),
                        }
                        {tr!("pw.class_lower")}
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.classes.digits(),
                            disabled: op_running || policy.classes.digits(),
                            onchange: move |ev| state.write().password_generator.classes.set_digits(ev.checked()),
                        }
                        {tr!("pw.class_digits")}
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.classes.special(),
                            disabled: op_running || policy.classes.special(),
                            onchange: move |ev| state.write().password_generator.classes.set_special(ev.checked()),
                        }
                        {tr!("pw.class_special")}
                    }
                }
                {pw_actions_row(state, op_running, pw_gen.has_any_class())}
            }
            {regenerate_confirm.then(|| regenerate_all_modal(state))}
        }
    }
}

/// "Fill empty" and "Regenerate all" action buttons (extracted to keep
/// `password_controls_view` within the 100-line pedantic limit).
fn pw_actions_row(mut state: Signal<AppState>, op_running: bool, has_any_class: bool) -> Element {
    rsx! {
        div { class: "pw-controls-actions",
            Button {
                kind: ButtonKind::Secondary,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Check),
                disabled: op_running || !has_any_class,
                onclick: move |_| {
                    crate::editable_table_view::fill_empty_passwords(&mut state);
                },
                {tr!("pw.fill_empty")}
            }
            Button {
                kind: ButtonKind::Ghost,
                size: ButtonSize::Small,
                disabled: op_running || !has_any_class,
                onclick: move |_| {
                    state.write().modals.pending_password_regenerate = true;
                },
                {tr!("pw.regenerate_all")}
            }
        }
    }
}

/// The "Regenerate all" confirmation Modal (an irreversible operation).
fn regenerate_all_modal(mut state: Signal<AppState>) -> Element {
    rsx! {
        Modal {
            title: tr!("pw.regenerate_modal_title"),
            icon: Some(Icon::Alert),
            icon_class: "modal-icon-danger".to_string(),
            on_close: move |()| {
                state.write().modals.pending_password_regenerate = false;
            },
            p { {tr!("pw.regenerate_modal_body")} }
            ModalFooter {
                Button {
                    kind: ButtonKind::Primary,
                    onclick: move |_| {
                        crate::editable_table_view::regenerate_all_passwords(&mut state);
                    },
                    {tr!("pw.regenerate_confirm")}
                }
                Button {
                    kind: ButtonKind::Ghost,
                    onclick: move |_| {
                        state.write().modals.pending_password_regenerate = false;
                    },
                    {tr!("action.cancel")}
                }
            }
        }
    }
}
