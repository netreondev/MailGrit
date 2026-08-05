//! Success/error markers of iRedAdmin operations (OSE forms).
//!
//! iRedAdmin returns HTTP 200 even on error (e.g. ALREADY_EXISTS), so the status
//! code alone is insufficient: a positive success marker and/or the absence of an
//! error marker in the response body/URL is required.

// The predicates/constants primarily serve test coverage of the marker set as a
// regression guard.
#![cfg_attr(not(test), allow(dead_code))]

/// Positive markers in the response URL (query `?msg=CREATED|UPDATED|DELETED`).
pub const SUCCESS_URL_MSGS: &[&str] = &["CREATED", "UPDATED", "DELETED"];

/// Positive markers in the response body (the success-notification HTML class).
pub const SUCCESS_BODY: &[&str] = &["note-success"];

/// Negative markers in the response URL (all known iRedAdmin error codes).
pub const ERROR_URL_MSGS: &[&str] = &[
    "ERROR",
    "NO_SUCH_ACCOUNT",
    "ALREADY_EXISTS",
    "ACCOUNT_EXISTS",
    "NO_SUCH_DOMAIN",
    "ALREADY_EXISTS_DOMAIN",
    "NOT_ALLOWED",
];

/// Negative markers in the response body (notification HTML classes and error
/// signatures).
pub const ERROR_BODY: &[&str] = &[
    "note-error",
    "note-warning",
    "note-danger",
    "ALREADY_EXISTS",
    "ACCOUNT_EXISTS",
    "LDAP",
];

/// Map of known iRedAdmin error codes (`?msg=CODE`) → human-readable message.
pub const ERROR_CODE_MAP: &[(&str, &str)] = &[
    // (iRedAdmin code, translation key in locales/app.<lang>.yml).
    ("NO_SUCH_ACCOUNT", "err_code.NO_SUCH_ACCOUNT"),
    ("ALREADY_EXISTS", "err_code.ALREADY_EXISTS"),
    ("ACCOUNT_EXISTS", "err_code.ACCOUNT_EXISTS"),
    ("NO_SUCH_DOMAIN", "err_code.NO_SUCH_DOMAIN"),
    ("ALREADY_EXISTS_DOMAIN", "err_code.ALREADY_EXISTS_DOMAIN"),
    ("NOT_ALLOWED", "err_code.NOT_ALLOWED"),
    ("INVALID", "err_code.INVALID"),
];

/// Success indicator by URL and response body (at least one positive marker is present).
#[must_use]
pub fn has_ose_success(haystack_url: &str, haystack_body: &str) -> bool {
    SUCCESS_URL_MSGS.iter().any(|m| haystack_url.contains(m))
        || SUCCESS_BODY.iter().any(|m| haystack_body.contains(m))
}

/// Error indicator by URL and response body (OSE forms).
#[must_use]
pub fn has_ose_error(haystack_url: &str, haystack_body: &str) -> bool {
    ERROR_URL_MSGS.iter().any(|m| haystack_url.contains(m))
        || ERROR_BODY.iter().any(|m| haystack_body.contains(m))
}

/// Final success verdict of an OSE operation: HTTP OK + a success marker is
/// present + no error.
#[must_use]
pub fn ose_final_ok(http_ok: bool, haystack_url: &str, haystack_body: &str) -> bool {
    http_ok
        && has_ose_success(haystack_url, haystack_body)
        && !has_ose_error(haystack_url, haystack_body)
}

/// Extracts the value of the `msg` parameter from a URL query string
/// (`?msg=CODE`).
#[must_use]
pub fn extract_msg_code(url: &str) -> Option<&str> {
    let key = "msg=";
    let start = url.find(key)?;
    let value_start = start.saturating_add(key.len());
    let rest = &url[value_start..];
    let end = rest.find('&').unwrap_or(rest.len());
    let code = &rest[..end];
    if code.is_empty() { None } else { Some(code) }
}

/// Maps an iRedAdmin error code to a localized message (`None` for success and
/// unknown codes).
#[must_use]
pub fn map_error_code(code: &str) -> Option<String> {
    ERROR_CODE_MAP
        .iter()
        .find(|(k, _)| *k == code)
        .map(|(_, key)| t!(*key).to_string())
}

/// Generates a JS fragment with the marker arrays and predicates for
/// `webview_js`. The JS side uses the same constants as the Rust tests (a single
/// source).
#[must_use]
pub fn build_marker_js() -> String {
    let success_url = js_array(SUCCESS_URL_MSGS);
    let success_body = js_array(SUCCESS_BODY);
    let error_url = js_array(ERROR_URL_MSGS);
    let error_body = js_array(ERROR_BODY);
    format!(
        r"
        // iRedAdmin success/error markers (the single source of truth — webview_markers.rs).
        const MF_SUCCESS_URL = {success_url};
        const MF_SUCCESS_BODY = {success_body};
        const MF_ERROR_URL = {error_url};
        const MF_ERROR_BODY = {error_body};
        function mfAny(haystack, arr) {{
            const s = String(haystack);
            for (var i = 0; i < arr.length; i++) {{ if (s.indexOf(arr[i]) >= 0) return true; }}
            return false;
        }}
        function mfHasSuccess(url, body) {{ return mfAny(url, MF_SUCCESS_URL) || mfAny(body, MF_SUCCESS_BODY); }}
        function mfHasError(url, body) {{ return mfAny(url, MF_ERROR_URL) || mfAny(body, MF_ERROR_BODY); }}
        "
    )
}

/// The iRedAdmin base path (`MF_BASE`): the origin root + the `/iredadmin` prefix,
/// derived from `window.location.pathname`. A single fragment for the batch-IIFE
/// and the diag-IIFE — previously the `'/iredadmin'` string and the computation
/// logic were duplicated in both IIFEs. Here there is one source: change the
/// prefix in one place.
#[must_use]
pub fn build_base_js() -> String {
    r"
        // The iRedAdmin base URL: origin + the /iredadmin prefix from the current path.
        // The webview is already on the iRedAdmin page, so the prefix is taken from
        // pathname rather than hardcoded (accounts for non-standard deployments).
        const MF_BASE = (() => {{
            const p = window.location.pathname;
            const idx = p.indexOf('/iredadmin');
            const prefix = idx >= 0 ? p.substring(0, idx + '/iredadmin'.length) : '/iredadmin';
            return window.location.origin + prefix;
        }})();
    "
    .to_string()
}

/// Builds a JS array string from a slice of literals (safe: literals without
/// quotes/escapes).
fn js_array(items: &[&str]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("\"{s}\"")).collect();
    format!("[{}]", quoted.join(", "))
}

/// Generates a JS fragment with the error-code map (`MF_ERROR_MAP`) and a function
/// that extracts a human-readable message. Returns an empty string (NOT raw HTML),
/// either a translation by `?msg=CODE` or text cut out of the `note-error` block.
#[must_use]
pub fn build_error_map_js() -> String {
    // The map values are localized via t!; escape quotes/backslashes.
    let map_entries: Vec<String> = ERROR_CODE_MAP
        .iter()
        .map(|(k, key)| {
            let val = t!(*key).replace('\\', "\\\\").replace('"', "\\\"");
            format!("\"{k}\": \"{val}\"")
        })
        .collect();
    let map_obj = format!("{{ {} }}", map_entries.join(", "));
    // raw-string for JS regexes; {MAP} is substituted via replace.
    r#"
        // The iRedAdmin error-code map — a single source of truth shared with the Rust tests.
        const MF_ERROR_MAP = {MAP};
        function mfExtractMsgCode(url) {
            const s = String(url || '');
            const idx = s.indexOf('msg=');
            if (idx < 0) return '';
            const rest = s.substring(idx + 4);
            const amp = rest.indexOf('&');
            return amp >= 0 ? rest.substring(0, amp) : rest;
        }
        // A human-readable message from the URL/body; '' if nothing can be extracted.
        function mfExtractMessage(url, body) {
            const code = mfExtractMsgCode(url);
            if (code && Object.prototype.hasOwnProperty.call(MF_ERROR_MAP, code)) {
                return MF_ERROR_MAP[code];
            }
            // iRedAdmin renders a DOUBLE class ("notification note-error"),
            // so use class-contains via \bnote-...\b, not class="note-...".
            const b = String(body || '');
            var m = b.match(/<div[^>]*class=["'][^"']*\bnote-(?:error|warning|danger)\b[^"']*["'][^>]*>([\s\S]*?)<\/div>/);
            if (!m) {
                // A notification without a <div> wrapper (rare).
                m = b.match(/note-(?:error|warning|danger)[^>]*>([\s\S]*?)<\//);
            }
            if (m && m[1]) {
                return m[1].replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim().slice(0, 200);
            }
            return '';
        }
        "#
        .replace("{MAP}", &map_obj)
}

#[cfg(test)]
#[path = "webview_markers_tests.rs"]
mod tests;
