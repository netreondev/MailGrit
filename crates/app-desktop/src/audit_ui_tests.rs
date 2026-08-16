//! Unit tests moved out of the production file (the `#[path]` pattern
//! used across the workspace; keeps the prod file under the 400-line spec).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique temporary directory for a test. With no external dependency
/// (`tempfile`): pid + a monotonic counter guarantee uniqueness even under
/// parallel nextest. Cleaned up (best-effort) via `Drop`.
struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Result<Self, std::io::Error> {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        // Rationale: test-only scratch directory with a pid+counter unique
        // name; no security decision is made on this path.
        let dir = std::env::temp_dir() // nosemgrep: rust.lang.security.temp-dir.temp-dir
            .join(format!("mailgrit-audit-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// First run (no file) → a key is created, the file has the correct length.
#[test]
fn missing_key_file_creates_new_on_first_run() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let result = load_or_create_persistent_key(dir.path(), b"strong-password-123");
    assert!(result.is_ok(), "the first run must create a key");
    let file = dir.path().join(".mailgrit-audit-key");
    let data = std::fs::read(&file)?;
    assert_eq!(data.len(), AUDIT_KEY_FILE_LEN, "the file length is correct");
    Ok(())
}

// The key file must be written atomically: no temp-file leftovers, and no
// path where a crash could leave a truncated (bricked) key behind.
#[test]
fn key_creation_leaves_no_temp_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    load_or_create_persistent_key(dir.path(), b"atomic-write-pw")?;
    let entries: Vec<String> = std::fs::read_dir(dir.path())?
        .filter_map(std::result::Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries,
        vec![".mailgrit-audit-key".to_string()],
        "only the key file must remain after creation"
    );
    Ok(())
}

// A correct file → the key is derived and verified by the same password.
#[test]
fn correct_key_file_loads_successfully() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    // Create a reference file.
    load_or_create_persistent_key(dir.path(), b"correct-password-1")?;
    // Reopening with the same password must return Ok.
    let result = load_or_create_persistent_key(dir.path(), b"correct-password-1");
    assert!(result.is_ok(), "a correct file must load");
    Ok(())
}

// A wrong password on a correct file → WrongMasterPassword (constant-time).
#[test]
fn wrong_password_fails() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    load_or_create_persistent_key(dir.path(), b"correct-password-1")?;
    let result = load_or_create_persistent_key(dir.path(), b"different-password-2");
    assert!(matches!(result, Err(AuditError::WrongMasterPassword)));
    Ok(())
}

// Regression for #5: a damaged file (wrong length) is NOT silently recreated
// — CorruptedKeyFile is returned, otherwise the legitimate audit history
// would become indistinguishable from a forgery.
#[test]
fn corrupted_key_file_returns_error_not_recreate() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let key_path = dir.path().join(".mailgrit-audit-key");
    // Write a file of a deliberately wrong length (1 byte instead of 48).
    std::fs::write(&key_path, vec![0u8; 1])?;
    let original_len = std::fs::metadata(&key_path)?.len();

    let result = load_or_create_persistent_key(dir.path(), b"any-password-here");
    assert!(
        matches!(result, Err(AuditError::CorruptedKeyFile { actual: 1 })),
        "a damaged file must yield CorruptedKeyFile, not a recreation"
    );
    // The file must NOT have been recreated (same length).
    let after_len = std::fs::metadata(&key_path)?.len();
    assert_eq!(
        original_len, after_len,
        "a damaged file must not be silently recreated"
    );
    Ok(())
}

// An empty file (0 bytes) is a special case of damage → CorruptedKeyFile.
#[test]
fn empty_key_file_is_corrupted() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let key_path = dir.path().join(".mailgrit-audit-key");
    std::fs::write(&key_path, b"")?;
    let result = load_or_create_persistent_key(dir.path(), b"pw");
    assert!(matches!(
        result,
        Err(AuditError::CorruptedKeyFile { actual: 0 })
    ));
    Ok(())
}

// An oversized file (> the expected length) is also damage.
#[test]
fn oversized_key_file_is_corrupted() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let key_path = dir.path().join(".mailgrit-audit-key");
    std::fs::write(&key_path, vec![0u8; AUDIT_KEY_FILE_LEN + 10])?;
    let result = load_or_create_persistent_key(dir.path(), b"pw");
    assert!(matches!(
        result,
        Err(AuditError::CorruptedKeyFile { actual })
        if actual == AUDIT_KEY_FILE_LEN + 10
    ));
    Ok(())
}

// A file of the correct length, but the verify-token does not match the
// derived key (e.g. the salt/token is random junk) → WrongMasterPassword,
// NOT CorruptedKeyFile: the length is correct, but the password does not
// fit.
#[test]
fn wrong_length_ok_but_token_garbage_is_wrong_password() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let key_path = dir.path().join(".mailgrit-audit-key");
    // 48 bytes of random junk of the correct length.
    std::fs::write(&key_path, vec![0xABu8; AUDIT_KEY_FILE_LEN])?;
    let result = load_or_create_persistent_key(dir.path(), b"pw");
    assert!(matches!(result, Err(AuditError::WrongMasterPassword)));
    Ok(())
}

// Two consecutive key derivations with the same password from a correct file
// are deterministic (the same key).
#[test]
fn derived_key_is_deterministic_across_loads() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    load_or_create_persistent_key(dir.path(), b"deterministic-pw-9")?;
    let k1 = load_or_create_persistent_key(dir.path(), b"deterministic-pw-9")?;
    let k2 = load_or_create_persistent_key(dir.path(), b"deterministic-pw-9")?;
    assert_eq!(
        k1.as_bytes(),
        k2.as_bytes(),
        "the key is deterministic for one password and file"
    );
    Ok(())
}
