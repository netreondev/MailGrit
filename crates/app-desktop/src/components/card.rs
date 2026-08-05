//! Card — the primary surface container.

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
    children: Element,
) -> Element {
    let tight_class = if tight { " card-pad-tight" } else { "" };
    rsx! {
        section { class: "card{tight_class} {class}",
            {children}
        }
    }
}
