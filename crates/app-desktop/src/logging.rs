//! Initializes debug-level logging to a file.
//!
//! The log is written to `mailgrit.log` in the application data directory
//! (`mailgrit-data/` next to the binary — portable mode). By default the level is
//! `info` for MailGrit and `warn` for dependencies; it is overridden by the
//! `RUST_LOG` environment variable (e.g. `RUST_LOG=debug`).
//!
//! The returned [`LogGuard`] must be kept alive until the end of the program
//! (drop drops the non-blocking writer and flushes the buffer).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes logging to a file. Returns the guard (keep until the end of main).
///
/// # Panics
///
/// Never panics: on a directory/file creation error it logs to stderr and
/// returns without file output (the application keeps running).
#[must_use]
pub fn init() -> Option<WorkerGuard> {
    let log_dir = log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        // Emitted BEFORE the subscriber is initialized, so tracing is not yet
        // available. Write directly to stderr via Write (not eprintln!, which is
        // denied by the print_stderr lint).
        use std::io::Write as _;
        let _ = writeln!(
            std::io::stderr(),
            "MailGrit: failed to create log directory {}: {e}",
            log_dir.display()
        );
        return None;
    }
    let file_appender = tracing_appender::rolling::never(&log_dir, "mailgrit.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Filter: by default debug for MailGrit crates, warn for dependencies.
    // RUST_LOG overrides it completely.
    //
    // IMPORTANT: crate target names are `mailgrit_app_desktop`, `mailgrit_core_*`
    // (crate_name with an underscore, not a hyphen), so the directive `mailgrit=debug`
    // will NOT match (there is no `::` separator). We use the glob `mailgrit_*=debug`,
    // which covers all our crates; `wry=info` keeps webview logs at info level.
    let default_filter = EnvFilter::try_new("warn,mailgrit_*=debug,wry=info").unwrap_or_default();
    let filter = EnvFilter::try_from_default_env().unwrap_or(default_filter);

    let result = tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .try_init();

    if result.is_err() {
        // Subscriber already installed (e.g. dioxus-logger) — not critical.
        // Written to stderr via Write (not eprintln!, which is denied by print_stderr).
        use std::io::Write as _;
        let _ = writeln!(
            std::io::stderr(),
            "MailGrit: tracing subscriber already initialized, file logging may be inactive"
        );
    } else {
        tracing::info!(
            "=== MailGrit started; log: {}/mailgrit.log ===",
            log_dir.display()
        );
    }
    Some(guard)
}

/// Directory for the log file — next to the binary (portability).
fn log_dir() -> PathBuf {
    crate::app_data_dir()
}
