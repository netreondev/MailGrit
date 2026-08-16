//! Operations panel: context-bar, top-level navigation (Operations/Audit),
//! and section dispatcher.
//!
//! MailGrit is focused: the "Operations" section is CSV load → editable table
//! → password generation → target → execution → result; the "Audit" section is
//! the hash-chained operations log. Default section is `Operations`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::badge::{Badge, BadgeKind, Dot, DotKind};
use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::card::Card;
use crate::components::donate_button::DonateButton;
use crate::components::icon::{Icon, IconView};
use crate::components::language_selector::LanguageSelector;
use crate::components::segmented::Segmented;
use crate::components::theme_toggle::ThemeToggle;
use crate::language::Language;
use crate::nav::DashboardSection;
use crate::operations_view::operations_section;
use crate::state::AppState;
use crate::views::audit_view;
use dioxus::prelude::*;

/// Operations panel: context-bar, section navigation, dispatcher.
#[component]
pub fn dashboard_screen() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let has_session = state.read().session_ok;
    let base_url = state.read().base_url.clone();
    // Read the language — to re-render localized strings when the language changes.
    let language = state.read().language;
    let error_msg = state.read().error_msg.clone();
    let section = state.read().section;

    rsx! {
        div { class: "dashboard",
            // Contextual header below the titlebar. Passing `language` (not the
            // whole state) re-renders the bar when the language flips; the
            // theme is read inside ThemeToggle itself.
            {context_bar(state, has_session, &base_url, language)}

            div { class: "dashboard-body",
                // Top-level navigation.
                div { class: "section-nav",
                    Segmented {
                        current: section,
                        options: DashboardSection::options(),
                        disabled: state.read().op_status == crate::state::OpStatus::Running,
                        onchange: move |s| {
                            state.write().section = s;
                        },
                    }
                }

                // Section dispatcher.
                {section_body(state, section)}

                // Error as a highlighted block (mirrors toasts in case of long messages).
                {error_msg.as_ref().map_or_else(
                    || rsx! {},
                    |err| rsx! {
                        div { class: "op-running error-banner",
                            IconView { icon: Icon::Alert, class: "toast-icon".to_string() }
                            "{err}"
                        }
                    },
                )}
            }
        }

        // Master password entry/creation modal (audit unlock).
        {crate::master_password_modal::master_password_modal(state)}
    }
}

/// Contextual header: session badge, URL, language, theme, logout.
fn context_bar(
    mut state: Signal<AppState>,
    has_session: bool,
    base_url: &str,
    language: Language,
) -> Element {
    rsx! {
        div { class: "context-bar",
            div { class: "context-bar-left",
                Badge {
                    kind: if has_session { BadgeKind::Success } else { BadgeKind::Default },
                    Dot { kind: if has_session { DotKind::Success } else { DotKind::Muted } }
                    if has_session {
                        {tr!("session.active")}
                    } else {
                        {tr!("session.none")}
                    }
                }
                if has_session {
                    span { class: "muted mono", "{base_url}" }
                }
            }
            span { class: "context-spacer" }
            // Language selector + theme toggle (shared components with the login screen).
            LanguageSelector { current: language, state: state }
            ThemeToggle { class: "btn btn-ghost btn-icon".to_string(), state: state }
            DonateButton { state: state }
            Button {
                kind: ButtonKind::Ghost,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Logout),
                onclick: move |_| {
                    // Logout: clear the session and return to the login screen.
                    let mut s = state.write();
                    s.reset_session();
                    s.error_msg = None;
                },
                {tr!("logout")}
            }
        }
    }
}

/// Section body: dispatches to operations/audit.
/// Default `Operations` → cards for CSV/operations/result.
fn section_body(state: Signal<AppState>, section: DashboardSection) -> Element {
    match section {
        DashboardSection::Operations => rsx! { {operations_section(state)} },
        DashboardSection::Audit => rsx! {
            div { class: "dash-grid",
                Card { class: "span-all".to_string(), data_card: "audit".to_string(),
                    h2 { IconView { icon: Icon::Shield } {tr!("nav.audit")} }
                    audit_view {}
                }
            }
        },
    }
}
