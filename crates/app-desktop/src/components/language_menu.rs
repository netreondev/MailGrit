//! Language selector — a modern dropdown with flags.
//!
//! A replacement for the native `<select>`: a compact trigger (flag + language
//! code + chevron) opens a dropdown menu of items (flag + endonym + checkmark
//! for the current one). Closes on click outside the menu or on Escape.
//!
//! Context: used in the dashboard context-bar and on the login screen.
//!
//! DOM structure (critical for positioning): the overlay and dropdown are
//! siblings (children of `.lang-menu`) — see the comment in the component body.

use super::icon::{Icon, IconSize, IconView};
use crate::language::Language;
use dioxus::prelude::*;

/// Language selector (dropdown). The current language comes in via `current`;
/// selection is reported via `onchange`.
///
/// `extra_class` — extra class for positioning (e.g. `login-lang-menu` to pin
/// it to the corner of the login screen).
#[component]
pub fn LanguageMenu(
    /// Currently selected language.
    current: Language,
    /// Selection handler (applies the locale, persists it, etc.).
    onchange: EventHandler<Language>,
    /// Extra CSS class (positioning).
    #[props(default)]
    extra_class: String,
) -> Element {
    // Local open/closed state — a Dioxus signal in the component scope.
    let mut open = use_signal(|| false);

    rsx! {
        div {
            class: "lang-menu {extra_class}",
            // Trigger: flag + language code (uppercase) + chevron.
            button {
                class: "lang-trigger",
                r#type: "button",
                "aria-haspopup": "listbox",
                "aria-expanded": "{open()}",
                "aria-label": tr!("lang.label"),
                title: tr!("lang.label"),
                onclick: move |_| {
                    let new_open = !*open.read();
                    open.set(new_open);
                },
                span { class: "lang-flag", "{current.flag()}" }
                span { class: "lang-code", "{current.as_str().to_uppercase()}" }
                IconView {
                    icon: Icon::ChevronRight,
                    size: IconSize::Small,
                    class: "lang-chevron".to_string(),
                }
            }

            // Menu — only when open.
            //
            // IMPORTANT (DOM structure): `.lang-overlay` and `.lang-dropdown` are
            // siblings (both direct children of `.lang-menu`), NOT nested inside
            // each other. If the dropdown were nested inside the overlay, the
            // containing block for `position: absolute; top: 100%` would become
            // the full-screen overlay (`position: fixed; inset: 0`), and the menu
            // would slide past the bottom edge of the viewport (100% of the screen
            // height). Siblings + z-index (dropdown 51 > overlay 50) → the
            // dropdown is anchored to `.lang-menu` (relative), clicks on items
            // reach their handlers, and a click outside — on the overlay — closes
            // the menu.
            if *open.read() {
                // Transparent full-screen layer — catches clicks outside the menu to close it.
                div {
                    class: "lang-overlay",
                    onclick: move |_| open.set(false),
                }
                // Dropdown menu card (premium, with shadow).
                div {
                    class: "lang-dropdown",
                    role: "listbox",
                    onclick: move |e| e.stop_propagation(),
                    for &lang in Language::all() {
                        {
                            let is_current = lang == current;
                            let item_class = if is_current {
                                "lang-item lang-item-active"
                            } else {
                                "lang-item"
                            };
                            rsx! {
                                button {
                                    key: "{lang.as_str()}",
                                    class: "{item_class}",
                                    r#type: "button",
                                    role: "option",
                                    "aria-selected": "{is_current}",
                                    onclick: move |_| {
                                        onchange.call(lang);
                                        open.set(false);
                                    },
                                    span { class: "lang-flag", "{lang.flag()}" }
                                    span { class: "lang-name", "{lang.label()}" }
                                    if is_current {
                                        IconView {
                                            icon: Icon::Check,
                                            size: IconSize::Small,
                                            class: "lang-check".to_string(),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
