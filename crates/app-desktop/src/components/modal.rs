//! Modal window / dialog (a replacement for the one-off confirm-box).

use super::icon::{Icon, IconView};
use dioxus::prelude::*;

/// A modal window with an overlay, title, body, and footer.
///
/// Closes on click on the backdrop. For destructive actions, use a warning icon.
#[component]
pub fn Modal(
    /// Title.
    title: String,
    /// Header icon (e.g. Alert for delete confirmation).
    icon: Option<Icon>,
    /// CSS class for the colored icon block (e.g. "modal-icon-danger").
    #[props(default)]
    icon_class: String,
    /// Close handler (backdrop click).
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| on_close.call(()),
            div {
                class: "modal",
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "{title}",
                // Stop propagation so a click on the window does not close it.
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    if let Some(icon) = icon {
                        div { class: "modal-icon {icon_class}",
                            IconView { icon: icon }
                        }
                    }
                    h3 { class: "modal-title", "{title}" }
                }
                div { class: "modal-body",
                    {children}
                }
            }
        }
    }
}

/// Modal footer (action buttons).
#[component]
pub fn ModalFooter(children: Element) -> Element {
    rsx! {
        div { class: "modal-footer",
            {children}
        }
    }
}
