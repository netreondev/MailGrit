//! Custom frameless titlebar.
//!
//! The main window is opened with `with_decorations(false)` (see main.rs).
//! This component draws its own bar with a drag-region (`-webkit-app-region: drag`),
//! a brandmark, and window control buttons (minimize/maximize/close).
//!
//! Window control is done via `dioxus::desktop::use_window()` (an accessor to the wry/tao window).

use super::icon::{Icon, IconView};
use dioxus::prelude::*;

/// Custom titlebar: brandmark on the left, window control buttons on the right.
#[component]
pub fn TitleBar(
    /// Subtitle/context shown to the right of the name (e.g. current screen).
    #[props(default)]
    subtitle: String,
) -> Element {
    let window = dioxus::desktop::use_window();

    rsx! {
        div { class: "titlebar",
            div { class: "titlebar-brand",
                super::icon::Logo { class: "titlebar-logo".to_string() }
                span { class: "titlebar-name", {crate::brand::APP_NAME} }
                if !subtitle.is_empty() {
                    span { class: "titlebar-sub", "{subtitle}" }
                }
            }
            div { class: "titlebar-controls",
                button {
                    class: "win-btn",
                    title: tr!("titlebar.minimize"),
                    "aria-label": tr!("titlebar.minimize"),
                    onclick: {
                        let w = window.clone();
                        move |_| {
                            w.set_minimized(true);
                        }
                    },
                    IconView { icon: Icon::Minimize, size: super::icon::IconSize::Small }
                }
                button {
                    class: "win-btn",
                    title: tr!("titlebar.maximize"),
                    "aria-label": tr!("titlebar.maximize"),
                    onclick: {
                        let w = window.clone();
                        move |_| {
                            // Toggle maximize/restore.
                            w.set_maximized(!w.is_maximized());
                        }
                    },
                    IconView { icon: Icon::Maximize, size: super::icon::IconSize::Small }
                }
                button {
                    class: "win-btn win-btn-close",
                    title: tr!("titlebar.close"),
                    "aria-label": tr!("titlebar.close"),
                    onclick: move |_| {
                        window.close();
                    },
                    IconView { icon: Icon::X, size: super::icon::IconSize::Small }
                }
            }
        }
    }
}
