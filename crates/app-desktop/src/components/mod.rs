//! Reusable UI components of the MailGrit design system.
//!
//! All components are built on design tokens (`assets/tokens.css`) and styles
//! from `assets/components.css`. Used by the login/dashboard screens.
//!
//! Imports in main.rs go directly to submodules (e.g. `use components::button::Button`),
//! so this file contains only module declarations without aggregate `pub use`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

pub mod badge;
pub mod button;
pub mod card;
pub mod icon;
pub mod input;
pub mod language_menu;
pub mod modal;
pub mod progress;
pub mod segmented;
pub mod spinner;
pub mod titlebar;
