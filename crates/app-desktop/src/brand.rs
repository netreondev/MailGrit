//! Branding: the application name and brand strings.
//!
//! A single source of truth for the displayed name "`MailGrit`", so there are no
//! scattered hardcoded strings in RSX and window titles.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// The product's display name. Used in the titlebar, the login hero screen, and
/// window titles. The name is intentionally NOT localized (a brand mark).
pub const APP_NAME: &str = "MailGrit";

/// Donation/support link (single source of truth — previously hardcoded in
/// both dashboard.rs and login.rs, inviting drift).
pub const DONATE_URL: &str = "https://donatello.to/VladymyrM";
