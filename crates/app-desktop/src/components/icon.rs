//! Inline SVG icons (Lucide style). No external crates — lightweight.
//! All icons use `currentColor` and stroke 1.75 (see `.icon` in components.css).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

use dioxus::prelude::*;

/// Available icons. Each maps to an SVG path in a 24×24 viewbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Lock.
    Lock,
    /// Log out (logout / sign-out).
    Logout,
    /// Upload.
    Upload,
    /// Plus (create).
    Plus,
    /// Edit (pencil).
    Edit,
    /// Trash.
    Trash,
    /// Shield (security/audit).
    Shield,
    /// Checkmark.
    Check,
    /// Cross / close.
    X,
    /// Minus (minimize window).
    Minimize,
    /// Square (maximize window).
    Maximize,
    /// Sun (light theme).
    Sun,
    /// Moon (dark theme).
    Moon,
    /// Right chevron.
    ChevronRight,
    /// Warning triangle.
    Alert,
    /// Search.
    Search,
    /// Link (URL).
    Link,
    /// Download (export).
    Download,
    /// Tool / wrench (diagnostics).
    Wrench,
    /// Heart (donate / support).
    Heart,
}

/// Icon size (CSS modifier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    Small,
    #[default]
    Regular,
}

impl IconSize {
    /// Size CSS class (applied to `.icon`).
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Small => "icon icon-sm",
            Self::Regular => "icon",
        }
    }
}

impl Icon {
    /// Returns the `<path>` body (and, if needed, a fill variant) for the SVG.
    /// Coordinates are in the viewBox="0 0 24 24" system.
    #[must_use]
    pub const fn paths(self) -> &'static str {
        match self {
            // Lock
            Self::Lock => {
                r#"<rect width="18" height="11" x="3" y="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>"#
            }
            // Log out
            Self::Logout => {
                r#"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>"#
            }
            // Upload
            Self::Upload => {
                r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/>"#
            }
            // Plus
            Self::Plus => r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#,
            // Edit
            Self::Edit => {
                r#"<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.12 2.12 0 0 1 3 3L12 15l-4 1 1-4Z"/>"#
            }
            // Trash
            Self::Trash => {
                r#"<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"#
            }
            // Shield
            Self::Shield => {
                r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>"#
            }
            // Checkmark
            Self::Check => r#"<path d="M20 6 9 17l-5-5"/>"#,
            // Cross
            Self::X => r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#,
            // Minus (minimize)
            Self::Minimize => r#"<path d="M5 12h14"/>"#,
            // Square (maximize)
            Self::Maximize => r#"<rect width="18" height="18" x="3" y="3" rx="2"/>"#,
            // Sun
            Self::Sun => {
                r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>"#
            }
            // Moon
            Self::Moon => r#"<path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>"#,
            // Right chevron
            Self::ChevronRight => r#"<path d="m9 18 6-6-6-6"/>"#,
            // Warning
            Self::Alert => {
                r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" x2="12" y1="9" y2="13"/><line x1="12" x2="12.01" y1="17" y2="17"/>"#
            }
            // Search
            Self::Search => r#"<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>"#,
            // Link
            Self::Link => {
                r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#
            }
            // Download
            Self::Download => {
                r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>"#
            }
            // Tool
            Self::Wrench => {
                r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"/>"#
            }
            // Heart (donate / support)
            Self::Heart => {
                r#"<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>"#
            }
        }
    }
}

/// Icon component. Renders an inline SVG.
#[component]
pub fn IconView(
    /// Which icon to render.
    icon: Icon,
    /// Size (defaults to Regular).
    #[props(default)]
    size: IconSize,
    /// Extra CSS classes.
    #[props(default)]
    class: String,
) -> Element {
    rsx! {
        svg {
            class: "{size.class()} {class}",
            view_box: "0 0 24 24",
            "aria-hidden": "true",
            dangerous_inner_html: icon.paths(),
        }
    }
}

/// The MailGrit "Forged M" logo: a monogram letter M (Mail) rendered as
/// forged/laminated metal (Grit — tempering/power/mass-production). A single
/// polyline stroke (left stem → V valley → right stem) sits on top of the
/// brand pillow-gradient, with a white-hot underlay and forge-seam notches on
/// the stems. A fully geometric mark with a premium (Linear/Vercel) aesthetic.
#[component]
pub fn Logo(#[props(default)] class: String) -> Element {
    // r##"..."## — the content contains the "# sequence (url(#...)), so a higher
    // hash level (##) is required to keep the raw string from closing prematurely.
    let logo_svg = r##"
        <defs>
            <linearGradient id="fm-logo-bg" x1="2" y1="2" x2="22" y2="22" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#1D4ED8"/>
                <stop offset="0.42" stop-color="#2563EB"/>
                <stop offset="0.78" stop-color="#0EA5E9"/>
                <stop offset="1" stop-color="#38BDF8"/>
            </linearGradient>
            <linearGradient id="fm-logo-sheen" x1="3" y1="3" x2="13" y2="14" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#ffffff" stop-opacity="0.34"/>
                <stop offset="0.55" stop-color="#ffffff" stop-opacity="0"/>
            </linearGradient>
            <linearGradient id="fm-logo-m" x1="6" y1="6" x2="18" y2="18" gradientUnits="userSpaceOnUse">
                <stop offset="0" stop-color="#ffffff"/>
                <stop offset="1" stop-color="#BAE6FD"/>
            </linearGradient>
        </defs>
        <!-- Pillow background with rounded corners: volumetric gradient + sheen. -->
        <rect x="2" y="2" width="20" height="20" rx="6" fill="url(#fm-logo-bg)"/>
        <rect x="2" y="2" width="20" height="20" rx="6" fill="url(#fm-logo-sheen)"/>
        <!-- Edge-light: a thin light inner outline on the top-left ("forged metal"). -->
        <rect x="2.6" y="2.6" width="18.8" height="18.8" rx="5.4" fill="none"
            stroke="#ffffff" stroke-opacity="0.18" stroke-width="0.7"/>
        <!-- White-hot forged glow underlay. -->
        <path d="M6 17 L6 7 L12 13 L18 7 L18 17" fill="none"
            stroke="#7DD3FC" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"
            opacity="0.55"/>
        <!-- Forged M: monogram in a single stroke (Mail) + laminated metal (Grit). -->
        <path d="M6 17 L6 7 L12 13 L18 7 L18 17" fill="none"
            stroke="url(#fm-logo-m)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
        <!-- Forge seams: notches on the outer stems (tempering/layering/mass-production). -->
        <g stroke="#1D4ED8" stroke-opacity="0.42" stroke-width="0.55" stroke-linecap="round">
            <line x1="5.0" y1="9.4" x2="7.0" y2="9.4"/>
            <line x1="5.0" y1="12.0" x2="7.0" y2="12.0"/>
            <line x1="5.0" y1="14.6" x2="7.0" y2="14.6"/>
            <line x1="17.0" y1="9.4" x2="19.0" y2="9.4"/>
            <line x1="17.0" y1="12.0" x2="19.0" y2="12.0"/>
            <line x1="17.0" y1="14.6" x2="19.0" y2="14.6"/>
        </g>
    "##;
    rsx! {
        svg {
            class: "logo {class}",
            view_box: "0 0 24 24",
            "aria-hidden": "true",
            dangerous_inner_html: logo_svg,
        }
    }
}
