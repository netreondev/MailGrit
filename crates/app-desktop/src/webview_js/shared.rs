//! Shared JS fragments reused by the user/domain/admin operation builders.
//!
//! Previously `mfMask`, `getCsrf`, `verifyOp`, the `doOp` pipeline itself, and
//! the final batch IIFE wrapper were duplicated in [`super::user`], [`super::domain`],
//! [`super::admin`] with minor differences. Here is a single parameterized
//! implementation: target modules pass their differences (endpoints, form fields,
//! log tag) through [`DoOpSpec`] and [`BatchFragments`].
//!
//! What is unique to each target — form fields (`buildFields`) and endpoints —
//! stays in the target modules; the `doOp` pipeline is universal and lives here.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::helpers::{get_csrf_js, mf_mask_js};
use mailgrit_core_domain::SanitizedUserRow;

/// JS function `verifyOp(base, kind, identifier)`: create/delete post-verification
/// via a profile GET (D4). Structurally identical for all targets.
///
/// `profile_segment` is the URL segment of the target's profile
/// (`/profile/user/general/`, `/profile/domain/general/`,
/// `/profile/admin/general/`). The identifier parameter is unified under the name
/// `identifier` (email/domain/mail in JS).
#[must_use]
pub(super) fn verify_op_js(profile_segment: &str) -> String {
    format!(
        r"
    async function verifyOp(base, kind, identifier) {{
        const t0 = performance.now();
        try {{
            const profileUrl = base + '{profile_segment}' + encodeURIComponent(identifier);
            const r = await fetch(profileUrl, {{credentials:'include', redirect:'follow'}});
            const body = await r.text().catch(() => '');
            const ms = Math.round(performance.now() - t0);
            const finalUrl = r.url || '';
            const onProfilePage = finalUrl.indexOf('{profile_segment}') >= 0;
            const noteErr = body.indexOf('note-error') >= 0 || body.indexOf('note-danger') >= 0;
            if (kind === 'create') {{
                const ok = r.status === 200 && onProfilePage && !noteErr;
                // url is propagated upward: if the session expired during verify,
                // finalUrl contains /login — this is the signal for the
                // session-expiry detector (P0: previously only the POST-url was
                // available, the verify-url was lost).
                return {{verified: ok, status: r.status, reason: ok ? '' : 'profile not found after create', url: finalUrl, ms: ms}};
            }} else {{ // delete
                const gone = !onProfilePage || noteErr || r.status !== 200;
                return {{verified: gone, status: r.status, reason: gone ? '' : 'profile still exists after delete', url: finalUrl, ms: ms}};
            }}
        }} catch (e) {{
            return {{verified: null, status: 0, reason: '', url: '', ms: Math.round(performance.now() - t0)}};
        }}
    }}
    "
    )
}

/// Assembles the set of JS helpers common to all targets: `mfMask` + `getCsrf`
/// (the CSRF log tag differentiates targets) + `verifyOp` (parameterized by the
/// target's profile segment). Before deduplication, this function was copied in
/// each target module (`user`, `domain`, `admin`) with a difference of only two
/// lines.
#[must_use]
pub(super) fn csrf_mask_verify_js(csrf_log_tag: &str, profile_segment: &str) -> String {
    let mut helpers = String::new();
    helpers.push_str(mf_mask_js());
    helpers.push('\n');
    helpers.push_str(&get_csrf_js(csrf_log_tag));
    helpers.push('\n');
    helpers.push_str(&verify_op_js(profile_segment));
    helpers
}

/// Differences of an operation target that parameterize the unified batch JS
/// assembler ([`build_target_batch_js`]). Grouping into a struct keeps the number
/// of function arguments within the pedantic limit and makes dependencies
/// explicit.
#[must_use]
pub(super) struct TargetBatchSpec<'a> {
    /// Log tag for `getCsrf` (`"CSRF"` / `"CSRF-DOMAIN"` / `"CSRF-ADMIN"`).
    pub csrf_log_tag: &'a str,
    /// Profile URL segment for `verifyOp`
    /// (`/profile/user/general/`, etc.).
    pub profile_segment: &'a str,
    /// JS definition of `buildFields(csrf, row)` — the target's form fields.
    pub build_fields: &'a str,
    /// JS definition of `doOp(base, row)` — the target's single-request pipeline.
    pub do_op: &'a str,
}

/// Universal JS batch builder for operations on any target. Before
/// deduplication, `build_user_batch_js` / `build_domain_batch_js` /
/// `build_admin_batch_js` repeated this skeleton with differences only in
/// [`TargetBatchSpec`]. The target passes its differences; everything else is
/// shared.
#[must_use]
pub(super) fn build_target_batch_js(
    id: u64,
    base_url: &str,
    rows: &[SanitizedUserRow],
    verify: bool,
    spec: &TargetBatchSpec<'_>,
) -> String {
    // base_url is not needed: the iRedAdmin root is derived from window.location
    // inside JS (the webview is already on the iRedAdmin page). The parameter is
    // kept for signature compatibility with the per-target wrappers.
    let _ = base_url;
    let rows_json: Vec<serde_json::Value> =
        rows.iter().map(crate::webview_ops::row_to_json).collect();
    let marker_js = crate::webview_markers::build_marker_js();
    let base_js = crate::webview_markers::build_base_js();
    let error_map_js = crate::webview_markers::build_error_map_js();
    let verify_flag = if verify { "true" } else { "false" };
    let csrf_helpers = csrf_mask_verify_js(spec.csrf_log_tag, spec.profile_segment);

    batch_iife_js(
        id,
        &marker_js,
        &base_js,
        &error_map_js,
        &BatchFragments {
            verify_flag,
            helpers_js: &csrf_helpers,
            build_fields: spec.build_fields,
            do_op: spec.do_op,
        },
        &serde_json::Value::Array(rows_json),
    )
}

/// Parameterization of the unified `doOp` JS pipeline for a specific operation
/// target (User/Domain/Admin). The target passes its differences — endpoints, the
/// JS expression of the post-verification target, and the log tag/identifier; the
/// rest of the pipeline (CSRF-fetch → `URLSearchParams` → POST → dump → verdict →
/// post-verification) is identical for all iRedAdmin OSE forms and lives in
/// [`build_do_op_js`].
#[must_use]
pub(super) struct DoOpSpec {
    /// JS expression for the form URL (e.g. `base + '/create/user/' + encodeURIComponent(row.domain)`).
    pub path_fn: &'static str,
    /// JS expression for the form's `action` suffix to bind CSRF (D6).
    pub form_action_suffix: &'static str,
    /// JS literal of the operation kind: `'create'` / `'edit'` / `'delete'`.
    pub kind_js: &'static str,
    /// JS expression of the post-verification target for `verifyOp`
    /// (`row.email`, `row.domain`, `row.username + '@' + row.domain`).
    pub verify_target_js: &'static str,
    /// Log line tag (`[OP]` / `[OP-DOMAIN]` / `[OP-ADMIN]`).
    pub log_tag: &'static str,
    /// JS expression of the target identifier for log lines
    /// (`row.username`, `row.domain`, `row.username + '@' + row.domain`).
    pub log_id_js: &'static str,
}

/// Max characters of a response HEADER VALUE kept in the dump (a header value
/// is diagnostic metadata; the rest is truncated).
const RESP_HEADER_VALUE_MAX: usize = 200;
/// Max characters of the response BODY kept in the dump (`responseBodyFull`) —
/// the "full operation dump" contract: enough HTML for debugging an iRedAdmin
/// verdict, bounded so a huge error page does not balloon the IPC message.
const RESP_BODY_MAX: usize = 5000;

/// Builds the unified JS function `doOp(base, row)` — the pipeline of a single
/// request to an iRedAdmin OSE form for any operation target.
///
/// Skeleton: GET form → CSRF → `URLSearchParams` → POST → full dump → verdict by
/// markers → create/delete post-verification. All differences between
/// User/Domain/Admin are parameterized via [`DoOpSpec`]; structurally and
/// semantically it is a single pipeline.
///
/// The response dump is uniform across all targets: it includes `csrfInputs` and
/// `responseBodyFull` (up to [`RESP_BODY_MAX`] characters) — matching the "Full
/// operation dump" contract (README).
//
// A single doOp JS pipeline whose fragments are interpolated into each other.
// Further splitting would break the pipeline's locality.
pub(super) fn build_do_op_js(spec: &DoOpSpec) -> String {
    let DoOpSpec {
        path_fn,
        form_action_suffix,
        kind_js,
        verify_target_js,
        log_tag,
        log_id_js,
    } = *spec;
    format!(
        r"
    async function doOp(base, row) {{
        const t0 = performance.now();
        const formUrl = {path_fn};
        const kind = {kind_js};
        // 1. Obtain CSRF (GET the form page); bound to the form by formActionSuffix (D6).
        const formActionSuffix = {form_action_suffix};
        const csrfResult = await getCsrf(formUrl, formActionSuffix);
        const csrf = csrfResult.token || '';
        if (!csrf) {{
            return {{ok: false, status: 0, error: 'CSRF token not found at ' + formUrl, dump: {{csrfResult: csrfResult}}}};
        }}
        // 2. Build the form fields as URL-encoded (NOT multipart/FormData —
        //    FortiWeb may block multipart).
        const fields = buildFields(csrf, row);
        const params = new URLSearchParams();
        for (const [k, v] of fields) params.append(k, v);
        // 3. POST with Content-Type: application/x-www-form-urlencoded.
        const r = await fetch(formUrl, {{
            method: 'POST',
            body: params,
            headers: {{'Content-Type': 'application/x-www-form-urlencoded'}},
            credentials: 'include'
        }});
        const t2 = performance.now();
        // 4. Full response dump.
        const respBody = await r.text().catch(() => '');
        const respHeaders = {{}};
        r.headers.forEach((val, key) => {{ respHeaders[key] = val.slice(0, {RESP_HEADER_VALUE_MAX}); }});
        // r.ok is sufficient: fetch uses redirect:'follow' (by default), so
        // r.status is the final code after the redirect (never 302/303). Earlier
        // there were dead `r.status === 302/303` checks here, unreachable under follow.
        const ok = r.ok;
        const respUrl = r.url || '';
        // D1/D2: verdict by markers from webview_markers.rs.
        //   Success = HTTP OK + a positive marker + no error marker.
        const hasSuccess = mfHasSuccess(respUrl, respBody);
        const isErrorMsg = mfHasError(respUrl, respBody);
        const finalOk = ok && hasSuccess && !isErrorMsg;
        // 5. D4: post-verification for create/delete (if the POST looked successful).
        //    Edit is already confirmed by ?msg=UPDATED.
        let verified = null;
        let verifyStatus = 0;
        let verifyReason = '';
        let verifyMs = 0;
        let verifyUrl = '';
        if (finalOk && MF_VERIFY && (kind === 'create' || kind === 'delete')) {{
            const vr = await verifyOp(base, kind, {verify_target_js});
            verified = vr.verified;
            verifyStatus = vr.status;
            verifyReason = vr.reason;
            verifyMs = vr.ms;
            verifyUrl = vr.url || '';
            if (!verified) {{
                console.warn('[VERIFY] ' + ({log_id_js}) + ' ' + kind + ' NOT confirmed: ' + verifyReason);
            }}
        }}
        const opOk = verified === null ? finalOk : (finalOk && verified);
        console.log('[{log_tag}] ' + ({log_id_js}) + ': POST ' + formUrl + ' → ' + r.status + ' ' + (opOk?'OK':'FAIL') + ' bodyLen=' + respBody.length + ' ' + Math.round(t2-t0) + 'ms');
        return {{
            ok: opOk,
            status: r.status,
            // E1: a human-readable message instead of raw HTML. Cascade:
            //   1. mfExtractMessage — the ?msg= code from the map OR the text from note-error;
            //   2. verifyReason — if post-verification failed (D4);
            //   3. a neutral fallback with the HTTP status (NOT HTML).
            // The full response HTML is preserved in dump.responseBodyFull for debugging.
            error: opOk ? undefined : (mfExtractMessage(respUrl, respBody) || verifyReason || ('HTTP ' + r.status)),
            dump: {{
                requestUrl: formUrl,
                requestMethod: 'POST',
                requestContentType: 'application/x-www-form-urlencoded',
                requestFields: fields.map(([k,v]) => k + '=' + mfMask(k, v)),
                csrfToken: mfMask('csrf_token', csrf),
                csrfStatus: csrfResult.status,
                csrfInputs: csrfResult.inputs,
                responseStatus: r.status,
                responseRedirected: r.redirected,
                responseUrl: r.url,
                responseHeaders: respHeaders,
                responseBodyLen: respBody.length,
                responseBodyFull: respBody.slice(0, {RESP_BODY_MAX}),
                successMarker: hasSuccess,
                errorMsg: isErrorMsg,
                verified: verified,
                verifyStatus: verifyStatus,
                verifyReason: verifyReason,
                verifyUrl: verifyUrl,
                verifyMs: verifyMs,
                timingMs: Math.round(t2 - t0)
            }}
        }};
    }}
    "
    )
}

/// Collected JS fragments for an operation batch of a single target. Groups the
/// parameters of [`batch_iife_js`] so the signature stays compact.
#[must_use]
pub(super) struct BatchFragments<'a> {
    /// String flag for JS post-verification (`"true"` / `"false"`).
    pub verify_flag: &'a str,
    /// The target's helper functions (mfMask/getCsrf/verifyOp).
    pub helpers_js: &'a str,
    /// JS definition of `buildFields(csrf, row)` — the target's form fields.
    pub build_fields: &'a str,
    /// JS definition of `doOp(base, row)` — the target's single-request pipeline.
    pub do_op: &'a str,
}

/// Final IIFE wrapper of the operation batch: `marker_js` + `base_js` + `error_map_js`
/// + `MF_VERIFY` + helpers + buildFields + doOp + the rows loop + IPC postMessage.
///
/// Identical for all targets; the target module passes its fragments in
/// `fragments`. `id` is the correlation-id for the IPC response
/// (`batch:{id}:{json}`).
#[must_use]
pub(super) fn batch_iife_js(
    id: u64,
    marker_js: &str,
    base_js: &str,
    error_map_js: &str,
    fragments: &BatchFragments<'_>,
    rows_json: &serde_json::Value,
) -> String {
    let BatchFragments {
        verify_flag,
        helpers_js,
        build_fields,
        do_op,
    } = *fragments;
    format!(
        r"(async () => {{
            {marker_js}
            {base_js}
            {error_map_js}
            const MF_VERIFY = {verify_flag};
            {helpers_js}
            {build_fields}
            {do_op}
            try {{
                const base = MF_BASE;
                const rows = {rows_json};
                const results = [];
                for (const row of rows) {{
                    try {{
                        const res = await doOp(base, row);
                        results.push({{username: row.username, domain: row.domain, ok: res.ok, status: res.status, error: res.error, dump: res.dump}});
                    }} catch (e) {{
                        results.push({{username: row.username, domain: row.domain, ok: false, status: 0, error: String(e)}});
                    }}
                }}
                window.ipc.postMessage('batch:{id}:' + JSON.stringify(results));
            }} catch (e) {{
                window.ipc.postMessage('batch:{id}:[]');
            }}
        }})()"
    )
}
