//! The editable CSV-rows table + the password-generation control panel.
//!
//! This is the core of the new MailGrit logic: the user loads a CSV (or adds
//! rows manually), sees a table with inline-editable cells (`<input>`), edits
//! values, and with one click auto-assigns passwords with configurable
//! complexity (length + character classes). On execution, the rows are
//! re-validated via [`EditableUserRow::to_sanitized`] (see
//! [`collect_sanitized_rows`](crate::screens::csv_load::collect_sanitized_rows)).
//!
//! Password logic (by user choice): "empty + button all".
//! - "Fill empty" generates a password only for rows with an empty password
//!   (manual/loaded passwords are preserved).
//! - "Regenerate all" replaces the password in all rows (with a confirmation
//!   Modal, since it is irreversible).

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::icon::{Icon, IconSize, IconView};
use crate::components::modal::{Modal, ModalFooter};
use crate::state::AppState;
use crate::state::OpStatus;
use dioxus::prelude::*;
use mailgrit_core_domain::{EditableField, EditableFieldError, EditableUserRow};

/// The password-generation control panel: length, character classes, buttons.
///
/// Character classes required by the server-side `[password_policy]`
/// (require_uppercase/lowercase/number/special) are locked — they cannot be
/// disabled, otherwise the generator would produce passwords failing the
/// strength check. The slider's minimum length is also no lower than the
/// policy's `min_len`.
pub fn password_controls_view(mut state: Signal<AppState>) -> Element {
    let has_rows = state
        .read()
        .editable_rows
        .as_ref()
        .is_some_and(|r| !r.is_empty());
    let op_running = state.read().op_status == OpStatus::Running;
    // Read the language for re-rendering localized strings.
    let _lang = state.read().language;
    let pw_gen = state.read().password_generator.clone();
    let policy = state.read().password_policy.clone();
    // Slider lower bound: no lower than the policy's min_len and no lower than 8
    // (the UI limit).
    let length_min = policy.min_len.max(8);
    let length_label = tr!("pw.length_label", n = pw_gen.clamped_for_label());
    let regenerate_confirm = state.read().pending_password_regenerate;

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
                            max: "32",
                            step: "1",
                            value: "{pw_gen.length}",
                            disabled: op_running,
                            oninput: move |ev: FormEvent| {
                                // Range slider: clamp to [length_min, 32] instead
                                // of a silent substitution on a parse failure.
                                let v = ev
                                    .value()
                                    .parse::<usize>()
                                    .ok()
                                    .map_or(state.read().password_generator.length, |n| n.clamp(length_min, 32));
                                state.write().password_generator.length = v;
                            },
                        }
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.use_uppercase,
                            disabled: op_running || policy.require_uppercase,
                            onchange: move |ev| state.write().password_generator.use_uppercase = ev.checked(),
                        }
                        " A–Z"
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.use_lowercase,
                            disabled: op_running || policy.require_lowercase,
                            onchange: move |ev| state.write().password_generator.use_lowercase = ev.checked(),
                        }
                        " a–z"
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.use_digits,
                            disabled: op_running || policy.require_number,
                            onchange: move |ev| state.write().password_generator.use_digits = ev.checked(),
                        }
                        " 0–9"
                    }
                    label { class: "pw-class",
                        input {
                            r#type: "checkbox",
                            checked: pw_gen.use_special,
                            disabled: op_running || policy.require_special,
                            onchange: move |ev| state.write().password_generator.use_special = ev.checked(),
                        }
                        " !@#"
                    }
                }
                div { class: "pw-controls-actions",
                    Button {
                        kind: ButtonKind::Secondary,
                        size: ButtonSize::Small,
                        icon_left: Some(Icon::Check),
                        disabled: op_running || !pw_gen.has_any_class(),
                        onclick: move |_| {
                            fill_empty_passwords(&mut state);
                        },
                        {tr!("pw.fill_empty")}
                    }
                    Button {
                        kind: ButtonKind::Ghost,
                        size: ButtonSize::Small,
                        disabled: op_running || !pw_gen.has_any_class(),
                        onclick: move |_| {
                            state.write().pending_password_regenerate = true;
                        },
                        {tr!("pw.regenerate_all")}
                    }
                }
            }
            {regenerate_confirm.then(|| regenerate_all_modal(state))}
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
                state.write().pending_password_regenerate = false;
            },
            p { {tr!("pw.regenerate_modal_body")} }
            ModalFooter {
                Button {
                    kind: ButtonKind::Primary,
                    onclick: move |_| {
                        regenerate_all_passwords(&mut state);
                    },
                    {tr!("pw.regenerate_confirm")}
                }
                Button {
                    kind: ButtonKind::Ghost,
                    onclick: move |_| {
                        state.write().pending_password_regenerate = false;
                    },
                    {tr!("action.cancel")}
                }
            }
        }
    }
}

/// The editable rows table: `<input>` cells, a validity indicator, and add/remove
/// row buttons.
///
/// For each row shows:
/// - per-cell format-error highlighting (domain/login/password/name/quota) via
///   [`EditableUserRow::validate_fields`] → the `input-cell-invalid` class;
/// - a password-strength indicator (a ⚠ icon with a tooltip) per the server-side
///   policy `state.password_policy` — warnings, not blocking the operation.
pub fn editable_table_view(mut state: Signal<AppState>) -> Element {
    let op_running = state.read().op_status == OpStatus::Running;
    // Read the rows, format errors, and password-strength warnings under a single
    // read-guard (the policy is taken here too, so a guard is not opened on every
    // row during render).
    let snapshot: Vec<(EditableUserRow, Vec<EditableFieldError>, String)> = {
        let read = state.read();
        let policy = read.password_policy.clone();
        match read.editable_rows.as_ref() {
            Some(rows) => rows
                .iter()
                .map(|r| {
                    let errs = crate::error_i18n::validate_fields_localized(r);
                    // Password strength per the server-side policy (the warnings
                    // do not contain the password itself, only violations — safe
                    // for the tooltip). Localization via error_i18n
                    // (PasswordWarning is typed).
                    let pw_warns: String = policy
                        .validate(&r.password)
                        .into_iter()
                        .map(|w| crate::error_i18n::password_warning(&w))
                        .collect::<Vec<_>>()
                        .join("; ");
                    (r.clone(), errs, pw_warns)
                })
                .collect(),
            None => Vec::new(),
        }
    };
    let row_count = snapshot.len();

    if row_count == 0 {
        return rsx! {};
    }

    rsx! {
        div { class: "editable-table-wrap",
            h3 { {tr!("table.rows_title", count = row_count)} }
            div { class: "editable-table-scroll",
                table { class: "table table-fixed editable-table",
                    colgroup {
                        col { class: "col-domain" }
                        col { class: "col-username" }
                        col { class: "col-password" }
                        col { class: "col-display" }
                        col { class: "col-quota" }
                        col { class: "col-actions" }
                    }
                    thead { tr {
                        th { {tr!("table.col_domain")} }
                        th { {tr!("table.col_username")} }
                        th { {tr!("table.col_password")} }
                        th { {tr!("table.col_display")} }
                        th { {tr!("table.col_quota")} }
                        th { "" }
                    } }
                    tbody {
                        for (idx, (row, errs, pw_warns)) in snapshot.iter().enumerate() {
                            {render_row(state, idx, row, errs, pw_warns, op_running)}
                        }
                    }
                }
            }
            div { class: "editable-table-foot",
                Button {
                    kind: ButtonKind::Secondary,
                    size: ButtonSize::Small,
                    icon_left: Some(Icon::Plus),
                    disabled: op_running,
                    onclick: move |_| {
                        state
                            .write()
                            .editable_rows
                            .get_or_insert_with(Vec::new)
                            .push(EditableUserRow::empty_with_default_quota());
                    },
                    {tr!("table.add_row")}
                }
            }
        }
    }
}

/// Renders a single table row with inline `<input>` cells.
///
/// Format-error highlighting is per-cell (the `input-cell-invalid` class on a
/// specific input); the whole row gets `row-invalid` for visual consistency and
/// a `title` with the first error (a native tooltip on hover). The
/// password-strength indicator (a ⚠ icon) is shown only when there are warnings.
//
// too_many_lines: the RSX markup of one table row with 5 inputs + actions;
// extracting the cells into separate components would complicate reading (shared
// `state`/`idx`). Allow.
#[allow(clippy::too_many_arguments)]
#[allow(
    clippy::too_many_lines,
    reason = "table-row RSX markup; extracting cells harms readability"
)]
fn render_row(
    mut state: Signal<AppState>,
    idx: usize,
    row: &EditableUserRow,
    errs: &[EditableFieldError],
    pw_warns: &str,
    op_running: bool,
) -> Element {
    let invalid = !errs.is_empty();
    let title_attr = errs.first().map_or("", |e| e.message.as_str());
    // Per-cell format-error flags + ready classes for each input.
    let class_domain = if errs.iter().any(|e| e.field == EditableField::Domain) {
        "input input-cell input-cell-invalid"
    } else {
        "input input-cell"
    };
    let class_username = if errs.iter().any(|e| e.field == EditableField::Username) {
        "input input-cell input-cell-invalid"
    } else {
        "input input-cell"
    };
    let class_password = if errs.iter().any(|e| e.field == EditableField::Password) {
        "input input-cell mono input-cell-invalid"
    } else {
        "input input-cell mono"
    };
    let class_display = if errs.iter().any(|e| e.field == EditableField::DisplayName) {
        "input input-cell input-cell-invalid"
    } else {
        "input input-cell"
    };
    let class_quota = if errs.iter().any(|e| e.field == EditableField::Quota) {
        "input input-cell input-cell-invalid"
    } else {
        "input input-cell"
    };
    let has_pw_warn = !pw_warns.is_empty();
    rsx! {
        tr {
            class: if invalid { "row-invalid" } else { "" },
            title: "{title_attr}",
            td { input {
                class: "{class_domain}",
                r#type: "text",
                value: "{row.domain}",
                disabled: op_running,
                placeholder: "example.com",
                oninput: move |ev: FormEvent| {
                    set_field(&mut state, idx, |r| r.domain = ev.value());
                },
            } }
            td { input {
                class: "{class_username}",
                r#type: "text",
                value: "{row.username}",
                disabled: op_running,
                placeholder: "ivan.petrov",
                oninput: move |ev: FormEvent| {
                    set_field(&mut state, idx, |r| r.username = ev.value());
                },
            } }
            td { class: "td-password",
                input {
                    class: "{class_password}",
                    r#type: "text",
                    value: "{row.password}",
                    disabled: op_running,
                    placeholder: tr!("table.placeholder_password"),
                    oninput: move |ev: FormEvent| {
                        set_field(&mut state, idx, |r| r.password = ev.value());
                    },
                }
                // Password-strength indicator: a warning icon with a tooltip
                // showing policy violations (length/classes). Does not block the
                // operation — it only informs. The warnings do not contain the
                // password. The span wrapper carries the title (the native
                // tooltip), since IconView itself is an SVG without a title
                // attribute.
                if has_pw_warn {
                    span {
                        class: "pw-strength-warn",
                        title: "{pw_warns}",
                        IconView {
                            icon: Icon::Alert,
                            size: IconSize::Small,
                        }
                    }
                }
                button {
                    class: "btn btn-ghost btn-icon btn-gen-pw",
                    title: tr!("pw.gen_one_title"),
                    r#type: "button",
                    disabled: op_running,
                    onclick: move |_| {
                        generate_one(&mut state, idx);
                    },
                    IconView { icon: Icon::Lock }
                }
            }
            td { input {
                class: "{class_display}",
                r#type: "text",
                value: "{row.display_name}",
                disabled: op_running,
                placeholder: tr!("table.placeholder_display"),
                oninput: move |ev: FormEvent| {
                    set_field(&mut state, idx, |r| r.display_name = ev.value());
                },
            } }
            td { input {
                class: "{class_quota}",
                r#type: "text",
                value: "{row.quota}",
                disabled: op_running,
                placeholder: "1024",
                oninput: move |ev: FormEvent| {
                    set_field(&mut state, idx, |r| r.quota = ev.value());
                },
            } }
            td { class: "td-actions",
                button {
                    class: "btn btn-ghost btn-icon",
                    title: tr!("table.delete_row_title"),
                    r#type: "button",
                    disabled: op_running,
                    onclick: move |_| {
                        if let Some(rows) = state.write().editable_rows.as_mut()
                            && idx < rows.len()
                        {
                            rows.remove(idx);
                        }
                    },
                    IconView { icon: Icon::Trash }
                }
            }
        }
    }
}

/// Applies a mutation to row `idx` of the editable table (a let-chain instead of
/// nested if let — clippy collapsible_if). The closure `f` receives `&mut` to the
/// row.
fn set_field<F>(state: &mut Signal<AppState>, idx: usize, f: F)
where
    F: FnOnce(&mut mailgrit_core_domain::EditableUserRow),
{
    if let Some(rows) = state.write().editable_rows.as_mut()
        && let Some(r) = rows.get_mut(idx)
    {
        f(r);
    }
}

// ============================================================================
// Password-generation actions (mutate editable_rows via password_generator).
// ============================================================================

/// Generates a password for a single row (the lock button in the password cell).
fn generate_one(state: &mut Signal<AppState>, idx: usize) {
    let pw = state.read().password_generator.generate();
    set_field(state, idx, |r| r.password = pw);
}

/// Fills a password only for rows with an empty password (manual/loaded ones are
/// NOT touched).
fn fill_empty_passwords(state: &mut Signal<AppState>) {
    let pw_gen = state.read().password_generator.clone();
    if !pw_gen.has_any_class() {
        state.write().error_msg = Some(tr!("pw.need_one_class"));
        return;
    }
    let mut filled = 0usize;
    if let Some(rows) = state.write().editable_rows.as_mut() {
        for r in rows.iter_mut() {
            if r.password_is_empty() {
                r.password = pw_gen.generate();
                filled = filled.saturating_add(1);
            }
        }
    }
    state.write().error_msg = match filled {
        0 => Some(tr!("pw.none_empty")),
        n => Some(tr!("pw.filled_count", n = n)),
    };
}

/// Regenerates the password in ALL rows (after a Modal confirmation).
fn regenerate_all_passwords(state: &mut Signal<AppState>) {
    let pw_gen = state.read().password_generator.clone();
    state.write().pending_password_regenerate = false;
    if !pw_gen.has_any_class() {
        state.write().error_msg = Some(tr!("pw.need_one_class"));
        return;
    }
    let mut count = 0usize;
    if let Some(rows) = state.write().editable_rows.as_mut() {
        for r in rows.iter_mut() {
            r.password = pw_gen.generate();
            count = count.saturating_add(1);
        }
    }
    state.write().error_msg = Some(tr!("pw.regenerated_count", n = count));
}
