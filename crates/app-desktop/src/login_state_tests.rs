// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

//! Unit tests for the login-window shared state (mutation-testing pilot:
//! `LoginWindowState` had no direct tests, so every mutant in the file was
//! MISSED). The state is observable without any real webview/window: all
//! fields start as `None`, and the request/channel methods are pure
//! mutex-protected state transitions.

use super::*;
use mailgrit_core_domain::{BulkOperationKind, OperationTarget, RawCsvRow};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Poison recovery for test-held mutex guards (the same policy as the
/// production `lock` helper).
fn unlocked<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A valid sanitized row (the same fixture shape as `ops_export_tests`).
fn sanitized_row() -> Result<mailgrit_core_domain::SanitizedUserRow, Box<dyn std::error::Error>> {
    Ok(RawCsvRow::new(vec![
        "example.com".into(),
        "ivan.petrov".into(),
        "S3cur3P@ss1".into(),
        "Ivan Petrov".into(),
        "1024".into(),
    ])
    .parse()?)
}

/// The global accessor must hand out the SAME instance on every call of a
/// thread (state registered earlier must stay visible through later calls).
/// A fresh instance per call would silently drop every queued request.
#[test]
fn login_state_returns_the_same_instance_within_a_thread() -> TestResult {
    let a = login_state();
    a.request_open("https://mail.example.com".to_string());
    // A second call must see the first call's request.
    let b = login_state();
    if !Arc::ptr_eq(&a, &b) {
        return Err("login_state() must return the shared per-thread instance".into());
    }
    let base_url = {
        let guard = unlocked(&b.request);
        guard
            .as_ref()
            .ok_or("request survives across calls")?
            .base_url
            .clone()
    };
    assert_eq!(base_url, "https://mail.example.com");
    Ok(())
}

#[test]
fn request_open_stores_the_base_url() -> TestResult {
    let s = LoginWindowState::new();
    s.request_open("https://iredadmin.example.com".to_string());
    let base_url = {
        let guard = unlocked(&s.request);
        guard
            .as_ref()
            .ok_or("request_open must store a LoginRequest")?
            .base_url
            .clone()
    };
    assert_eq!(base_url, "https://iredadmin.example.com");
    Ok(())
}

#[test]
#[allow(clippy::significant_drop_tightening)] // the guard is held while the callback runs
fn set_on_login_stores_an_invocable_callback() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let fired = std::sync::Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    let s = LoginWindowState::new();
    s.set_on_login(Box::new(move || {
        flag.store(true, Ordering::SeqCst);
    }));
    // The stored callback must be retrievable and runnable.
    let guard = unlocked(&s.on_login);
    let cb = guard.as_ref().map(|b| b as &dyn Fn());
    if let Some(cb) = cb {
        cb();
    }
    assert!(fired.load(Ordering::SeqCst), "stored callback must fire");
}

#[test]
fn set_session_cookie_name_stores_the_name() {
    let s = LoginWindowState::new();
    s.set_session_cookie_name("webpy_session_id".to_string());
    let stored = unlocked(&s.session_cookie_name).clone();
    assert_eq!(
        stored.as_deref(),
        Some("webpy_session_id"),
        "the cookie name must be stored for the login predicate"
    );
}

#[test]
fn report_page_load_stores_the_auth_event() -> TestResult {
    let s = LoginWindowState::new();
    s.report_page_load(
        "https://mail.example.com".to_string(),
        "https://mail.example.com/iredadmin/dashboard".to_string(),
    );
    let ev = unlocked(&s.auth_event)
        .clone()
        .ok_or("report_page_load must record the event")?;
    assert_eq!(ev.base_url, "https://mail.example.com");
    assert_eq!(
        ev.final_url,
        "https://mail.example.com/iredadmin/dashboard"
    );
    Ok(())
}

#[test]
#[allow(clippy::significant_drop_tightening)] // the guard is scoped to the take/assert block
fn request_op_registers_first_and_rejects_duplicate() -> TestResult {
    let s = LoginWindowState::new();
    let row = sanitized_row()?;
    let rx = s.request_op(
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://mail.example.com".to_string(),
        vec![row],
    );
    let rx = rx.ok_or("the first request must be accepted")?;
    // A duplicate while one is pending must be rejected with None.
    let dup = s.request_op(
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://mail.example.com".to_string(),
        Vec::new(),
    );
    assert!(dup.is_none(), "a duplicate request must be rejected");
    // The stored request mirrors the arguments; the sender must be live.
    let tx = {
        let mut guard = unlocked(&s.op_request);
        let req = guard
            .take()
            .ok_or("the accepted request must be stored")?;
        assert_eq!(req.base_url, "https://mail.example.com");
        assert_eq!(req.rows.len(), 1);
        req.tx
    };
    // The returned receiver must resolve when the stored sender fires.
    if tx.send(Vec::new()).is_err() {
        return Err("the stored sender must be live".into());
    }
    match rx.blocking_recv() {
        Ok(received) => assert!(received.is_empty(), "the empty batch must arrive intact"),
        Err(_) => return Err("receiver must yield the result".into()),
    }
    Ok(())
}

#[test]
#[allow(clippy::significant_drop_tightening)] // the guard is scoped to the take/assert block
fn request_diag_registers_first_and_rejects_duplicate() -> TestResult {
    let s = LoginWindowState::new();
    let rx = s
        .request_diag("example.com".to_string())
        .ok_or("the first diagnostics request must be accepted")?;
    let dup = s.request_diag("example.com".to_string());
    assert!(dup.is_none(), "a duplicate diagnostics request is rejected");
    let tx = {
        let mut guard = unlocked(&s.diag_request);
        let req = guard
            .take()
            .ok_or("the accepted request must be stored")?;
        assert_eq!(req.domain, "example.com");
        req.tx
    };
    if tx.send("<html/>".to_string()).is_err() {
        return Err("the stored sender must be live".into());
    }
    match rx.blocking_recv() {
        Ok(html) => assert_eq!(html, "<html/>"),
        Err(_) => return Err("receiver must yield the HTML".into()),
    }
    Ok(())
}

/// Without a webview, script evaluation must FAIL loudly (the single mapping
/// of "no webview" to a wry error) — a silent `Ok(())` would make the batch
/// dispatcher believe JS ran.
#[test]
fn evaluate_script_without_webview_is_an_error() -> TestResult {
    let s = LoginWindowState::new();
    let err = match s.evaluate_script("1+1") {
        Ok(()) => return Err("without a webview, evaluate_script must Err".into()),
        Err(e) => e,
    };
    assert!(
        err.to_string().contains("login webview is gone"),
        "the error must name the missing webview: {err}"
    );
    Ok(())
}

/// Documented behavior: with no window open, `hide` is a no-op (the window
/// field starts as `None`; the visible branch needs a real tao window and is
/// exercised by the E2E suite).
#[test]
fn hide_without_window_is_a_noop() {
    let s = LoginWindowState::new();
    s.hide(); // must not panic
    assert!(unlocked(&s.window).is_none());
}

/// Documented behavior: `with_webview_cookies` yields `None` while no webview
/// exists. (The `Some` branch needs a live `wry` `WebView` and is covered by E2E.)
#[test]
fn with_webview_cookies_without_webview_is_none() {
    let s = LoginWindowState::new();
    let r: Option<u8> = s.with_webview_cookies(|_| 7u8);
    assert_eq!(r, None);
}
