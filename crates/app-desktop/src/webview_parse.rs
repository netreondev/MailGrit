//! Parsing of JSON responses arriving from the login-WebView2 over IPC.
//!
//! See the rationale for the webview approach (FortiWeb/WAF) at the root of
//! [`crate::webview_ops`]. Here are the parsers for JS operation results
//! (`parse_batch_result`).
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use crate::webview_ops::OpResult;

/// Parses a JS response (a JSON array of results) into Vec<OpResult>.
///
/// The response arrives via IPC `window.ipc.postMessage("batch:id:json")`:
/// `dispatch` strips the `batch:id:` prefix and passes `json` here — the raw JSON
/// array (starting with `[`). Previously there was a branch here for unquoting
/// JSON quotes from `evaluate_script_with_callback`, but that path is not used in
/// batch operations (see `ipc.rs`), so the branch was unreachable and removed.
///
/// On a parse error the raw response is **not** logged: a malformed response
/// bypasses the JS-side `mfMask` redaction (which only runs when building the
/// structured `dump`), so its raw text is untrusted and may contain a plaintext
/// secret. Only the parse error and the response length are emitted.
pub fn parse_batch_result(js_response: &str) -> Vec<OpResult> {
    let s = js_response.trim();
    let arr: Vec<serde_json::Value> = match serde_json::from_str(s) {
        Ok(v) => v,
        Err(e) => {
            // SECURITY: a malformed response arrives BEFORE the normal dump path,
            // so the JS-side masking (mfMask) that redacts passwords/CSRF/email/PII
            // inside the structured `dump` has NOT been applied to this raw string.
            // Log only the error + the response length, NEVER the raw text — a
            // server echo or an attacker-controlled payload could otherwise carry a
            // plaintext secret into `mailgrit.log` verbatim. (Bounded previews are
            // still unsafe: a secret may sit anywhere in the response and passwords
            // are up to MAX_PASSWORD_LEN chars, longer than any sane preview.)
            tracing::warn!(
                "parsing JS batch response failed: {e}; response was {} chars (not logged)",
                s.chars().count()
            );
            return Vec::new();
        }
    };
    arr.iter().map(parse_one_result).collect()
}

/// Extracts a field from an operation result's `dump` by key.
fn dump_field<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    v.get("dump").and_then(|d| d.get(key))
}

/// A top-level string field of a result (with a fallback to `"<unknown>"`).
fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<unknown>")
        .to_string()
}

/// Parses a single JSON result object into an [`OpResult`] with logging.
fn parse_one_result(v: &serde_json::Value) -> OpResult {
    let username = str_field(v, "username");
    let domain = str_field(v, "domain");
    let ok = v
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let status = v
        .get("status")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let err = v
        .get("error")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    // P0: the final response URL from dump — an objective signal of session
    // expiry (contains /login after a redirect), independent of the reason text.
    let resp_url = dump_field(v, "responseUrl")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    // P0: the post-verification URL (profile GET) — contains /login if the
    // session expired during verify (between a successful POST and the
    // verify-GET). Without this field, the detector missed session loss in the
    // verify window.
    let verify_url = dump_field(v, "verifyUrl")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // E3: the reason as-is. After E1, `err` is a human-readable message, so the
    // redundant HTTP {status} prefix is not added (the status is visible in
    // dump). An empty err → a neutral status.
    let outcome = if ok {
        Ok(())
    } else if !err.is_empty() {
        tracing::warn!("operation failed: {username}@{domain}: {err}");
        Err(err.to_string())
    } else {
        let reason = format!("HTTP {status}");
        tracing::warn!("operation failed: {username}@{domain}: {reason}");
        Err(reason)
    };
    log_result(v, &username, &domain, ok, status, resp_url.as_deref());
    OpResult {
        username,
        domain,
        outcome,
        status,
        resp_url,
        verify_url,
    }
}

/// Logs an operation result (separately for success/failure — P2a).
///
/// Success → a compact INFO line without the body (~5000 characters of HTML is
/// noise on success) + the full dump at debug level. Failure → the full dump at
/// INFO (the body is needed for debugging, including `responseBodyFull`).
fn log_result(
    v: &serde_json::Value,
    username: &str,
    domain: &str,
    ok: bool,
    status: i64,
    resp_url: Option<&str>,
) {
    let Some(dump) = v.get("dump") else {
        return;
    };
    let dump_str = serde_json::to_string_pretty(dump).unwrap_or_else(|_| dump.to_string());
    if ok {
        let marker = dump_field(v, "successMarker")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let verified = dump_field(v, "verified").map(std::string::ToString::to_string);
        let timing = dump_field(v, "timingMs")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        tracing::info!(
            "operation OK: {username}@{domain} status={status} marker={marker} \
             verified={verified:?} {timing}ms url={}",
            resp_url.unwrap_or("?")
        );
        tracing::debug!(
            "=== OPERATION DUMP {username}@{domain} (ok={ok}, status={status}) ===\n{dump_str}"
        );
    } else {
        tracing::info!(
            "=== OPERATION DUMP {username}@{domain} (ok={ok}, status={status}) ===\n{dump_str}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ok=true → Ok(()); status and resp_url are populated.
    #[test]
    fn parse_success_ok() -> Result<(), Box<dyn std::error::Error>> {
        let js = r#"[{"username":"u","domain":"d","ok":true,"status":200,"error":null,"dump":{"responseUrl":"https://x/?msg=CREATED"}}]"#;
        let r = parse_batch_result(js);
        assert_eq!(r.len(), 1);
        let first = r.first().ok_or("expected at least one result")?;
        assert!(first.outcome.is_ok());
        assert_eq!(first.username, "u");
        assert_eq!(first.domain, "d");
        assert_eq!(first.status, 200);
        assert_eq!(first.resp_url.as_deref(), Some("https://x/?msg=CREATED"));
        Ok(())
    }

    // ok=false with a non-empty reason → Err(reason), without an HTTP prefix (E3).
    #[test]
    fn parse_failure_with_reason() -> Result<(), Box<dyn std::error::Error>> {
        let js = r#"[{"username":"u","domain":"d","ok":false,"status":200,"error":"Account does not exist","dump":{"responseUrl":"https://x/?msg=NO_SUCH_ACCOUNT"}}]"#;
        let r = parse_batch_result(js);
        let first = r.first().ok_or("expected at least one result")?;
        assert_eq!(first.outcome, Err("Account does not exist".to_string()));
        assert_eq!(first.status, 200);
        assert_eq!(
            first.resp_url.as_deref(),
            Some("https://x/?msg=NO_SUCH_ACCOUNT")
        );
        Ok(())
    }

    // ok=false with an empty reason → Err("HTTP {status}") (E3 fallback).
    #[test]
    fn parse_failure_empty_reason_synthesizes_status() -> Result<(), Box<dyn std::error::Error>> {
        let js =
            r#"[{"username":"u","domain":"d","ok":false,"status":500,"error":null,"dump":null}]"#;
        let r = parse_batch_result(js);
        let first = r.first().ok_or("expected at least one result")?;
        assert_eq!(first.outcome, Err("HTTP 500".to_string()));
        assert_eq!(first.status, 500);
        assert!(first.resp_url.is_none());
        Ok(())
    }

    // Malformed JSON → an empty vector (triggers the "0 results" guard in ops.rs).
    #[test]
    fn parse_malformed_json_returns_empty() {
        let r = parse_batch_result("not JSON at all");
        assert!(r.is_empty());
    }

    // No dump → resp_url = None (does not panic).
    #[test]
    fn parse_no_dump_yields_none_url() -> Result<(), Box<dyn std::error::Error>> {
        let js = r#"[{"username":"u","domain":"d","ok":true,"status":200}]"#;
        let r = parse_batch_result(js);
        let first = r.first().ok_or("expected at least one result")?;
        assert!(first.resp_url.is_none());
        assert_eq!(first.status, 200);
        Ok(())
    }

    // Multiple results preserve order.
    #[test]
    fn parse_multiple_preserves_order() -> Result<(), Box<dyn std::error::Error>> {
        let js = r#"[
            {"username":"a","domain":"d","ok":true,"status":200},
            {"username":"b","domain":"d","ok":false,"status":200,"error":"Error"}
        ]"#;
        let r = parse_batch_result(js);
        assert_eq!(r.len(), 2);
        let first = r.first().ok_or("expected first result")?;
        assert_eq!(first.username, "a");
        assert!(first.outcome.is_ok());
        let second = r.get(1).ok_or("expected second result")?;
        assert_eq!(second.username, "b");
        assert!(second.outcome.is_err());
        Ok(())
    }
}
