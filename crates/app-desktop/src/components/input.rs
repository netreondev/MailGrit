//! Input fields: the Field wrapper (label + hint) and TextField (with icon).

use super::icon::{Icon, IconView};
use dioxus::prelude::*;

/// Text input field with an optional left icon.
#[component]
pub fn TextField(
    /// Current value.
    value: String,
    /// Placeholder.
    #[props(default)]
    placeholder: String,
    /// Input type (defaults to text).
    #[props(default = String::from("text"))]
    r#type: String,
    /// Left icon.
    icon: Option<Icon>,
    /// Disabled.
    #[props(default)]
    disabled: bool,
    /// Input handler.
    oninput: Option<EventHandler<FormEvent>>,
    /// Extra CSS classes.
    #[props(default)]
    class: String,
) -> Element {
    let input_el = rsx! {
        input {
            class: "input {class}",
            r#type,
            value: "{value}",
            placeholder: "{placeholder}",
            disabled,
            oninput: move |e| {
                if let Some(handler) = &oninput {
                    handler.call(e);
                }
            },
        }
    };

    match icon {
        Some(ic) => rsx! {
            div { class: "input-wrap",
                IconView { icon: ic }
                {input_el}
            }
        },
        None => rsx! { {input_el} },
    }
}

/// Field wrapper: label + content + hint/error.
#[component]
pub fn Field(
    /// Field label.
    #[props(default)]
    label: String,
    /// Hint below the field.
    #[props(default)]
    hint: String,
    /// Error message (when present, triggers highlight).
    #[props(default)]
    error: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "field",
            if !label.is_empty() {
                label { class: "field-label", "{label}" }
            }
            {children}
            if !error.is_empty() {
                span { class: "field-error", "{error}" }
            } else if !hint.is_empty() {
                span { class: "field-hint", "{hint}" }
            }
        }
    }
}
