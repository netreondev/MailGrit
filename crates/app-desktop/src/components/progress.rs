//! Progress bar (determinate and indeterminate).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use dioxus::prelude::*;

/// Progress bar.
///
/// With `indeterminate = true`, shows an animated stripe (for operations with
/// no known percentage). Otherwise, shows a bar `value %` long (0..=100).
#[component]
pub fn Progress(
    /// 0..=100 when `indeterminate = false`.
    #[props(default = 0)]
    value: u32,
    /// Indeterminate mode (animated stripe).
    #[props(default)]
    indeterminate: bool,
    /// Extra CSS classes.
    #[props(default)]
    class: String,
) -> Element {
    let bar_class = if indeterminate {
        "progress-bar indeterminate"
    } else {
        "progress-bar"
    };
    let style = if indeterminate {
        String::new()
    } else {
        format!("width: {}%", value.min(100))
    };
    rsx! {
        div { class: "progress {class}", role: "progressbar",
            "aria-valuenow": "{value}",
            div { class: "{bar_class}", style: "{style}" }
        }
    }
}
