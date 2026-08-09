//! Build script: embeds the application icon and metadata into the `.exe`
//! (Windows).
//!
//! On Windows the icon shown in Explorer/taskbar/alt-tab is taken from a PE-file
//! resource, not from the runtime `set_window_icon`. So we compile a `.rc` (via
//! `winres`) with an `IDI_APPLICATION`-style reference to the multi-resource
//! `mailgrit.ico`.
//!
//! On macOS/Linux this script is a no-op: the window icon is set by `tao` at
//! runtime via `.with_window_icon(...)` (see `window_icon.rs`).
//!
//! Note: Cargo runs build.rs with CWD = crate root, so paths are relative
//! (`assets/icons/mailgrit.ico`).

fn main() {
    // Rebuild when the icon/resource changes.
    println!("cargo:rerun-if-changed=assets/icons/mailgrit.ico");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icons/mailgrit.ico");
        res.set(
            "FileDescription",
            "MailGrit - bulk automation for iRedAdmin",
        );
        res.set("ProductName", "MailGrit");
        res.set("OriginalFilename", "mailgrit-app-desktop.exe");
        res.set(
            "LegalCopyright",
            "(c) 2026 netreon and contributors (MIT OR Apache-2.0)",
        );
        if let Err(err) = res.compile() {
            // winres requires the Windows SDK (rc.exe / windres). If the toolchain
            // is incomplete, do not fail the build: the runtime icon is still set
            // via with_window_icon. Log a warning to the build output.
            println!(
                "cargo:warning=winres: failed to embed the icon into the .exe ({err}). \
                 The window icon will be set at runtime via with_window_icon."
            );
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows a resource cannot be embedded into the binary; the window
        // icon is set at runtime (tao). Nothing to do here, but Cargo needs the
        // build.rs entry point.
    }
}
