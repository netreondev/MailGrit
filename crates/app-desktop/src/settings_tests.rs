// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

//! Unit tests for config.toml load/save. They drive the `_from`/`_to` cores
//! with an explicit temp path — the REAL config next to the binary must never
//! be touched by the test suite.

use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A temp config.toml whose file AND directory are removed on drop, however
/// the test ends. The directory name is unique per invocation (parallel tests
/// must not share files), so without the guard every run would litter %TEMP%
/// with `mailgrit-settings-tests-*` directories.
struct TempConfig {
    path: PathBuf,
}

impl TempConfig {
    fn new(label: &str) -> Result<Self, Box<dyn std::error::Error>> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir()
            .join(format!("mailgrit-settings-tests-{pid}-{n}-{label}"))
            .join("config.toml");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        if let Some(dir) = self.path.parent() {
            // Empty once the config file above is gone.
            let _ = std::fs::remove_dir(dir);
        }
    }
}

/// A two-party rendezvous used by the concurrency test: [`Rendezvous::meet`]
/// returns `true` only when two callers are inside `meet` SIMULTANEOUSLY. A
/// timed-out meet takes its token back, or a later serialized pair would
/// match the leftover of an earlier one.
struct Rendezvous {
    met: std::sync::Mutex<u32>,
    cv: std::sync::Condvar,
}

impl Rendezvous {
    // Condvar protocol: the guard must be held while testing the count and
    // then MOVED into wait_timeout_while (which releases the mutex while
    // waiting) — dropping it any earlier would open a lost-wakeup window,
    // so the nursery drop-tightening lint is waived here deliberately.
    #[allow(clippy::significant_drop_tightening)]
    fn meet(&self, timeout: std::time::Duration) -> bool {
        // Poison recovery as everywhere in this crate: the count is still
        // structurally valid after a panic under the lock.
        let mut n = self
            .met
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *n = (*n).saturating_add(1);
        if *n == 2 {
            self.cv.notify_all();
            return true;
        }
        let (mut guard, _) = self
            .cv
            .wait_timeout_while(n, timeout, |n| *n < 2)
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *guard == 2 {
            return true;
        }
        // Timed out: take the token back so a later serialized pair cannot
        // match this leftover.
        *guard = (*guard).saturating_sub(1);
        false
    }
}

/// Joins a writer thread, propagating its panic (if any) as the test failure.
fn join_propagating_panic(handle: std::thread::JoinHandle<bool>) -> bool {
    match handle.join() {
        Ok(overlapped) => overlapped,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// Round-trip: save → load preserves every field.
#[test]
fn save_then_load_round_trips_all_fields() -> TestResult {
    let cfg = TempConfig::new("roundtrip")?;
    let settings = Settings {
        base_url: "https://mail.example.com/iredadmin".into(),
        theme: "light".into(),
        language: "de".into(),
        password_generator: PasswordGeneratorConfig {
            length: 24,
            ..Settings::default().password_generator
        },
        password_policy: PasswordPolicyConfig {
            min_len: 12,
            ..Settings::default().password_policy
        },
        ..Settings::default()
    };
    save_to(cfg.path(), &settings);

    let loaded = load_from(cfg.path());
    assert_eq!(loaded.base_url, settings.base_url);
    assert_eq!(loaded.theme, "light");
    assert_eq!(loaded.language, "de");
    assert_eq!(loaded.password_generator.length, 24);
    assert_eq!(loaded.password_policy.min_len, 12);
    Ok(())
}

/// A missing file yields the defaults AND creates a sample on disk.
#[test]
fn missing_file_creates_sample_and_returns_defaults() -> TestResult {
    let cfg = TempConfig::new("missing")?;
    let loaded = load_from(cfg.path());
    assert_eq!(loaded, Settings::default());
    assert!(
        cfg.path().exists(),
        "a sample config must be created for the user"
    );
    Ok(())
}

/// A partial TOML gets serde defaults for the missing keys (documented behavior).
#[test]
fn partial_toml_falls_back_to_field_defaults() -> TestResult {
    let cfg = TempConfig::new("partial")?;
    std::fs::write(cfg.path(), "theme = \"light\"\n")?;

    let loaded = load_from(cfg.path());
    assert_eq!(loaded.theme, "light");
    assert_eq!(
        loaded.language, "en",
        "missing language falls back to English"
    );
    assert_eq!(
        loaded.session_cookie_name, "webpy_session_id",
        "missing cookie name falls back to the web.py default"
    );
    Ok(())
}

/// A corrupt file yields the defaults (logged warning, no panic).
#[test]
fn corrupt_toml_yields_defaults_without_panicking() -> TestResult {
    let cfg = TempConfig::new("corrupt")?;
    std::fs::write(cfg.path(), "not [ valid toml")?;

    let loaded = load_from(cfg.path());
    assert_eq!(loaded, Settings::default());
    Ok(())
}

/// A stale/empty cookie name is migrated to the current default (documented).
#[test]
fn stale_cookie_name_is_migrated() -> TestResult {
    let cfg = TempConfig::new("cookie-migrate")?;
    std::fs::write(cfg.path(), "session_cookie_name = \"sessionid\"\n")?;

    let loaded = load_from(cfg.path());
    assert_eq!(loaded.session_cookie_name, "webpy_session_id");
    Ok(())
}

/// Concurrent read-modify-write cycles cannot interleave: two threads run one
/// `save_field_at` cycle each against the same temp file, with a rendezvous
/// INSIDE the apply closure — i.e. between the load and the save. Under
/// [`CONFIG_WRITE_LOCK`] the two applies can never overlap, so each rendezvous
/// must time out and both final fields survive. With the lock removed the two
/// applies would meet (both loads done before either save), and whichever
/// thread saved second would revert the other's field to its stale snapshot.
#[test]
fn concurrent_field_saves_do_not_interleave() -> TestResult {
    let cfg = TempConfig::new("concurrent")?;
    let path = cfg.path().to_path_buf();
    save_to(&path, &Settings::default());

    let rendezvous = std::sync::Arc::new(Rendezvous {
        met: std::sync::Mutex::new(0),
        cv: std::sync::Condvar::new(),
    });
    let writer = |writes_theme: bool, path: PathBuf, rendezvous: std::sync::Arc<Rendezvous>| {
        std::thread::spawn(move || {
            let mut overlapped = false;
            save_field_at(&path, |s| {
                if writes_theme {
                    s.theme = "light-final".into();
                } else {
                    s.language = "de-final".into();
                }
                overlapped |= rendezvous.meet(std::time::Duration::from_millis(250));
            });
            overlapped
        })
    };
    let a = writer(true, path.clone(), rendezvous.clone());
    let b = writer(false, path.clone(), rendezvous);
    let overlapped_a = join_propagating_panic(a);
    let overlapped_b = join_propagating_panic(b);

    assert!(
        !overlapped_a && !overlapped_b,
        "two save_field_at applies ran concurrently — the lock no longer spans the whole load→modify→save"
    );
    let loaded = load_from(&path);
    assert_eq!(
        loaded.theme, "light-final",
        "the theme write was reverted by an interleaved cycle"
    );
    assert_eq!(
        loaded.language, "de-final",
        "the language write was reverted by an interleaved cycle"
    );
    Ok(())
}

/// `save_field_at` applies its closure to the LOADED config (not blank state):
/// the documented contract "updates only the field, preserving the others".
#[test]
fn save_field_at_preserves_untouched_fields() -> TestResult {
    let cfg = TempConfig::new("preserve")?;
    let start = Settings {
        language: "uk".into(),
        base_url: "https://mail.example.com/iredadmin".into(),
        ..Settings::default()
    };
    save_to(cfg.path(), &start);

    save_field_at(cfg.path(), |s| s.theme = "light".into());

    let loaded = load_from(cfg.path());
    assert_eq!(loaded.theme, "light");
    assert_eq!(
        loaded.language, "uk",
        "untouched field must survive the cycle"
    );
    assert_eq!(
        loaded.base_url, "https://mail.example.com/iredadmin",
        "untouched field must survive the cycle"
    );
    Ok(())
}
