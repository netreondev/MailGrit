//! UI theme (dark / light) with persistence to config.toml.
//!
//! The theme is stored in `AppState.theme` and applied to `<html>` via the
//! `data-theme="dark"|"light"` attribute (see tokens.css). The browser engine
//! itself substitutes the corresponding set of semantic tokens.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

/// Available themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// String representation for the `data-theme` attribute and the TOML config.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    /// Parses from a config string. An unknown value -> dark (the default theme).
    #[must_use]
    pub fn from_config(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    /// Inverts the theme (for the toggle).
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

/// Applies the current theme to the document by setting `data-theme` on `<html>`.
/// Called via `use_effect` on every theme change.
///
/// Dioxus 0.7: JS runs via `wry::WebView::evaluate_script` (the `webview` field
/// of `DesktopContext`, accessible via `use_window()`).
pub fn apply_theme(theme: Theme) {
    let window = dioxus::desktop::use_window();
    let js = format!(
        "document.documentElement.setAttribute('data-theme', '{}');",
        theme.as_str()
    );
    // An error is not critical — the theme will apply on a CSS re-render via the tokens.
    if let Err(e) = window.webview.evaluate_script(&js) {
        tracing::warn!("applying theme: {e}");
    }
}
