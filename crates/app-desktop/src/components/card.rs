//! Card — the primary surface container.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use dioxus::prelude::*;

/// Design system card.
#[component]
pub fn Card(
    /// Extra CSS classes (e.g. "card-accent", "card-hover", "span-all").
    #[props(default)]
    class: String,
    /// Tight padding.
    #[props(default)]
    tight: bool,
    /// Stable semantic identifier rendered as `data-card="…"` (e.g. "csv",
    /// "ops", "result", "audit"). Used by the E2E selectors: positional
    /// `:nth-child` selectors silently retarget onto the WRONG card when the
    /// grid is reordered, while a missing `data-card` fails loudly. Empty on
    /// cards that no test addresses.
    #[props(default)]
    data_card: String,
    children: Element,
) -> Element {
    let tight_class = if tight { " card-pad-tight" } else { "" };
    let class_attr = format!("card{tight_class} {class}");
    // The attribute is only rendered when set: a bare `data-card=""` on every
    // un-targeted card would make a bare `[data-card]` selector match all of
    // them — the opposite of the fail-loudly contract above.
    if data_card.is_empty() {
        rsx! {
            section { class: "{class_attr}",
                {children}
            }
        }
    } else {
        rsx! {
            section { class: "{class_attr}", "data-card": "{data_card}",
                {children}
            }
        }
    }
}
