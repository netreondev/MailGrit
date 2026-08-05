# MailGrit — E2E (Playwright + CDP)

End-to-end tests for the MailGrit desktop application. Connection to the built
`.exe` (Dioxus -> WebView2) happens via the **Chrome DevTools Protocol** — no
separate browser; the real UI in the real WebView2 is tested.

> Isolated from the Rust build: it does not touch the workspace and introduces no
> Node/JS toolchain into the build. All files live only in this folder.

**Українська:** [README.uk.md](README.uk.md)

## Prerequisites

- **Node.js >= 20** (on the developer machine; not needed in the Rust build/CI)
- **WebView2 Runtime** (preinstalled on Windows 11)
- A built `.exe`: `cargo build -p mailgrit-app-desktop` (debug is enough)

## Run

```bash
cd e2e
npm ci
npx playwright install chromium   # once — installs the Playwright CDP client
npm test                          # all tests
npm run test:headed               # with a visible window (handy for debugging)
npm run report                    # HTML report of the last run
```

## How it works

1. `fixtures/app.ts` finds the `.exe` and **copies it to a temporary directory**
   (isolating `mailgrit-data/` — every run starts with a clean config/cookie store).
2. Before spawn, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333`
   is injected.
3. After the process starts, we wait for the CDP endpoint ->
   `chromium.connectOverCDP(...)` -> take the MailGrit window page -> pass it to
   the test as `{ page }`.
4. Teardown kills the process and deletes the temporary directory.

`workers: 1` (one `.exe` + one CDP port at a time).

## Test suite

### Login screen (normal start)

| File | What it checks |
|------|----------------|
| `launch.spec.ts` | startup, titlebar, login screen, window title |
| `branding.spec.ts` | SVG logo (the "Forged Spark"), gradient, "MailGrit" text |
| `language.spec.ts` | **regression** of the language-selector dropdown bug |
| `i18n.spec.ts` | loop over the 9 languages, localized text changes |
| `theme.spec.ts` | dark/light, persistence in config.toml |
| `url-validation.spec.ts` | server-URL validation (real UI logic) |
| `window-controls.spec.ts` | minimize/maximize buttons |

### Dashboard (start via the `MAILGRIT_E2E_DASHBOARD` env hook)

These spec files import `testDashboard` from `fixtures/app` — the app starts
directly in the dashboard state with prefilled test rows (bypassing the iRedAdmin
login flow). The focus is on assessing the **quality, symmetry, usability, and
clarity** of every screen without a network round-trip.

| File | What it checks |
|------|----------------|
| `dashboard.spec.ts` | dashboard access, section navigation, target, logout |
| `dashboard-layout.spec.ts` | **symmetry**: grid centering, no overlaps, colgroup |
| `dashboard-theme.spec.ts` | **theme/contrast**: dark/light tokens, WCAG AA >= 4.5, surface != bg |
| `modals.spec.ts` | modals (delete/regenerate/master-password): ARIA, centering, close |
| `editable-table.spec.ts` | table: add/edit/delete, validation (highlight), per-row password |
| `password-controls.spec.ts` | length slider, policy-locked checkboxes, fill-empty, regenerate-all |
| `dashboard-i18n.spec.ts` | localization of every screen across 9 languages, no broken keys |
| `a11y.spec.ts` | **accessibility**: ARIA roles, accessible names, focus-ring, h2 semantics |

Quality-assessment helpers — `helpers/layout.ts` (`assertContrast`,
`assertCenteredBoth`, `assertSymmetricMargins`, `parseColor`, `contrastRatio`,
`assertNoRawKey`).

## Dashboard startup E2E hook (`MAILGRIT_E2E_DASHBOARD`)

The dashboard is reachable in the real app only through the login-webview
auto-detection of the login (navigation to `/dashboard`), and the Dioxus
`Signal<AppState>` is not accessible from CDP/JS directly. So for E2E coverage of
the dashboard without a live server, a test hook is implemented in Rust
(`crates/app-desktop/src/e2e_state.rs`):

- Activated **only** by the `MAILGRIT_E2E_DASHBOARD=1` env variable.
- Without the env (production) — a complete no-op: the app starts on the login
  screen as usual. The hook does not affect the release build.
- When activated, it parses an embedded valid CSV (2 rows) through the same
  canonical parser (`parse_csv_bytes_auto`) used by user uploads, and sets
  `screen=Dashboard`, `session_ok=true`, `auth_status=Connected`, prefilled
  `editable_rows`/`csv`/`column_mapping`.
- Applied once at startup (`use_hook`), not on every render — otherwise logout
  (resetting `screen=Login`) would immediately roll back.

The `testDashboard` fixture in `fixtures/app.ts` sets this env before spawn.

## What is not tested

- A real login to iRedAdmin (needs a live server) — out of scope. The dashboard is
  tested through the startup env hook (see above), not through a real login.
- CSV loading via the native `rfd` dialog (not accessible to Playwright). Instead
  the table is prefilled by the env hook; the CSV parser is covered by Rust
  unit/property/fuzz tests.
- The network round-trip of bulk operations (create/edit/delete against
  iRedAdmin). Operations run as JS `fetch` inside a separate login-webview; the
  success/error-marker contract is covered by Rust tests
  (`webview_markers_tests.rs`).
