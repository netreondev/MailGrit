//! Standalone JS helper functions (`mfMask`, `getCsrf`) shared by every
//! target's operation pipeline. Extracted from `shared.rs` to keep that file
//! under the 400-line spec.
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/// Masks PII/secrets in the log dump (D5): hides the email local part, display
/// name, and values of secret fields (password, CSRF). Identical for all
/// iRedAdmin OSE forms.
#[must_use]
pub(super) const fn mf_mask_js() -> &'static str {
    r"
    function mfMask(k, v) {
        const s = String(v);
        if (k === 'newpw' || k === 'confirmpw' || k === 'csrf_token') {
            // Full masking: echoing even the first 8 chars of a secret into
            // logs leaks a meaningful prefix of passwords/CSRF tokens.
            return s.length === 0 ? '(empty)' : '***(' + s.length + ')';
        }
        if (k === 'mail' || k === 'username') {
            const at = s.lastIndexOf('@');
            if (at > 0) {
                const local = s.substring(0, at);
                const domain = s.substring(at);
                const head = local.length > 0 ? local.charAt(0) : '';
                return head + '***' + domain;
            }
            return s.length <= 1 ? '***' : s.charAt(0) + '***';
        }
        if (k === 'cn') {
            return s.length <= 1 ? '***' : s.charAt(0) + '***';
        }
        return s;
    }
    "
}

/// JS function `getCsrf(formUrl, formActionSuffix)`: GETs the form page, parses
/// the CSRF token bound to a specific form (D6). Identical for all targets;
/// `log_tag` only differentiates the log line (`[CSRF]` / `[CSRF-DOMAIN]` /
/// `[CSRF-ADMIN]`).
#[must_use]
pub(super) fn get_csrf_js(log_tag: &str) -> String {
    format!(
        r#"
    async function getCsrf(formUrl, formActionSuffix) {{
        const t0 = performance.now();
        try {{
            const r = await fetch(formUrl, {{credentials:'include'}});
            const html = await r.text();
            const doc = new DOMParser().parseFromString(html, 'text/html');
            let csrfEl = null;
            if (formActionSuffix) {{
                const form = doc.querySelector('form[action$="' + formActionSuffix + '"]');
                if (form) csrfEl = form.querySelector('input[name="csrf_token"]');
            }}
            if (!csrfEl) csrfEl = doc.querySelector('input[name="csrf_token"]');
            const token = csrfEl ? (csrfEl.getAttribute('value') || csrfEl.value || '') : '';
            const allInputs = Array.from(doc.querySelectorAll('input,select')).map(i => i.name + '(' + i.type + ')');
            const ms = Math.round(performance.now() - t0);
            console.log('[{log_tag}] url=' + formUrl + ' status=' + r.status + ' token=' + (token ? 'present(' + token.length + ')' : 'EMPTY') + ' inputs=[' + allInputs.join(',') + '] ' + ms + 'ms');
            return {{token: token, status: r.status, inputs: allInputs, ms: ms, htmlHead: html.slice(0, 800)}};
        }} catch(e) {{
            console.error('[{log_tag}] ERROR ' + e);
            return {{token: '', error: String(e)}};
        }}
    }}
    "#
    )
}
