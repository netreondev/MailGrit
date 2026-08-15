//! Premium login screen: a hero composition with auth auto-polling.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::card::Card;
use crate::components::icon::{Icon, IconSize, IconView, Logo};
use crate::components::input::{Field, TextField};
use crate::components::language_selector::LanguageSelector;
use crate::components::spinner::Spinner;
use crate::components::theme_toggle::ThemeToggle;
use crate::state::{AppState, AuthStatus};
use crate::util::validate_base_url;
use crate::views::cookies_disclosure;
use crate::{auth_bridge, login_window};
use dioxus::prelude::*;

/// Premium login screen: a hero composition with auth auto-polling.
#[component]
pub fn login_screen() -> Element {
    let mut state = use_context::<Signal<AppState>>();
    // Arc clone for the login-window open closure (thread_local — shared).
    let login_state_for_open = login_window::login_state();
    let url = state.read().url_input.clone();
    let auth_status = state.read().auth_status;
    let error_msg = state.read().error_msg.clone();
    // Read the language — to re-render localized strings when the language changes.
    let language = state.read().language;

    rsx! {
        div { class: "login-screen",
            div { class: "login-card",
                // Branding (symmetric, centered).
                div { class: "login-brand",
                    Logo { class: "login-logo".to_string() }
                    h1 { {crate::brand::APP_NAME} }
                    p { class: "login-tagline", {tr!("login.tagline")} }
                    p { class: "login-notice", {tr!("login.authorized_notice")} }
                }

                Card { class: "card-accent".to_string(),
                    // Server URL field.
                    Field {
                        label: tr!("login.server_label"),
                        hint: tr!("login.server_hint"),
                        TextField {
                            value: url,
                            r#type: "url".to_string(),
                            placeholder: "https://mail.example.com/iredadmin".to_string(),
                            icon: Some(Icon::Link),
                            disabled: auth_status == AuthStatus::AwaitingLogin,
                            oninput: move |e: FormEvent| state.write().url_input = e.value(),
                        }
                    }

                    // Button to open the login form → opens the iRedAdmin window.
                    // Auth is detected data-driven: via the webview navigation
                    // event (navigation_handler), not by timer-based polling.
                    div { class: "login-actions",
                        Button {
                            kind: ButtonKind::Primary,
                            size: ButtonSize::Large,
                            icon_right: Some(Icon::ChevronRight),
                            disabled: auth_status == AuthStatus::AwaitingLogin,
                            onclick: move |_| {
                                let base = state.read().url_input.clone();
                                match validate_base_url(&base) {
                                    Ok(()) => {
                                        tracing::info!("URL validation passed: {base}");
                                        // Open the login window and switch to waiting.
                                        // The transition to the dashboard happens
                                        // automatically on the iRedAdmin navigation
                                        // event after login.
                                        {
                                            let mut s = state.write();
                                            s.base_url.clone_from(&base);
                                            s.error_msg = None;
                                            s.auth_status = AuthStatus::AwaitingLogin;
                                        }
                                        auth_bridge::request_login_window(&login_state_for_open, &base);
                                    }
                                    Err(e) => {
                                        tracing::warn!("URL validation failed: {e}");
                                        let msg = crate::error_i18n::url_error(&e);
                                        state.write().error_msg = Some(msg);
                                    }
                                }
                            },
                            {tr!("login.open_form")}
                        }
                    }

                    // Waiting-for-login state: the iRedAdmin window is open, waiting for navigation.
                    {match auth_status {
                        AuthStatus::AwaitingLogin => rsx! {
                            div { class: "poll-banner",
                                Spinner {}
                                div { class: "poll-banner-text",
                                    {tr!("login.waiting")}
                                    div { class: "muted",
                                        {tr!("login.waiting_hint")}
                                    }
                                }
                            }
                        },
                        AuthStatus::None | AuthStatus::Connected => rsx! {},
                    }}

                    // Error (if any).
                    {error_msg.as_ref().map_or_else(
                        || rsx! {},
                        |err| rsx! {
                            div { class: "poll-banner error-banner",
                                IconView { icon: Icon::Alert, class: "toast-icon".to_string() }
                                div { class: "poll-banner-text", "{err}" }
                            }
                        },
                    )}

                    // Cookie diagnostics — collapsible (hidden by default).
                    {cookies_disclosure(&state)}
                }

                // Privacy.
                p { class: "login-footer",
                    IconView { icon: Icon::Lock, size: IconSize::Small }
                    {tr!("login.footer")}
                }
                // Donate / support link — opens in the system browser (Dioxus
                // hands external URLs to the OS by default).
                a {
                    class: "login-donate",
                    href: crate::brand::DONATE_URL,
                    IconView { icon: Icon::Heart, size: IconSize::Small }
                    {tr!("donate.label")}
                }
            }
            // Language selector + theme toggle — tucked into the top-right corner
            // (shared components with the dashboard context bar).
            LanguageSelector {
                current: language,
                extra_class: "login-lang-menu".to_string(),
                state: state
            }
            ThemeToggle { class: "theme-toggle".to_string(), state: state }
        }
    }
}
