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
use crate::components::icon::{Icon, IconView};
use crate::components::language_menu::LanguageMenu;
use crate::components::segmented::Segmented;
use crate::language::Language;
use crate::nav::DashboardSection;
use crate::operations_view::operations_section;
use crate::settings;
use crate::state::AppState;
use crate::theme::Theme;
use crate::views::audit_view;
use dioxus::prelude::*;

/// Operations panel: context-bar, section navigation, dispatcher.
#[component]
pub fn dashboard_screen() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    let has_session = state.read().session_ok;
    let base_url = state.read().base_url.clone();
    let theme = state.read().theme;
    // Read the language — to re-render localized strings when the language changes.
    let language = state.read().language;
    let error_msg = state.read().error_msg.clone();
    let section = state.read().section;

    rsx! {
        div { class: "dashboard",
            // Contextual header below the titlebar.
            {context_bar(state, has_session, &base_url, theme, language)}

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
    theme: Theme,
    language: Language,
) -> Element {
    let theme_title = if theme == Theme::Dark {
        tr!("theme.light")
    } else {
        tr!("theme.dark")
    };
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
            // Language selector (compact dropdown). The current language is in `language`.
            {language_selector(state, language)}
            button {
                class: "btn btn-ghost btn-icon",
                title: "{theme_title}",
                "aria-label": tr!("theme.toggle"),
                onclick: move |_| {
                    let new_theme = theme.toggle();
                    state.write().theme = new_theme;
                    crate::theme::apply_theme(new_theme);
                    settings::save_theme(new_theme.as_str());
                },
                IconView { icon: if theme == Theme::Dark { Icon::Sun } else { Icon::Moon } }
            }
            Button {
                kind: ButtonKind::Ghost,
                size: ButtonSize::Small,
                icon_left: Some(Icon::Heart),
                // External link: Dioxus/wry hands it to the system browser.
                onclick: move |_| {
                    if let Err(e) =
                        dioxus::desktop::use_window().webview
                            .evaluate_script("window.open('https://donatello.to/VladymyrM','_blank','noopener');")
                    {
                        tracing::warn!("opening donate link: {e}");
                    }
                },
                {tr!("donate.label")}
            }
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

/// Compact language selector (dropdown with flags). Updates `state.language`,
/// applies the locale globally, and persists it to config.toml (mirrors the theme).
fn language_selector(mut state: Signal<AppState>, current: Language) -> Element {
    rsx! {
        LanguageMenu {
            current: current,
            onchange: move |lang: Language| {
                state.write().language = lang;
                rust_i18n::set_locale(lang.as_str());
                settings::save_language(lang.as_str());
            },
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
                Card { class: "span-all".to_string(),
                    h2 { IconView { icon: Icon::Shield } {tr!("nav.audit")} }
                    audit_view {}
                }
            }
        },
    }
}
