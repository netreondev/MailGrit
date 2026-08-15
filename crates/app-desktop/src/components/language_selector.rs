//! Shared language selector wrapper (single source for the switch handler).
//!
//! Both the login screen and the dashboard context bar used to hand-roll the
//! same `LanguageMenu` wiring (set state → set global locale → persist); only
//! the positioning class differed.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::language_menu::LanguageMenu;
use crate::language::Language;
use crate::state::AppState;
use dioxus::prelude::*;

/// Compact language dropdown. `extra_class` positions it on the screen
/// (`"login-lang-menu"` pins it to the login screen corner).
#[component]
pub fn LanguageSelector(
    current: Language,
    #[props(default)] extra_class: String,
    state: Signal<AppState>,
) -> Element {
    rsx! {
        LanguageMenu {
            current: current,
            extra_class: extra_class,
            onchange: move |lang: Language| {
                state.write().language = lang;
                rust_i18n::set_locale(lang.as_str());
                crate::settings::save_language(lang.as_str());
            },
        }
    }
}
