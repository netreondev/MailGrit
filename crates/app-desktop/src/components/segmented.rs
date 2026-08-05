//! Segmented control (switch between mutually exclusive options).

use super::icon::{Icon, IconView};
use dioxus::prelude::*;

/// A single option of the segmented control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedOption<T: Clone + PartialEq> {
    /// The value this option represents.
    pub value: T,
    /// Text label.
    pub label: String,
    /// Optional icon.
    pub icon: Option<Icon>,
}

/// Segmented control — a premium replacement for a group of toggle buttons.
///
/// Generic over the value type `T` (e.g. `OperationTarget`, `DashboardSection`).
#[component]
pub fn Segmented<T>(
    /// Currently selected value.
    current: T,
    /// List of options.
    options: Vec<SegmentedOption<T>>,
    /// Disabled.
    #[props(default)]
    disabled: bool,
    /// Selection handler (receives the selected value).
    onchange: EventHandler<T>,
) -> Element
where
    T: Clone + PartialEq + 'static,
{
    rsx! {
        div { class: "segmented", role: "radiogroup",
            for opt in options.iter() {
                {
                    let value = opt.value.clone();
                    let active = value == current;
                    let btn_class = if active {
                        "segmented-btn segmented-active"
                    } else {
                        "segmented-btn"
                    };
                    let opt_icon = opt.icon;
                    let opt_label = opt.label.clone();
                    rsx! {
                        button {
                            key: "{opt_label}",
                            class: "{btn_class}",
                            role: "radio",
                            "aria-checked": "{active}",
                            disabled: disabled || active,
                            onclick: move |_| onchange.call(value.clone()),
                            if let Some(icon) = opt_icon {
                                IconView { icon: icon, size: super::icon::IconSize::Small }
                            }
                            "{opt_label}"
                        }
                    }
                }
            }
        }
    }
}
