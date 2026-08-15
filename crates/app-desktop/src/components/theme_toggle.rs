//! Shared theme-toggle button (single source for the flip handler).
//!
//! Previously the same onclick body (toggle → apply → persist) was copy-pasted
//! in dashboard.rs and login.rs with only the CSS class differing — the
//! comment in login.rs literally said "Mirrors dashboard.rs".
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::state::AppState;
use crate::theme::Theme;
use dioxus::prelude::*;

/// Icon button that flips dark↔light: applies the theme to the document and
/// persists it to config.toml. `class` selects the per-screen styling
/// (`"theme-toggle"` on the login screen, `"btn btn-ghost btn-icon"` in the
/// dashboard context bar).
#[component]
pub fn ThemeToggle(class: String, state: Signal<AppState>) -> Element {
    crate::i18n::subscribe_to_language(state);
    let theme = state.read().theme;
    rsx! {
        button {
            class: "{class}",
            title: if theme == Theme::Dark { tr!("theme.light") } else { tr!("theme.dark") },
            "aria-label": tr!("theme.toggle"),
            onclick: move |_| {
                let new_theme = theme.toggle();
                state.write().theme = new_theme;
                crate::theme::apply_theme(new_theme);
                crate::settings::save_theme(new_theme.as_str());
            },
            crate::components::icon::IconView {
                icon: if theme == Theme::Dark {
                    crate::components::icon::Icon::Sun
                } else {
                    crate::components::icon::Icon::Moon
                }
            }
        }
    }
}
