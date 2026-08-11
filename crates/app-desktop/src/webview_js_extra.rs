//! JS builder for form diagnostics (an extra operation via the login-webview).
//!
//! Extracted from [`crate::webview_js`] to comply with the spec's file-size limit
//! of ≤400 lines. The main batch builder (`build_batch_js`) stays in
//! [`crate::webview_js`]; here is only the non-operation builder: form
//! diagnostics (`build_diag_js`). It follows the same pattern: IIFE → fetch →
//! `window.ipc.postMessage`.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

/// JS for diagnostics: GET the user-creation form page and return the HTML.
/// Shows the real field names, action URL, and CSRF — to know exactly which
/// parameters to send in the POST. The result comes via window.ipc.postMessage.
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
                // Extract all input/select from forms + action for analysis.
                const forms = Array.from(document.querySelectorAll('form')).map(f => ({{
                    action: f.action,
                    method: f.method,
                    inputs: Array.from(f.querySelectorAll('input,select,textarea')).map(i => ({{
                        name: i.name, type: i.type, value: i.type==='hidden'?i.value.slice(0,50):'(user input)'
                    }}))
                }}));
                // Also parse the form from the fetched HTML (since we fetched it rather than being on that page).
                const doc = new DOMParser().parseFromString(html, 'text/html');
                const fetchedForms = Array.from(doc.querySelectorAll('form')).map(f => ({{
                    action: f.action, method: f.method,
                    inputs: Array.from(f.querySelectorAll('input,select,textarea')).map(i => ({{
                        name: i.name, type: i.type, value: i.type==='hidden'?i.value.slice(0,50):'(user input)'
                    }}))
                }}));
                window.ipc.postMessage('diag:{id}:' + JSON.stringify({{
                    status: r.status,
                    url: formUrl,
                    forms_on_page: forms,
                    forms_in_response: fetchedForms,
                    html_excerpt: html.slice(0, 3000)
                }}));
            }} catch (e) {{
                window.ipc.postMessage('diag:{id}:' + JSON.stringify({{error: String(e)}}));
            }}
        }})()"
    )
}
