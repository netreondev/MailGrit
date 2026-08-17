// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/**
 * Batch-JS smoke harness — EXECUTES the real generated webview JS.
 *
 * The Rust-side tests over the batch builder are string-presence checks
 * (`js.contains("finalOk = ...")`) — they cannot catch a syntactically valid
 * but LOGICALLY broken builder. This harness closes that gap: it asks the
 * real binary (built with --features e2e) to emit the exact IIFE it would
 * evaluate inside the webview, then runs it in Node against a mocked
 * `fetch`/`window`/`DOMParser` and asserts the observable behavior:
 *
 *   1. happy path (create + post-verification)      → ok, verified, masked dump
 *   2. server error (?msg=ALREADY_EXISTS, HTTP 200)  → ok=false, human message
 *   3. session expiry during the verify-GET          → verifyUrl carries /login
 *   4. missing CSRF token in the form                → ok=false, structured dump
 *
 * Usage: npm run test:jsmoke   (from e2e/; the exe must be built with
 * `cargo build -p mailgrit-app-desktop --features e2e`).
 */
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..', '..');
const EXE = join(REPO_ROOT, 'target', 'debug', 'mailgrit-app-desktop.exe');
if (!existsSync(EXE)) {
  console.error(
    `exe not found: ${EXE}\n` +
      'Build first:  cargo build -p mailgrit-app-desktop --features e2e',
  );
  process.exit(1);
}

// Mirrors the base URL hardcoded in main.rs's --emit-batch-js path: the mocks
// below must produce URLs the emitted JS treats as same-origin/expected.
// Single constant so the five use sites cannot drift from each other.
const BASE_URL = 'https://mail.example.com';

// ---------------------------------------------------------------------------
// Minimal browser shims (the generated JS touches exactly these).
// ---------------------------------------------------------------------------

/** A parsed element: tag + attributes; attribute-derived `name`/`type`/`value`. */
class MiniElement {
  constructor(tag, attrs) {
    this.tag = tag;
    this.attrs = attrs;
    this.name = attrs.name ?? '';
    this.type = attrs.type ?? (tag === 'input' ? 'text' : tag);
    this.value = attrs.value ?? '';
  }
  getAttribute(name) {
    return Object.hasOwn(this.attrs, name) ? this.attrs[name] : null;
  }
  /** Absolute form action (diag/CSRF code only reads it for reporting). */
  get action() {
    const a = this.attrs.action ?? '';
    return /^https?:/.test(a) ? a : `${BASE_URL}${a}`;
  }
  get method() {
    return (this.attrs.method ?? 'get').toLowerCase();
  }
}

class MiniForm extends MiniElement {
  constructor(attrs, inner) {
    super('form', attrs);
    this.inner = inner;
  }
  /** Supports the selector subset the generated JS uses on forms. */
  querySelectorAll(sel) {
    return selectInputs(this.inner).filter((el) => matchesSimple(el, sel));
  }
  querySelector(sel) {
    return this.querySelectorAll(sel)[0] ?? null;
  }
}

/** Supports the selector subset the generated JS uses on the document. */
class MiniDocument {
  constructor(html) {
    this.forms = [];
    const formRe = /<form\b([^>]*)>([\s\S]*?)<\/form>/gi;
    let m;
    while ((m = formRe.exec(html)) !== null) {
      this.forms.push(new MiniForm(parseAttrs(m[1]), m[2]));
    }
    this.allInputs = selectInputs(html);
  }
  querySelectorAll(sel) {
    if (sel === 'form') return this.forms;
    if (sel.startsWith('form[')) return this.forms.filter((f) => attrSuffixMatch(f, sel));
    return this.allInputs.filter((el) => matchesSimple(el, sel));
  }
  querySelector(sel) {
    return this.querySelectorAll(sel)[0] ?? null;
  }
}

globalThis.DOMParser = class {
  parseFromString(html) {
    return new MiniDocument(html);
  }
};

function parseAttrs(s) {
  const attrs = {};
  const re = /([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*("([^"]*)"|'([^']*)')/g;
  let m;
  while ((m = re.exec(s)) !== null) {
    attrs[m[1]] = m[3] ?? m[4] ?? '';
  }
  return attrs;
}

function selectInputs(html) {
  const out = [];
  const re = /<(input|select|textarea)\b([^>]*)>/gi;
  let m;
  while ((m = re.exec(html)) !== null) {
    out.push(new MiniElement(m[1].toLowerCase(), parseAttrs(m[2])));
  }
  return out;
}

/** `input[name="csrf_token"]` — one attribute-equality term. */
function matchesSimple(el, sel) {
  const m = sel.match(/^([a-z]+)\[([a-zA-Z-]+)="([^"]*)"\]$/i);
  if (m) return el.tag === m[1].toLowerCase() && el.attrs[m[2]] === m[3];
  return sel.split(',').some((part) => part.trim().toLowerCase() === el.tag);
}

/** `form[action$="suffix"]` — ends-with attribute selector. */
function attrSuffixMatch(form, sel) {
  const m = sel.match(/^form\[action\$="([^"]*)"\]$/i);
  return m ? (form.attrs.action ?? '').endsWith(m[1]) : false;
}

/** A fetch Response shim with exactly the surface the generated JS reads. */
function resp({ status = 200, url = '', body = '', headers = {} } = {}) {
  return {
    ok: status >= 200 && status < 300,
    status,
    url,
    redirected: false,
    headers: {
      forEach(cb) {
        for (const [k, v] of Object.entries(headers)) cb(v, k);
      },
    },
    text: async () => body,
  };
}

// ---------------------------------------------------------------------------
// Emitted-JS acquisition + execution.
// ---------------------------------------------------------------------------

/** Rows in CSV column order: domain, username, password, display_name, quota. */
const ROWS = [
  ['example.com', 'smoke.user', 'Sm0kePw!xyz', 'Smoke User', '1024'],
  ['example.com', 'smoke.two', 'An0therPw!ok', 'Smoke Two', '2048'],
];

function emitBatchJs(target, kind, verify) {
  const dir = mkdtempSync(join(tmpdir(), 'mailgrit-jsmoke-'));
  try {
    const rowsFile = join(dir, 'rows.json');
    writeFileSync(rowsFile, JSON.stringify(ROWS));
    return execFileSync(EXE, ['--emit-batch-js', rowsFile, target, kind, verify ? '1' : '0'], {
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** Installs the window/fetch shims, executes the emitted IIFE, returns the parsed IPC frame. */
async function runJs(js, fetchMock) {
  const posted = [];
  globalThis.window = {
    location: { origin: BASE_URL, pathname: '/iredadmin/dashboard' },
    ipc: { postMessage: (s) => posted.push(s) },
  };
  globalThis.fetch = fetchMock;
  // The emitted string is `(async () => { ... })()` — evaluating the wrapped
  // expression returns the IIFE's promise; awaiting it waits for the IPC frame.
  await (0, eval)(`(${js})`); // eslint-disable-line no-eval
  assert.equal(posted.length, 1, `exactly one IPC frame (got ${posted.length})`);
  const frame = posted[0];
  assert.ok(frame.startsWith('batch:1:'), `frame must be batch:1:… (got ${frame.slice(0, 40)}…)`);
  return JSON.parse(frame.slice('batch:1:'.length));
}

// ---------------------------------------------------------------------------
// Scenarios.
// ---------------------------------------------------------------------------

const CSRF_FORM = (domain) =>
  `<html><form action="/iredadmin/create/user/${domain}" method="post">` +
  `<input type="hidden" name="csrf_token" value="TOKEN-abc123def456">` +
  `<input type="text" name="username"><input type="password" name="newpw"></form></html>`;

/** Scenario 1: happy path — create + post-verification succeeds. */
async function scenarioHappyCreate() {
  const calls = [];
  const js = emitBatchJs('user', 'create', true);
  const results = await runJs(js, async (url, opts) => {
    calls.push([opts?.method ?? 'GET', url]);
    if ((opts?.method ?? 'GET') === 'POST') {
      return resp({ status: 200, url: `${url.split('?')[0]}?msg=CREATED`, body: '<div class="note-success">ok</div>' });
    }
    if (url.includes('/create/user/')) {
      return resp({ status: 200, body: CSRF_FORM('example.com') }); // the form GET (CSRF)
    }
    if (url.includes('/profile/user/general/')) {
      return resp({ status: 200, url, body: '<html>smoke.user@example.com</html>' }); // verify-GET
    }
    throw new Error(`unexpected fetch: ${url}`);
  });

  // Sequence per row: form-GET (CSRF) → POST → verify-GET.
  assert.equal(results.length, 2, 'one result per row');
  assert.equal(calls.length, 6, '2 rows × (CSRF GET + POST + verify GET)');
  for (const r of results) {
    assert.equal(r.ok, true, `create must succeed: ${JSON.stringify(r.error)}`);
    assert.equal(r.status, 200);
    assert.equal(r.dump.verified, true, 'post-verification confirms the profile page');
  }
  // The dump must be MASKED: neither the password nor the CSRF token verbatim.
  const dumpStr = JSON.stringify(results[0].dump);
  assert.ok(!dumpStr.includes('Sm0kePw!xyz'), 'dump must not contain the plaintext password');
  assert.ok(!dumpStr.includes('TOKEN-abc123def456'), 'dump must not contain the CSRF token');
  assert.match(results[0].dump.csrfToken, /\*\*\*\(\d+\)/, 'csrfToken is length-masked');
  // The POST body carried the real password + token (the request side works).
  const dumpFields = results[0].dump.requestFields.join(' ');
  assert.match(dumpFields, /newpw=\*\*\*\(\d+\)/, 'the password field is masked-with-length in the dump');
}

/** Scenario 2: server error on HTTP 200 (?msg=ALREADY_EXISTS) — the iRedAdmin way. */
async function scenarioServerError() {
  const js = emitBatchJs('user', 'create', false);
  const results = await runJs(js, async (url, opts) => {
    if ((opts?.method ?? 'GET') === 'POST') {
      // The POST "succeeds" transport-wise but reports an error code.
      return resp({ status: 200, url: `${BASE_URL}/iredadmin/create/user/example.com?msg=ALREADY_EXISTS`, body: '' });
    }
    return resp({ status: 200, body: CSRF_FORM('example.com') });
  });

  assert.equal(results.length, 2);
  for (const r of results) {
    assert.equal(r.ok, false, 'ALREADY_EXISTS must not be treated as success');
    assert.equal(r.status, 200);
    assert.ok(typeof r.error === 'string' && r.error.length > 0, 'a human-readable error is set');
    assert.ok(!r.error.includes('<'), 'the error must not be raw HTML');
  }
}

/** Scenario 3: session expires between POST and verify-GET → verifyUrl carries /login. */
async function scenarioSessionExpiryInVerify() {
  const js = emitBatchJs('user', 'create', true);
  const results = await runJs(js, async (url, opts) => {
    if ((opts?.method ?? 'GET') === 'POST') {
      return resp({ status: 200, url: `${BASE_URL}/iredadmin/create/user/example.com?msg=CREATED`, body: '' });
    }
    if (url.includes('/profile/user/general/')) {
      // The verify-GET is redirected to the login page (session died in between).
      return resp({ status: 200, url: `${BASE_URL}/iredadmin/login?msg=LOGIN_REQUIRED`, body: '' });
    }
    return resp({ status: 200, body: CSRF_FORM('example.com') });
  });

  assert.equal(results.length, 2);
  for (const r of results) {
    assert.equal(r.ok, false, 'unverified create must not be reported as ok');
    assert.ok(
      r.dump.verifyUrl.includes('/login'),
      `dump.verifyUrl must carry the /login redirect for the Rust session-expiry detector (got ${r.dump.verifyUrl})`,
    );
  }
}

/** Scenario 4: the form has no CSRF token → structured failure. The failure
 * dump embeds csrfResult.htmlHead — the page head with every value= attribute
 * redacted. The canaries cover BOTH quoting forms (unquoted value=token is
 * valid HTML5); neither may survive into the dump. */
async function scenarioMissingCsrf() {
  const js = emitBatchJs('user', 'create', false);
  const results = await runJs(js, async () =>
    resp({
      status: 200,
      body:
        '<html><form action="/x" method="post"><input name="username">' +
        '<input type="hidden" name="legacy" value="QUOTED-canary-456">' +
        '<input type="hidden" name=legacy2 value=UNQUOTED-canary-987>' +
        '</form></html>',
    }),
  );

  assert.equal(results.length, 2);
  for (const r of results) {
    assert.equal(r.ok, false);
    assert.equal(r.status, 0);
    assert.match(r.error, /CSRF token not found/, 'the reason names the missing CSRF');
    assert.ok(r.dump && r.dump.csrfResult, 'the csrfResult is embedded for diagnosis');
    const dumpStr = JSON.stringify(r.dump);
    assert.ok(!dumpStr.includes('QUOTED-canary-456'), 'a quoted value= must be redacted in htmlHead');
    assert.ok(!dumpStr.includes('UNQUOTED-canary-987'), 'an unquoted value= must be redacted in htmlHead (valid HTML5)');
    assert.ok(r.dump.csrfResult.htmlHead.includes('value="(redacted)"'), 'the redaction marker is present');
  }
}

// ---------------------------------------------------------------------------
// Runner.
// ---------------------------------------------------------------------------

const scenarios = [
  ['happy create + verify', scenarioHappyCreate],
  ['server error (ALREADY_EXISTS on HTTP 200)', scenarioServerError],
  ['session expiry in the verify window', scenarioSessionExpiryInVerify],
  ['missing CSRF token', scenarioMissingCsrf],
];

let failed = 0;
for (const [name, fn] of scenarios) {
  try {
    await fn();
    console.log(`PASS  ${name}`);
  } catch (e) {
    failed++;
    // The stack localizes the failing assert inside a multi-step scenario —
    // the bare message does not.
    console.error(`FAIL  ${name}\n      ${e.stack ?? e.message}`);
  }
}
if (failed > 0) {
  console.error(`\n${failed}/${scenarios.length} smoke scenarios FAILED`);
  process.exit(1);
}
console.log(`\nall ${scenarios.length} batch-JS smoke scenarios passed`);
