//! Design system button: variants, sizes, icons, loading state.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use super::icon::{Icon, IconSize, IconView};
use super::spinner::Spinner;
use dioxus::prelude::*;

/// Button variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonKind {
    /// Gradient accent (primary action).
    #[default]
    Primary,
    /// Surface + border.
    Secondary,
    /// Transparent (secondary).
    Ghost,
    /// Destructive action.
    Danger,
}

/// Button size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    #[default]
    Regular,
    Small,
    Large,
}

impl ButtonSize {
    /// Size CSS class.
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Regular => "",
            Self::Small => " btn-sm",
            Self::Large => " btn-lg",
        }
    }

    /// Inner icon size.
    #[must_use]
    pub const fn icon_size(self) -> IconSize {
        match self {
            Self::Small => IconSize::Small,
            _ => IconSize::Regular,
        }
    }
}

impl ButtonKind {
    /// Variant CSS class.
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Primary => "btn btn-primary",
            Self::Secondary => "btn btn-secondary",
            Self::Ghost => "btn btn-ghost",
            Self::Danger => "btn btn-danger",
        }
    }
}

/// Design system button.
///
/// Supports variants (`kind`), sizes (`size`), a left icon (`icon_left`),
/// a right icon (`icon_right`), and a loading state (`loading` — shows a
/// spinner instead of the left icon and disables the button).
#[component]
pub fn Button(
    /// Variant.
    #[props(default)]
    kind: ButtonKind,
    /// Size.
    #[props(default)]
    size: ButtonSize,
    /// Left icon (hidden while loading).
    icon_left: Option<Icon>,
    /// Right icon.
    icon_right: Option<Icon>,
    /// Loading state: shows a spinner and blocks clicks.
    #[props(default)]
    loading: bool,
    /// Disabled.
    #[props(default)]
    disabled: bool,
    /// Extra CSS classes (e.g. "btn-icon" for a square button).
    #[props(default)]
    class: String,
    /// Click handler.
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let is_disabled = disabled || loading;
    let base = format!(
        "{kind_class}{size_class} {extra}",
        kind_class = kind.class(),
        size_class = size.class(),
        extra = class
    );
    // The primary and danger variants use a white spinner.
    let spinner_class = if kind == ButtonKind::Primary || kind == ButtonKind::Danger {
        "spinner spinner-on-accent"
    } else {
        "spinner"
    };

    rsx! {
        button {
            class: "{base.trim()}",
            disabled: is_disabled,
            onclick: move |e| {
                if let Some(handler) = &onclick {
                    handler.call(e);
                }
            },
            if loading {
                Spinner { class: spinner_class }
            } else if let Some(icon) = icon_left {
                IconView { icon: icon, size: size.icon_size() }
            }
            {children}
            if let Some(icon) = icon_right {
                if !loading {
                    IconView { icon: icon, size: size.icon_size() }
                }
            }
        }
    }
}
