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
import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync, mkdirSync, copyFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { randomBytes } from 'node:crypto';
import { CDP_PORT } from '../playwright.config';

/** Path to the built .exe (debug profile). */
function resolveExePath(): string {
  // e2e/ → repo root → target/debug
  const repoRoot = join(__dirname, '..', '..');
  const candidate = join(repoRoot, 'target', 'debug', 'mailgrit-app-desktop.exe');
  if (!existsSync(candidate)) {
    throw new Error(
      `.exe not found: ${candidate}\n` +
        'Build the application:  cargo build -p mailgrit-app-desktop',
    );
  }
  return candidate;
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
 */
async function launchApp(use: (f: AppFixture) => Promise<void>, dashboardMode: boolean): Promise<void> {
  const srcExe = resolveExePath();
  const { exe, dir } = stageIsolatedCopy(srcExe);

  // WebView2: open Chrome DevTools on a fixed port. The variable is read BEFORE
  // the WebView2 environment is created inside the process.
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT}`,
    RUST_LOG: process.env.RUST_LOG ?? 'warn',
  };
  if (dashboardMode) {
    // E2E hook to start in the dashboard (see crates/app-desktop/src/e2e_state.rs).
    env.MAILGRIT_E2E_DASHBOARD = '1';
  }

  let child: ChildProcess | null = null;
  try {
    child = spawn(exe, { env, cwd: dir, stdio: 'ignore', windowsHide: false });
    child.once('error', (e) => {
      throw new Error(`failed to launch the .exe: ${e.message}`);
    });

    await waitForCdp();

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

    await use({ page, dataDir: dir });

    try {
      await browser.close();
    } catch {
      // The CDP connection may already be broken — not critical.
    }
  } finally {
    if (child && !child.killed) {
      try {
        child.kill();
      } catch {
        // the process may have already exited
      }
    }
    // Give the OS time to release the .exe (Windows holds the file handle) before deleting.
    await new Promise((r) => setTimeout(r, 400));
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // Windows sometimes does not release the .exe immediately — leave the folder in temp.
    }
  }
}

/**
 * Extended test object: each test gets a ready connection to the UI
 * (normal start — login screen). For login tests.
 */
export const test = base.extend<{ app: AppFixture }>({
  app: async ({}, use) => {
    await launchApp(use, false);
  },
});

/**
 * Extended test object for dashboard tests: starts in the dashboard state
 * (env `MAILGRIT_E2E_DASHBOARD=1`) with pre-filled test rows.
 */
export const testDashboard = base.extend<{ app: AppFixture }>({
  app: async ({}, use) => {
    await launchApp(use, true);
  },
});

export { expect };
