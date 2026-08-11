// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';

/**
 * iRedAdmin server URL validation on the login screen (util.rs::validate_base_url).
 *
 * This is real, E2E-driven UI logic (unlike CSV upload, which requires a
 * dashboard behind a live iRedAdmin + a native rfd dialog not accessible to
 * Playwright). We verify: an incorrect/non-https URL -> a localized error;
 * a valid https URL -> entering the awaiting-login state (without opening an
 * external window in a headless environment — but the UI status changes).
 */
test.describe('Server URL validation', () => {
  test('rejects non-https URL with a localized error', async ({ app }) => {
    const { page } = app;
    const input = page.locator(SEL.serverInput);
    const btn = page.locator(SEL.openFormButton).first();

    await input.fill('http://mail.example.com/iredadmin');
    await btn.click();

    // An error banner appears mentioning https.
    const banner = page.locator('.poll-banner.error-banner, .error-banner');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText(/https/i);
  });

  test('rejects malformed input', async ({ app }) => {
    const { page } = app;
    const input = page.locator(SEL.serverInput);
    const btn = page.locator(SEL.openFormButton).first();

    await input.fill('not-a-url');
    await btn.click();

    const banner = page.locator('.poll-banner.error-banner, .error-banner');
    await expect(banner).toBeVisible();
  });

  test('accepts a valid https URL and enters awaiting state', async ({ app }) => {
    const { page } = app;
    const input = page.locator(SEL.serverInput);
    const btn = page.locator(SEL.openFormButton).first();

    await input.fill('https://mail.example.com/iredadmin');
    await btn.click();

    // No error is shown (validation passed).
    await expect(page.locator('.error-banner')).toHaveCount(0);

    // Entered the awaiting-login state — the "Waiting..." (en) banner is visible.
    // The form button is disabled in AwaitingLogin.
    await expect(btn).toBeDisabled();
    await expect(page.getByText(/waiting/i)).toBeVisible();
  });
});
