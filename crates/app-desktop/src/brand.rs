//! Branding: the application name and brand strings.
//!
//! A single source of truth for the displayed name "MailGrit", so there are no
//! scattered hardcoded strings in RSX and window titles.

/// The product's display name. Used in the titlebar, the login hero screen, and
/// window titles. The name is intentionally NOT localized (a brand mark).
pub const APP_NAME: &str = "MailGrit";
