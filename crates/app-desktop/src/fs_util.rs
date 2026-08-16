//! Durable file-replacement primitives (temp file + fsync + rename).
//!
//! `std::fs::write` truncates the destination first: a crash mid-write leaves
//! a half-written file. For files whose loss bricks a feature (the audit key
//! file — a truncated key makes the whole audit history unverifiable) the
//! write must be atomic: write to a uniquely named temp file in the SAME
//! directory, fsync it, then `rename` over the destination (atomic on Unix;
//! `MOVEFILE_REPLACE_EXISTING` on Windows).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Atomically replaces `path`'s content with `bytes`.
///
/// Guarantees: the destination is either the OLD complete file or the NEW
/// complete file — never a truncated mix. A crash can only leave the temp
/// file behind (picked up by the next attempt's stale-temp handling).
///
/// # Errors
///
/// - [`std::io::Error`] — as returned by the underlying file operations.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = temp_sibling(path);
    let file = match OpenOptions::new().write(true).create_new(true).open(&tmp) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // A stale temp from an earlier crash — replace it and retry once.
            let _ = fs::remove_file(&tmp);
            OpenOptions::new().write(true).create_new(true).open(&tmp)?
        }
        Err(e) => return Err(e),
    };
    let result = write_sync_rename(file, &tmp, path, bytes);
    if result.is_err() {
        // Best-effort cleanup so a failed attempt does not leave the temp behind.
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Unique temp-file path next to `path` (same directory → same volume → the
/// final `rename` is atomic). Uniqueness: target name + pid; concurrent
/// processes get different pids, and a same-pid collision is handled by the
/// stale-temp retry in [`atomic_write`].
fn temp_sibling(path: &Path) -> PathBuf {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path.file_name().map_or_else(
        || std::borrow::Cow::Borrowed("file"),
        |s| s.to_string_lossy(),
    );
    dir.join(format!(".{name}.tmp-{}", std::process::id()))
}

fn write_sync_rename(file: fs::File, tmp: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = file;
    file.write_all(bytes)?;
    // Flush the OS buffer before the rename: without fsync the rename could
    // land while the content is still only in the page cache.
    file.sync_all()?;
    drop(file);
    #[cfg(unix)]
    restrict_permissions(tmp)?;
    fs::rename(tmp, path)
}

/// 0600 on Unix — the files written this way are secret-bearing (audit key).
#[cfg(unix)]
fn restrict_permissions(tmp: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(tmp, fs::Permissions::from_mode(0o600))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Same TempDir approach as audit_ui tests (no external tempfile dep).
    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Result<Self, std::io::Error> {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            // Rationale: test-only scratch directory with a pid+counter
            // unique name; no security decision is made on this path.
            let dir = std::env::temp_dir() // nosemgrep: rust.lang.security.temp-dir.temp-dir
                .join(format!("mailgrit-fsutil-test-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&dir)?;
            Ok(Self(dir))
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn writes_new_file_with_exact_content() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let target = dir.path().join("data.bin");
        atomic_write(&target, b"hello")?;
        assert_eq!(fs::read(&target)?, b"hello");
        Ok(())
    }

    #[test]
    fn replaces_existing_file_completely() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let target = dir.path().join("data.bin");
        fs::write(&target, b"old-content")?;
        atomic_write(&target, b"new")?;
        assert_eq!(fs::read(&target)?, b"new");
        Ok(())
    }

    #[test]
    fn leaves_no_temp_files_behind() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let target = dir.path().join("key.file");
        atomic_write(&target, b"0123456789")?;
        let leftovers: Vec<_> = fs::read_dir(dir.path())?
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            leftovers,
            vec!["key.file".to_string()],
            "only the target remains"
        );
        Ok(())
    }

    #[test]
    fn recovers_from_a_stale_temp_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let target = dir.path().join("data.bin");
        // Simulate a crash leftover: the exact temp name atomic_write would pick.
        let stale = temp_sibling(&target);
        fs::write(&stale, b"garbage-from-a-crash")?;
        atomic_write(&target, b"fresh")?;
        assert_eq!(fs::read(&target)?, b"fresh");
        Ok(())
    }
}
