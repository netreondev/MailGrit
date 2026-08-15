//! Shared "Donate" button (single source for the support/donation action).
//!
//! One component for BOTH screens — previously the dashboard had a proper
//! button while the login screen had a barely visible muted link, so the
//! affordance read as an unexplained heart glyph at the bottom.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::components::button::{Button, ButtonKind, ButtonSize};
use crate::components::icon::Icon;
use crate::state::AppState;
use dioxus::prelude::*;

/// The donation/support button: heart icon + localized label + a tooltip that
/// says WHERE the click leads. Opens the page in the SYSTEM browser (never the
/// app's own webview — see [`crate::util::open_in_system_browser`]).
#[component]
pub fn DonateButton(state: Signal<AppState>) -> Element {
    crate::i18n::subscribe_to_language(state);
    rsx! {
        Button {
            kind: ButtonKind::Ghost,
            size: ButtonSize::Small,
            icon_left: Some(Icon::Heart),
            title: tr!("donate.tooltip"),
            onclick: move |_| {
                crate::util::open_in_system_browser(crate::brand::DONATE_URL);
            },
            {tr!("donate.label")}
        }
    }
}
