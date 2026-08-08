//! Application infrastructure: the data directory and the global tokio runtime.

/// Application data directory — **next to the binary** (portability).
/// All files (logs, config, audit, dumps, cookie-store) live in the `mailgrit-data`
/// folder next to the executable. Convenient for porting: copy the binary + the folder.
pub fn app_data_dir() -> std::path::PathBuf {
    // Path of the executable -> sibling mailgrit-data folder.
    // Fallback (None): mailgrit-data in the current directory.
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
        .map_or_else(
            || std::path::PathBuf::from("mailgrit-data"),
            |exe| exe.join("mailgrit-data"),
        )
}

/// Global tokio runtime. Stored in a `static OnceLock`, NOT in `AppState`:
/// on application exit, a runtime in `AppState` would be dropped inside the
/// asynchronous event-loop context, causing a panic "Cannot drop a runtime in a
/// context where blocking is not allowed". The global runtime is NOT dropped
/// (the process exits, the OS cleans up) — no panic.
static TOKIO_RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// Returns the global tokio runtime (created on first call).
//
// Justification for the suppression (spec §Clippy — "make your case to the prosecutor"):
// this is the critical-error path BEFORE std::process::exit — the tracer (tracing)
// is not initialized yet (this call happens before logging::init in the early
// stack), so stderr is the only channel to report the fatal error to the user.
// There is no alternative: panic is forbidden (spec §Clippy), and the logger is
// unavailable here.
#[expect(
    clippy::print_stderr,
    reason = "fatal error before exit; the tracer is not initialized yet"
)]
pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        // multi-thread; on failure, current-thread. If both fail, the process
        // cannot continue (bulk operations/export require a runtime).
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .or_else(|_| {
                tracing::warn!("multi-thread tokio unavailable, trying current-thread");
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
            })
            // Fundamental initialization — without a runtime the application is non-functional.
            // std::process::exit instead of panic (spec §Clippy: panic is forbidden).
            .unwrap_or_else(|e| {
                eprintln!("MailGrit: critical error — tokio runtime was not created: {e}");
                std::process::exit(1);
            })
    })
}
