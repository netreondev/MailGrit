//! Deterministic regression tests: secrets (passwords, CSRF tokens) must never
//! appear in `tracing` log output.
//!
//! These tests install a capturing `tracing-subscriber` (via `MakeWriter` over a
//! shared buffer), drive the IPC/batch/diag code paths with inputs that contain a
//! known secret marker, and assert the marker does NOT appear in the captured
//! log lines. They pin the current (correct) behavior so a future regression —
//! e.g. someone adding `tracing::debug!("rows: {rows_json:?}")` where `rows_json`
//! was built from `row_to_json` (which embeds the plaintext password), or
//! re-introducing a `warn!("...raw: {s}")` log of a malformed JS response — is
//! caught at PR time rather than leaking a password to disk in `mailgrit.log`.
//!
//! Coverage:
//! - The REAL leak surface for a malformed response (`parse_batch_result` parse
//!   error): the raw text is logged with an error + length only, never the
//!   payload, because a malformed response bypasses the JS-side `mfMask`.
//! - The failure-dump INFO path (`log_result`): the structured `dump` IS logged
//!   (the contract is that JS already masked it), and Rust must not additionally
//!   inject in-memory password state into that line.
//! - A tripwire over the JS-builder modules (no `tracing::` calls there today).
//!
//! Scope: Rust-side logging only. The JS-side masking (`mfMask` in
//! `webview_js/shared.rs`) is covered by string-presence tests in
//! `webview_js_tests.rs`; here we assert the Rust `tracing` path is clean.

use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

use mailgrit_core_domain::{
    BulkOperationKind, CsvRowError, OperationTarget, RawCsvRow, SanitizedUserRow,
};

// This test module is wired as a submodule of `webview_ops` (see webview_ops.rs),
// so `super::` resolves to webview_ops, where these functions live / are re-exported.
use super::{build_batch_js, parse_batch_result, row_to_json};

/// A unique, grep-able secret marker used as the test password. If this exact
/// string ever appears in captured log output, a secret has leaked.
const SECRET_PASSWORD_MARKER: &str = "ZZLEAKMARKERZZ_s3cr3t_ZZ";

/// A capturing writer: all `tracing` events emitted while the subscriber is
/// active are appended to the shared buffer. Based on the documented
/// `MakeWriter` pattern from tracing-subscriber.
struct CapturingWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl<'a> MakeWriter<'a> for CapturingWriter {
    type Writer = VecWriter;
    fn make_writer(&'a self) -> Self::Writer {
        VecWriter {
            buf: Arc::clone(&self.buf),
        }
    }
}

struct VecWriter {
    buf: Arc<Mutex<Vec<u8>>>,
}

impl std::io::Write for VecWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buf
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Builds a `SanitizedUserRow` whose password is the secret marker. Goes through
/// the canonical typestate parser (the only way to construct `SanitizedUserRow`).
fn row_with_secret_password() -> Result<SanitizedUserRow, CsvRowError> {
    RawCsvRow::new(vec![
        "example.com".into(),
        "test.user".into(),
        SECRET_PASSWORD_MARKER.into(),
        "Test User".into(),
        "1024".into(),
    ])
    .parse()
}

/// Installs a capturing subscriber for the duration of the returned guard.
/// Dropping the guard restores the previous subscriber state. Uses the
/// `MakeWriter` API so no global `try_init` conflict arises between tests.
fn capture_logs() -> (Arc<Mutex<Vec<u8>>>, tracing::subscriber::DefaultGuard) {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = CapturingWriter {
        buf: Arc::clone(&buf),
    };
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(writer)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    (buf, guard)
}

/// Returns the captured log output as a UTF-8 string.
fn captured_string(buf: &Arc<Mutex<Vec<u8>>>) -> String {
    String::from_utf8_lossy(&buf.lock().map(|b| b.clone()).unwrap_or_default()).into_owned()
}

// ============================================================================
// parse_batch_result — malformed-response path: the raw response is NOT logged.
//
// This is the REAL leak surface for a malformed response: a server echo, a dump
// field, or an attacker-controlled payload could carry the plaintext password
// into the raw string. Because the response is malformed, JSON parsing fails
// BEFORE the structured `dump` is ever read, so the JS-side `mfMask` redaction
// (which only runs when building the `dump`) has NOT been applied — the raw
// text is untrusted.
//
// The contract under test: on a parse failure, `parse_batch_result` must emit
// only the error + the response length, NEVER the raw text. This test feeds a
// malformed response that DOES contain the secret marker (simulating a leak in
// the raw payload) and asserts the marker never reaches tracing. It FAILS if
// anyone reintroduces a `warn!("...raw: {s}")`-style log of the raw response.
// ============================================================================
#[test]
fn malformed_response_with_secret_marker_is_not_logged() {
    let (buf, _guard) = capture_logs();

    // Malformed JSON (trailing comma) AND it embeds the secret marker in a
    // field value — exactly the shape of a leaked secret in the raw response.
    let malformed = format!(
        r#"{{"username":"u","password":"{SECRET_PASSWORD_MARKER}","dump":"{SECRET_PASSWORD_MARKER}",}}"#
    );
    let results = parse_batch_result(&malformed);
    assert!(results.is_empty(), "malformed response yields no results");

    let logs = captured_string(&buf);
    assert!(
        !logs.contains(SECRET_PASSWORD_MARKER),
        "plaintext password marker leaked into tracing logs on the malformed-response path:\n{logs}"
    );
}

// ============================================================================
// parse_batch_result — well-formed failure path: the structured `dump` IS
// logged by `log_result` at INFO. The dump is expected to already be masked by
// the JS side (mfMask) before it reaches Rust; this test pins that the RUST
// side does not additionally inject the in-memory password into that log line.
//
// This is a contract test: the dump in the response is a value WE supply here,
// so we make it NOT contain the marker (as a correctly-masked dump would not),
// and assert the marker (which lives only in Rust SanitizedUserRow state) is
// not serialized into the dump log line by Rust. If Rust ever started logging
// row state alongside the dump, this would catch it.
// ============================================================================
#[test]
fn failure_dump_log_does_not_inject_rust_password_state() -> Result<(), Box<dyn std::error::Error>>
{
    let (buf, _guard) = capture_logs();

    // Hold a row whose password is the secret marker, so the marker genuinely
    // exists in Rust state (this is what we do NOT want re-serialized into logs).
    let _row = row_with_secret_password()?;

    // A well-formed FAILURE result whose dump is correctly masked (no marker).
    // This exercises log_result's INFO dump-emission branch.
    let js = r#"[{"username":"u","domain":"d","ok":false,"status":500,"error":"boom","dump":{"responseUrl":"https://x/?err=1","requestFields":"newpw=abcdefgh***"}}]"#;
    let results = parse_batch_result(js);
    assert_eq!(results.len(), 1);
    let first = results.first().ok_or("expected at least one result")?;
    assert!(first.outcome.is_err());

    let logs = captured_string(&buf);
    // The dump IS logged (proving we exercised the real path)...
    assert!(
        logs.contains("OPERATION DUMP"),
        "expected the failure dump to be logged (test is otherwise vacuous):\n{logs}"
    );
    // ...but the Rust-held secret marker must not have been injected into it.
    assert!(
        !logs.contains(SECRET_PASSWORD_MARKER),
        "Rust injected the in-memory password marker into the dump log line:\n{logs}"
    );
    Ok(())
}

// ============================================================================
// Tripwire (NOT a current-behavior test): `row_to_json` and `build_batch_js`
// emit ZERO tracing events today (verified: the webview_js builder modules
// contain no `tracing::`/`log::` calls). This test therefore cannot fail unless
// someone later adds instrumentation that logs the built row JSON / batch JS —
// in which case the plaintext password embedded by `row_to_json` would leak.
//
// It is deliberately kept as a regression tripwire for future code changes,
// not a claim that the current code path was exercised for leakage.
// ============================================================================
#[test]
fn tripwire_row_and_batch_js_builders_stay_silent() -> Result<(), Box<dyn std::error::Error>> {
    let (buf, _guard) = capture_logs();

    let row = row_with_secret_password()?;
    // Build the JSON value (contains plaintext password by design — it is sent
    // to the trusted webview, not to logs).
    let _json = row_to_json(&row);
    // Build the batch JS (embeds the JSON).
    let _js = build_batch_js(
        1,
        OperationTarget::User,
        BulkOperationKind::Create,
        "https://example.com",
        std::slice::from_ref(&row),
        true,
    );

    let logs = captured_string(&buf);
    assert!(
        !logs.contains(SECRET_PASSWORD_MARKER),
        "plaintext password marker leaked while building row JSON / batch JS \
         (a tracing call was added to the JS builders):\n{logs}"
    );
    Ok(())
}

// ============================================================================
// Sanity check: the marker DOES exist in the row JSON (otherwise the leak
// assertions above would be vacuous for the row/batch path). This confirms the
// marker is genuinely present in the data that must NOT reach logs.
// ============================================================================
#[test]
fn marker_is_actually_present_in_row_json_non_vacuous() -> Result<(), Box<dyn std::error::Error>> {
    let row = row_with_secret_password()?;
    let json = row_to_json(&row).to_string();
    assert!(
        json.contains(SECRET_PASSWORD_MARKER),
        "test is vacuous: marker not present in row_to_json output:\n{json}"
    );
    Ok(())
}
