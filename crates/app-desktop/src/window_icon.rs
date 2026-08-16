//! Application window icon at runtime.
//!
//! Decodes the embedded `icon-64.png` (RGBA) into a `tao::icon::Icon` to pass to
//! `WindowBuilder::with_window_icon`. Used by both windows (main + login) so the
//! brand icon appears in the taskbar/alt-tab/window bar on all OSes.
//!
//! On Windows this icon duplicates the `.exe` resource from `build.rs` (winres):
//! wry/tao shows it for the webview window itself, while the PE resource is for
//! shortcuts and Explorer. 64x64 is chosen as a balance of quality and RGBA
//! buffer weight (16 KB): tao scales it for the needed DPI itself.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use dioxus::desktop::tao::window::{BadIcon, Icon};

/// Size of the embedded PNG (square). Must match the asset filename.
const ICON_SIZE: u32 = 64;

/// Returns the `MailGrit` brand window icon, or `None` if decoding failed.
///
/// `None` (rather than a panic) because the icon is non-critical: the window
/// opens with the default one. The decode error is logged (warn), not masked.
#[must_use]
pub fn window_icon() -> Option<Icon> {
    const PNG: &[u8] = include_bytes!("../assets/icons/icon-64.png");
    match image::load_from_memory(PNG) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            match Icon::from_rgba(rgba.into_raw(), ICON_SIZE, ICON_SIZE) {
                Ok(icon) => Some(icon),
                Err(BadIcon::ByteCountNotDivisibleBy4 { .. }) => {
                    tracing::error!("icon: RGBA buffer is not a multiple of 4 bytes");
                    None
                }
                Err(err) => {
                    tracing::error!("icon: failed to create tao::Icon ({err})");
                    None
                }
            }
        }
        Err(err) => {
            tracing::error!("icon: failed to decode icon-64.png ({err})");
            None
        }
    }
}
