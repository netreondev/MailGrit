//! Loading spinner.

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
