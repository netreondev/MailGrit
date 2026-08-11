//! Application screens: login and the operations panel.
//!
//! Split into submodules to keep each file under the spec's 400-line limit
//! (RSX screens are structurally large). The root component `app()` in
//! `main.rs` routes between [`login::login_screen`] and [`dashboard::dashboard_screen`].
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

pub mod csv_load;
pub mod dashboard;
pub mod login;

pub use dashboard::dashboard_screen;
pub use login::login_screen;
