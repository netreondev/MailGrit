// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { defineConfig } from '@playwright/test';

/**
 * Playwright config for the MailGrit desktop E2E suite.
 *
 * Approach: we test the built `.exe` (Dioxus → WebView2) by connecting to its
 * Chromium instance via the Chrome DevTools Protocol (CDP). The debug port is
 * injected through the WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS environment
 * variable BEFORE the process starts (see fixtures/app.ts).
 *
 * This is the ONLY place where the CDP port is defined — a single source of
 * truth.
 */
export const CDP_PORT = 9333;

export default defineConfig({
  testDir: './tests',
  fullyParallel: false, // only one desktop process at a time (shared CDP port)
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
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
