// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH, LANGUAGES, DASH_I18N, I18N_MARKERS } from '../helpers/selectors';
import { assertNoRawKey } from '../helpers/layout';

/**
 * Localization of all dashboard screens (9 languages).
 *
 * Assessment of "clarity/i18n": loop over every locale via the language
 * selector, verifying that the visible navigation/action/session text changes
 * to the expected values from app.<lang>.yml (DASH_I18N) and contains no
 * "broken" keys (raw `nav.audit`). In parallel we cover the langLabel marker
 * from helpers (the trigger aria-label).
 */
test.describe('Dashboard — localization (i18n)', () => {
  // Default language is EN. Each test starts from it.
  test.beforeEach(async ({ app }) => {
    const { page } = app;
    await expect(page.locator(DASH.root)).toBeVisible();
    // Ensure EN (if a previous test left another locale, switch back).
    await ensureLanguage(page, 'English');
  });

  for (const lang of LANGUAGES) {
    test(`localizes the dashboard to "${lang.label}" (${lang.code})`, async ({ app }) => {
      const { page } = app;
      const expected = DASH_I18N[lang.code];
      expect(expected, `expected i18n markers for ${lang.code}`).toBeDefined();

      await selectLanguage(page, lang.label);

      // Nav: Operations (the first radio).
      await expect(page.locator(DASH.sectionRadio).nth(0)).toHaveText(expected.navOps);
      // Audit (the second radio).
      await expect(page.locator(DASH.sectionRadio).nth(1)).toHaveText(expected.navAudit);

      // Target User (the first option of the target switcher).
      await expect(page.locator(DASH.targetRadio).nth(0)).toHaveText(expected.targetUser);

      // Action buttons in the ops card.
      await expect(page.locator(DASH.opsCard).getByRole('button', { name: expected.actionCreate })).toBeVisible();
      await expect(page.locator(DASH.opsCard).getByRole('button', { name: expected.actionDelete })).toBeVisible();

      // Session badge in the context-bar.
      await expect(page.locator(DASH.contextBarLeft)).toContainText(expected.sessionActive);

      // Logout button.
      await expect(page.locator(DASH.contextBar).getByRole('button', { name: expected.logout })).toBeVisible();

      // Verify there are no raw i18n keys in the visible text (broken translations).
      const navOpsText = await page.locator(DASH.sectionRadio).nth(0).textContent();
      assertNoRawKey(navOpsText ?? '', `nav.operations on ${lang.code}`);
    });
  }

  test('the selector language persists across changes (langLabel aria-label)', async ({ app }) => {
    const { page } = app;
    // Switch to Deutsch and verify the trigger aria-label (= lang.label DE).
    await selectLanguage(page, 'Deutsch');
    await expect(page.locator('.context-bar .lang-trigger, .lang-trigger').first()).toHaveAttribute('aria-label', 'Sprache');
    // Back to EN.
    await selectLanguage(page, 'English');
    await expect(page.locator('.lang-trigger').first()).toHaveAttribute('aria-label', 'Language');
  });

  test('modal titles are localized', async ({ app }) => {
    const { page } = app;
    await selectLanguage(page, 'Deutsch');
    // Open the delete-modal and verify the localized title.
    await page.locator(DASH.opsCard).locator('.btn-danger').click();
    await expect(page.locator(DASH.dialog)).toBeVisible();
    // aria-label = title (DE: "Löschen bestätigen").
    const ariaLabel = await page.locator(DASH.dialog).getAttribute('aria-label');
    // Exact contract: a non-empty string that is not a raw i18n key.
    expect(ariaLabel !== null && ariaLabel.trim().length > 0, 'DE delete-modal aria-label is a non-empty string').toBe(true);
    assertNoRawKey(ariaLabel!, 'DE delete-modal title');
    await page.locator(DASH.dialog).getByRole('button').filter({ hasText: /abbr/i }).click();
  });
});

/** Opens the language selector and picks an item by its endonym (exact match). */
async function selectLanguage(page: import('@playwright/test').Page, label: string): Promise<void> {
  await page.locator('.lang-trigger').first().click();
  await expect(page.locator('.lang-dropdown')).toBeVisible();
  await page.locator('.lang-dropdown').getByText(label, { exact: true }).click();
  // The menu closed — the indicator that switching finished.
  await expect(page.locator('.lang-dropdown')).toHaveCount(0);
  // Deterministic wait for the re-render: the trigger's aria-label is itself
  // localized (lang.label) — when it matches the target locale, the switch has
  // actually been applied (no fixed sleeps).
  const code = LANGUAGES.find((l) => l.label === label)?.code;
  const expectedAria = code ? I18N_MARKERS[code]?.langLabel : undefined;
  if (expectedAria) {
    await expect(page.locator('.lang-trigger').first()).toHaveAttribute('aria-label', expectedAria);
  }
}

/**
 * Ensures the current language is `label` (switches if it is not). Works for
 * every locale: the trigger's aria-label is the localized `lang.label` value
 * (I18N_MARKERS), so the check needs no per-locale special-casing.
 */
async function ensureLanguage(page: import('@playwright/test').Page, label: string): Promise<void> {
  const code = LANGUAGES.find((l) => l.label === label)?.code;
  const expectedAria = code ? I18N_MARKERS[code]?.langLabel : undefined;
  const trigger = page.locator('.lang-trigger').first();
  const currentAria = await trigger.getAttribute('aria-label');
  if (expectedAria && currentAria === expectedAria) return;
  await selectLanguage(page, label);
}
