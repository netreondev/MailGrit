// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';

// The login-screen class (a single selector; we don't pull in all of SEL for it).
const SEL_LOGIN_SCREEN = '.login-screen';

/**
 * Dashboard access and navigation (Operations/Audit sections, target, logout).
 *
 * The dashboard starts via the env hook `MAILGRIT_E2E_DASHBOARD=1` (see
 * fixtures/app.ts `testDashboard`) — the application skips the login flow and
 * immediately shows the operations panel with pre-filled test rows. Here we
 * verify "usability": root elements are visible, switching sections changes the
 * content, the target switches, and logout returns to the login screen.
 */
test.describe('Dashboard — access and navigation', () => {
  test('dashboard is reachable: context-bar, cards, session badge', async ({ app }) => {
    const { page } = app;

    // The dashboard root instead of the login screen.
    await expect(page.locator(DASH.root)).toBeVisible();
    await expect(page.locator(SEL_LOGIN_SCREEN)).toHaveCount(0);

    // Context-bar: session badge (success — the env hook set session_ok=true),
    // base_url, language/theme/logout buttons.
    await expect(page.locator(DASH.contextBar)).toBeVisible();
    await expect(page.locator(DASH.badge)).toBeVisible();
    await expect(page.locator(DASH.baseUrl)).toContainText('mail.example.com');

    // Three grid cards: CSV, operations, result (span-all).
    await expect(page.locator(DASH.dashGrid)).toBeVisible();
    const cards = page.locator(DASH.cards);
    await expect(cards).toHaveCount(3);
    await expect(page.locator(DASH.spanAll)).toBeVisible();
  });

  test('switching sections Operations <-> Audit changes the content', async ({ app }) => {
    const { page } = app;

    // Default — Operations (CSV/ops/result cards).
    await expect(page.locator(DASH.dashGrid)).toBeVisible();
    await expect(page.locator(DASH.editableTable)).toBeVisible();

    // Switch to Audit (the second radio in section-nav).
    const radios = page.locator(DASH.sectionRadio);
    await expect(radios).toHaveCount(2);
    await radios.nth(1).click();

    // Audit section: locked state (audit is not yet unlocked with a master password).
    await expect(page.locator(DASH.auditLocked)).toBeVisible();
    // The Operations table has disappeared.
    await expect(page.locator(DASH.editableTable)).toHaveCount(0);

    // Back — Operations is visible again.
    await radios.nth(0).click();
    await expect(page.locator(DASH.editableTable)).toBeVisible();
  });

  test('the User/Domain/Admin target switcher switches', async ({ app }) => {
    const { page } = app;

    // Target switcher in the ops card: 3 options.
    const targets = page.locator(DASH.targetRadio);
    await expect(targets).toHaveCount(3);

    // Switch to Domain (2nd option). aria-checked updates.
    await targets.nth(1).click();
    await expect(targets.nth(1)).toHaveAttribute('aria-checked', 'true');

    // Admin (3rd option).
    await targets.nth(2).click();
    await expect(targets.nth(2)).toHaveAttribute('aria-checked', 'true');

    // Back to User.
    await targets.nth(0).click();
    await expect(targets.nth(0)).toHaveAttribute('aria-checked', 'true');
  });

  test('logout returns to the login screen', async ({ app }) => {
    const { page } = app;

    await expect(page.locator(DASH.root)).toBeVisible();

    // Click the Logout button (text "Log out" in the default EN locale).
    const logout = page.locator(DASH.logoutButton).filter({ hasText: /Log out|logout/i }).first();
    await logout.click();

    // The login screen is visible again, the dashboard is gone.
    await expect(page.locator(SEL_LOGIN_SCREEN)).toBeVisible();
    await expect(page.locator(DASH.root)).toHaveCount(0);
  });
});
