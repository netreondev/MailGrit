//! Loading spinner.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use dioxus::prelude::*;

/// Loading indicator (a spinning ring).
#[component]
pub fn Spinner(
    /// Extra CSS classes (e.g. "spinner-lg" or "spinner-on-accent").
    #[props(default)]
    class: String,
) -> Element {
    rsx! {
        span { class: "spinner {class}", "aria-hidden": "true" }
    }
}
