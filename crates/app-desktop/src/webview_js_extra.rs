//! JS builder for form diagnostics (an extra operation via the login-webview).
//!
//! Extracted from [`crate::webview_js`] to comply with the spec's file-size limit
//! of ≤400 lines. The main batch builder (`build_batch_js`) stays in
//! [`crate::webview_js`]; here is only the non-operation builder: form
//! diagnostics (`build_diag_js`). It follows the same pattern: IIFE → fetch →
//! `window.ipc.postMessage`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// JS for diagnostics: GET the user-creation form page and report its structure.
/// Shows the real field names, action URL, and hidden-value LENGTHS (a present
/// CSRF token shows up as a non-zero `value_len`) — to know exactly which
/// parameters to send in the POST. The result comes via window.ipc.postMessage.
///
/// SECURITY: hidden input VALUES and raw HTML must never leave the page. The
/// response is logged at INFO (`ops.rs`) and a create-form HTML embeds the CSRF
/// token in a hidden `value="..."` attribute — the same secret `mfMask` refuses
/// to log even a prefix of. Field name/type/length is all the diagnosis needs.
pub fn build_diag_js(id: u64, domain: &str) -> String {
    let domain = serde_json::Value::String(domain.to_string());
    // MF_BASE is the fragment shared with the batch-IIFE for computing the
    // iRedAdmin base URL (see webview_markers::build_base_js). Previously the
    // '/iredadmin' string and the prefix-computation logic were duplicated here
    // and in batch_iife_js.
    let base_js = crate::webview_markers::build_base_js();
    format!(
        r"(async () => {{
            {base_js}
            try {{
                const formUrl = MF_BASE + '/create/user/' + encodeURIComponent({domain});
                const r = await fetch(formUrl, {{credentials:'include'}});
                const html = await r.text();
                // Parse the form from the fetched HTML (we fetched it rather than
                // being on that page). Values are NOT collected: only the name,
                // type, and — for hidden inputs — the value LENGTH.
                const doc = new DOMParser().parseFromString(html, 'text/html');
                const fetchedForms = Array.from(doc.querySelectorAll('form')).map(f => ({{
                    action: f.action, method: f.method,
                    inputs: Array.from(f.querySelectorAll('input,select,textarea')).map(i => ({{
                        name: i.name, type: i.type,
                        value_len: i.type==='hidden'?i.value.length:null
                    }}))
                }}));
                window.ipc.postMessage('diag:{id}:' + JSON.stringify({{
                    status: r.status,
                    url: formUrl,
                    forms_in_response: fetchedForms
                }}));
            }} catch (e) {{
                window.ipc.postMessage('diag:{id}:' + JSON.stringify({{error: String(e)}}));
            }}
        }})()"
    )
}

#[cfg(test)]
#[path = "webview_js_extra_tests.rs"]
mod tests;
