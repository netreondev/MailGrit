// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { defineConfig } from '@playwright/test';

/**
 * Playwright config for the MailGrit desktop E2E suite.
 *
 * Approach: we test the built `.exe` (Dioxus → WebView2) by connecting to its
 * Chromium instance via the Chrome DevTools Protocol (CDP). The debug port is
 * injected through the WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS environment
 * variable BEFORE the process starts (see fixtures/app.ts).
 *
 * The default here is the single source of truth for LOCAL runs; CI injects
 * the same value through the CDP_PORT env var (workflow-level in ci.yml, also
 * interpolated into the HKLM policy and the preflight probe), so the four
 * places can never drift apart.
 */
function resolveCdpPort(): number {
  const raw = process.env.CDP_PORT;
  if (raw === undefined || raw === '') return 9333;
  // Fail loudly on garbage instead of producing 0/NaN, which later surfaces as
  // an unrelated "did not respond within 30000ms on port NaN" timeout.
  const port = Number(raw);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`CDP_PORT must be an integer in [1, 65535], got: ${JSON.stringify(raw)}`);
  }
  return port;
}

export const CDP_PORT = resolveCdpPort();

export default defineConfig({
  testDir: './tests',
  fullyParallel: false, // only one desktop process at a time (shared CDP port)
  forbidOnly: !!process.env.CI,
  retries: 0, // no retries anywhere: a flaky test must FAIL loudly, not pass on the second attempt
  workers: 1, // strictly one worker: one .exe + one CDP port
  reporter: [['list'], ['html', { open: 'never' }]],
  timeout: 60_000,
  expect: { timeout: 10_000 },
  use: {
    actionTimeout: 15_000,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
  },
  projects: [
    {
      name: 'desktop',
      use: {},
    },
  ],
});
