// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

use super::build_diag_js;

// IIFE structure and brace balance (same contract as the batch builders).
#[test]
fn diag_js_is_balanced_iife() {
    let js = build_diag_js(5, "example.com");
    assert!(js.starts_with("(async () => {"));
    assert!(js.ends_with(")()"));
    assert_eq!(js.matches('{').count(), js.matches('}').count());
}

// The IPC frame carries the request id so `dispatch` can route the reply.
#[test]
fn diag_js_uses_ipc_frame_with_id() {
    let js = build_diag_js(42, "example.com");
    assert!(js.contains("'diag:42:'"), "frame must be diag:42:");
}

// The domain reaches JS only through serde_json interpolation (injection-safe).
#[test]
fn diag_js_embeds_domain_as_json_string() {
    let js = build_diag_js(1, "example.com");
    assert!(
        js.contains("\"example.com\""),
        "domain must be embedded as a JSON string literal"
    );
}

// SECURITY: hidden input VALUES must not be collected — a create-form hidden
// value IS the CSRF token, and the diag response is logged at INFO (ops.rs).
// A `value` pass-through (even slice(0,N)) would leak the token prefix/full.
#[test]
fn diag_js_never_collects_hidden_input_values() {
    let js = build_diag_js(1, "example.com");
    assert!(
        !js.contains("i.value.slice"),
        "diag JS slices a hidden value into the report — CSRF leak"
    );
    assert!(
        !js.contains(": i.value") && !js.contains("value:"),
        "diag JS collects an input value verbatim — potential secret leak:\n{js}"
    );
    assert!(
        js.contains("value_len: i.type==='hidden'?i.value.length:null"),
        "hidden inputs must report the value LENGTH only"
    );
}

// SECURITY: no raw HTML excerpt may be embedded — it carries the CSRF token in
// a hidden value attribute (same reason as above).
#[test]
fn diag_js_has_no_raw_html_excerpt() {
    let js = build_diag_js(1, "example.com");
    assert!(
        !js.contains("html_excerpt") && !js.contains("html.slice"),
        "diag JS embeds raw HTML — CSRF leak"
    );
}

// The dashboard-DOM scan (`document.querySelectorAll` on the CURRENT page) was
// noise: it described the dashboard, not the fetched create form.
#[test]
fn diag_js_does_not_scan_current_page_dom() {
    let js = build_diag_js(1, "example.com");
    assert!(
        !js.contains("document.querySelectorAll"),
        "diag JS reads the dashboard DOM instead of the fetched form"
    );
    assert!(
        js.contains("forms_in_response"),
        "the fetched form is reported"
    );
}
