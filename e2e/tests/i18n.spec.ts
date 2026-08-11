// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { test, expect } from '../fixtures/app';
import { SEL, LANGUAGES, I18N_MARKERS } from '../helpers/selectors';

/**
 * Full loop over all 9 languages: switching changes the localized UI text.
 * The marker is the `login.open_form` button text on the login screen (always
 * visible, independent of session/dashboard). We compare against exact values
 * from the catalogs.
 */
test.describe('Internationalization — all 9 locales', () => {
  test('switching locale changes the on-screen UI text', async ({ app }) => {
    const { page } = app;
    const btn = page.locator(SEL.openFormButton).first();

    for (const lang of LANGUAGES) {
      await page.locator(SEL.langTrigger).click();
      await expect(page.locator(SEL.langDropdown)).toBeVisible();
      await page.locator(SEL.langDropdown).getByText(lang.label, { exact: true }).click();

      // The menu closed.
      await expect(page.locator(SEL.langDropdown)).toHaveCount(0);

      // The button text changed to the localized value.
      const expected = I18N_MARKERS[lang.code as keyof typeof I18N_MARKERS].openForm;
      await expect(btn, `UI text for locale ${lang.code}`).toHaveText(expected, { timeout: 10_000 });

      // The current language is marked with a check in the menu (lang-item-active).
      await page.locator(SEL.langTrigger).click();
      const active = page.locator(SEL.langItemActive);
      await expect(active).toHaveCount(1);
      await expect(active).toContainText(lang.label);
      // Close the menu by clicking outside, so it does not affect the next iteration.
      await page.locator(SEL.langOverlay).click({ position: { x: 5, y: 5 } });
      await expect(page.locator(SEL.langDropdown)).toHaveCount(0);
    }
  });
});
