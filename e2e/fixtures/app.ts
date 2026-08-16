// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/**
 * Fixture: launches the MailGrit desktop application and connects Playwright to
 * its WebView2 via the Chrome DevTools Protocol (CDP).
 *
 * Architecture:
 *  1. Locate/build the `.exe` (the debug profile is fast enough and carries the
 *     same RSX/CSS).
 *  2. Copy the `.exe` into a TEMP directory on every test run. The application
 *     stores config.toml/cookie-store in `mailgrit-data/` NEXT TO the binary, so
 *     copying gives each run a clean state (we test persistence honestly, without
 *     polluting the real installation and without runs interfering with each
 *     other).
 *  3. Inject the WebView2 debug port through `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`
 *     BEFORE the process starts. WebView2 brings up Chrome DevTools on that port.
 *  4. spawn the process → wait for the CDP endpoint → chromium.connectOverCDP →
 *     first page.
 *  5. teardown: kill the process, clean up the temp directory.
 *
 * WebView2 opens a SINGLE Chromium instance per process, so workers=1
 * (see playwright.config.ts) — it cannot be run in parallel on a single port.
 *
 * Two test exports:
 *  - `test`          — normal start (login screen). For login tests.
 *  - `testDashboard` — starts in the dashboard state (env `MAILGRIT_E2E_DASHBOARD=1`):
 *    the application skips the login flow and immediately shows the operations
 *    panel with pre-filled test table rows. For dashboard tests (modals, table,
 *    password controls, theme, i18n, a11y, layout). Implemented by the Rust hook
 *    `e2e_state.rs` (a no-op without the env var — production is unaffected).
 */
import { test as base, expect, chromium, type Page } from '@playwright/test';
import { execFile, spawn, type ChildProcess } from 'node:child_process';
import { promisify } from 'node:util';
import { existsSync, mkdirSync, copyFileSync, openSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { CDP_PORT } from '../playwright.config';
import { DASH, SEL } from '../helpers/selectors';

const execFileAsync = promisify(execFile);

/** Path to the built .exe (debug profile). */
function resolveExePath(): string {
  // e2e/ → repo root → target/debug
  const repoRoot = join(__dirname, '..', '..');
  const candidate = join(repoRoot, 'target', 'debug', 'mailgrit-app-desktop.exe');
  if (!existsSync(candidate)) {
    throw new Error(
      `.exe not found: ${candidate}\n` +
        'Build the application:  cargo build -p mailgrit-app-desktop --features e2e\n' +
        '(the --features e2e part is REQUIRED: without it the binary exists but the\n' +
        'MAILGRIT_E2E_DASHBOARD hook is not compiled in, and every dashboard test\n' +
        'then fails with an opaque 20-second sentinel timeout).',
    );
  }
  return candidate;
}

/**
 * Fails fast when something already serves the CDP port — typically a
 * leftover msedgewebview2.exe from an earlier crashed run. Without this
 * check, waitForCdp() succeeds against the STALE browser and the tests fail
 * with misleading page/sentinel errors instead of naming the real problem.
 */
async function assertCdpPortFree(): Promise<void> {
  try {
    const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json/version`);
    if (res.ok) {
      throw new Error(
        `Port ${CDP_PORT} is already serving a DevTools endpoint — a stale ` +
          'msedgewebview2.exe (or another CDP app) holds it. Kill it first, e.g.:\n' +
          '  PowerShell:  Stop-Process -Name msedgewebview2 -Force',
      );
    }
  } catch (e) {
    if (e instanceof TypeError) {
      // fetch network error = connection refused = port free.
      return;
    }
    throw e;
  }
}

/**
 * Copies the .exe to a temporary directory and returns the path to the copy.
 * Data isolation: a fresh mailgrit-data/ is created next to the copy.
 */
function stageIsolatedCopy(srcExe: string): { exe: string; dir: string } {
  const dir = join(tmpdir(), `mailgrit-e2e-${randomBytes(6).toString('hex')}`);
  mkdirSync(dir, { recursive: true });
  const exe = join(dir, 'mailgrit-app-desktop.exe');
  copyFileSync(srcExe, exe);
  return { exe, dir };
}

/** Waits until the CDP endpoint becomes available (WebView2 has brought up DevTools). */
async function waitForCdp(timeoutMs = 30_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`http://127.0.0.1:${CDP_PORT}/json/version`);
      if (res.ok) return;
    } catch {
      // not up yet — keep waiting
    }
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error(
    `CDP endpoint did not respond within ${timeoutMs}ms on port ${CDP_PORT}. ` +
      'Make sure the WebView2 Runtime is installed and the port is free.',
  );
}

export type AppFixture = {
  page: Page;
  /** Temporary directory where the .exe is spawned (mailgrit-data/ sits next to it). */
  dataDir: string;
};

/**
 * Launches an isolated copy of the .exe and connects to its WebView2 over CDP.
 * `dashboardMode=true` → env `MAILGRIT_E2E_DASHBOARD=1` (start in the dashboard).
 * `testInfo` is used to preserve the app's stdio log for failed tests.
 */
async function launchApp(
  use: (f: AppFixture) => Promise<void>,
  dashboardMode: boolean,
  testInfo: { outputDir: string; status?: string },
): Promise<void> {
  const srcExe = resolveExePath();
  // The port check runs BEFORE staging the copy: its whole purpose is the
  // stale-msedgewebview2 case, and a throw after stageIsolatedCopy would
  // leak the full .exe copy into %TEMP% (the try/finally below is not yet
  // active at that point).
  await assertCdpPortFree();
  const { exe, dir } = stageIsolatedCopy(srcExe);

  // WebView2: open Chrome DevTools on a fixed port. The variable is read BEFORE
  // the WebView2 environment is created inside the process. --disable-gpu:
  // CI Windows runners have no interactive GPU session; without it
  // msedgewebview2 may never bring up the DevTools endpoint there (fine on a
  // developer desktop, silently failing on the runner — seen 2026-08-16).
  // WEBVIEW2_USER_DATA_FOLDER: a fresh browser profile per staged copy. When
  // the default profile is reused, a msedgewebview2.exe that survived an
  // earlier launch keeps serving that profile — the new app instance then
  // attaches to the EXISTING browser process and our additional arguments
  // (the debug port) are never applied.
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT} --disable-gpu`,
    WEBVIEW2_USER_DATA_FOLDER: join(dir, 'webview2-profile'),
    RUST_LOG: process.env.RUST_LOG ?? 'warn',
  };
  if (dashboardMode) {
    // E2E hook to start in the dashboard (see crates/app-desktop/src/e2e_state.rs).
    env.MAILGRIT_E2E_DASHBOARD = '1';
  }

  let child: ChildProcess | null = null;
  try {
    // stdout/stderr go to a log file in the temp dir (NOT 'ignore'): when a
    // test fails, the application's own diagnostics are available instead of
    // being discarded.
    const stdioLog = openSync(join(dir, 'app-stdio.log'), 'a');
    const spawned = spawn(exe, { env, cwd: dir, stdio: ['ignore', stdioLog, stdioLog], windowsHide: false });
    child = spawned;
    // A spawn failure (missing exe, AV block) must FAIL the awaiting test —
    // throwing inside this EventEmitter callback would be an uncaught error
    // that kills the runner. Race a rejected promise instead.
    const launchFailed = new Promise<never>((_, reject) => {
      spawned.once('error', (e) => reject(new Error(`failed to launch the .exe: ${e.message}`)));
    });
    await Promise.race([waitForCdp(), launchFailed]);

    const browser = await chromium.connectOverCDP(`http://127.0.0.1:${CDP_PORT}`);
    // WebView2 exposes a single context; take the default one and its first window page.
    const ctx = browser.contexts()[0] ?? (await browser.newContext());

    // The first page is the MailGrit window. Sometimes the context is empty at
    // startup — wait for a page to appear (Dioxus renders after WebView starts).
    let page = ctx.pages()[0] ?? null;
    if (!page) {
      page = await ctx.waitForEvent('page', { timeout: 20_000 });
    }
    await page.waitForLoadState('domcontentloaded');

    // Dioxus mounts the UI asynchronously after domcontentloaded — on a slow
    // CI runner the first test action can race the very first render (seen as
    // a one-off 15s click timeout on the add-row button, 2026-08-16). Wait for
    // the mode's root section before handing the page to the test.
    const sentinel = dashboardMode ? DASH.opsCard : SEL.loginScreen;
    await page.locator(sentinel).waitFor({ state: 'visible', timeout: 20_000 });

    await use({ page, dataDir: dir });

    try {
      await browser.close();
    } catch {
      // The CDP connection may already be broken — not critical.
    }
  } finally {
    // On failure, copy the app's own diagnostics into the Playwright output
    // BEFORE the temp dir is deleted — the stdio-log comment above promises
    // them, and teardown used to destroy them unconditionally.
    if (testInfo.status !== undefined && testInfo.status !== 'passed') {
      try {
        mkdirSync(testInfo.outputDir, { recursive: true });
        copyFileSync(join(dir, 'app-stdio.log'), join(testInfo.outputDir, 'app-stdio.log'));
      } catch {
        // best effort — never mask the original failure
      }
    }
    if (child && !child.killed) {
      if (process.platform === 'win32' && child.pid) {
        // Kill the whole tree AWAITED: child.kill() only terminates the app
        // process, leaving msedgewebview2.exe browser subprocesses behind — a
        // survivor keeps serving the CDP port and the NEXT test would connect
        // to a headless leftover browser instead of its own fresh instance.
        try {
          await execFileAsync('taskkill', ['/pid', String(child.pid), '/T', '/F']);
        } catch {
          // the process may have already exited
        }
      } else {
        try {
          child.kill();
        } catch {
          // the process may have already exited
        }
      }
    }
    // Windows may hold the .exe handle briefly even after taskkill /F
    // completes — retry the removal instead of a fixed sleep.
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        rmSync(dir, { recursive: true, force: true });
        break;
      } catch {
        // After the last attempt: leave the folder in temp (swallowed on
        // purpose — cleanup must not mask a test failure).
        if (attempt < 2) {
          await new Promise((r) => setTimeout(r, 200));
        }
      }
    }
  }
}

/**
 * Extended test object: each test gets a ready connection to the UI
 * (normal start — login screen). For login tests.
 */
export const test = base.extend<{ app: AppFixture }>({
  app: async ({}, use, testInfo) => {
    await launchApp(use, false, testInfo);
  },
});

/**
 * Extended test object for dashboard tests: starts in the dashboard state
 * (env `MAILGRIT_E2E_DASHBOARD=1`) with pre-filled test rows.
 */
export const testDashboard = base.extend<{ app: AppFixture }>({
  app: async ({}, use, testInfo) => {
    await launchApp(use, true, testInfo);
  },
});

export { expect };
