//! Badges and status dots.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use dioxus::prelude::*;

/// Color variant of the badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BadgeKind {
    #[default]
    Default,
    Success,
}

impl BadgeKind {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Default => "badge",
            Self::Success => "badge badge-success",
        }
    }
}

/// Badge / pill (metadata: mode, session status).
#[component]
pub fn Badge(
    /// Color variant.
    #[props(default)]
    kind: BadgeKind,
    /// Extra CSS classes.
    #[props(default)]
    class: String,
    children: Element,
) -> Element {
    rsx! {
        span { class: "{kind.class()} {class}",
            {children}
        }
    }
}

/// Status dot (activity indicator).
#[component]
pub fn Dot(
    /// Color variant.
    #[props(default = DotKind::Muted)]
    kind: DotKind,
    /// Extra CSS classes.
    #[props(default)]
    class: String,
) -> Element {
    rsx! {
        span { class: "dot {kind.class()} {class}", "aria-hidden": "true" }
    }
}

/// Color variant of the status dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DotKind {
    #[default]
    Muted,
    Success,
}

impl DotKind {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Muted => "",
            Self::Success => "dot-success",
        }
    }
}
