// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { test, expect } from '../fixtures/app';
import { SEL, LANGUAGES } from '../helpers/selectors';

/**
 * REGRESSION: the language selector did not drop down — the dropdown rendered
 * outside the viewport because of a CSS containing-block bug (a fixed overlay
 * covering the whole screen became the ancestor of the absolute dropdown).
 * The fix is that the overlay and dropdown are now siblings. This test is a
 * guard against the bug returning.
 */
test.describe('Language selector', () => {
  test('trigger opens a visible dropdown with all 9 languages', async ({ app }) => {
    const { page } = app;

    // Initially the menu is closed: neither overlay nor dropdown in the DOM.
    await expect(page.locator(SEL.langOverlay)).toHaveCount(0);
    await expect(page.locator(SEL.langDropdown)).toHaveCount(0);

    // Click the trigger.
    await page.locator(SEL.langTrigger).click();

    // The overlay appears (fixed over the whole screen — catches outside clicks).
    await expect(page.locator(SEL.langOverlay)).toBeVisible();

    // KEY CHECK: the dropdown is VISIBLE (not pushed off the viewport).
    const dropdown = page.locator(SEL.langDropdown);
    await expect(dropdown).toBeVisible();
    const box = await dropdown.boundingBox();
    expect(box, 'dropdown must have a boundingBox').not.toBeNull();
    const vp = page.viewportSize() ?? { width: 1120, height: 780 };
    expect(box!.x, 'dropdown within the screen on X').toBeGreaterThanOrEqual(0);
    expect(box!.y, 'dropdown within the screen on Y (not pushed below)').toBeLessThan(vp.height - 20);
    expect(box!.x + box!.width, 'right edge within the screen').toBeLessThanOrEqual(vp.width);

    // Exactly 9 languages.
    await expect(page.locator(SEL.langItemButton)).toHaveCount(LANGUAGES.length);

    // Each endonym is present.
    for (const lang of LANGUAGES) {
      await expect(page.locator(SEL.langDropdown).getByText(lang.label, { exact: true })).toBeVisible();
    }
  });

  test('selecting a language closes the menu and changes the UI locale', async ({ app }) => {
    const { page } = app;

    // Open and select Deutsch.
    await page.locator(SEL.langTrigger).click();
    await expect(page.locator(SEL.langDropdown)).toBeVisible();
    await page.locator(SEL.langDropdown).getByText('Deutsch', { exact: true }).click();

    // The menu closed.
    await expect(page.locator(SEL.langDropdown)).toHaveCount(0);
    await expect(page.locator(SEL.langOverlay)).toHaveCount(0);

    // The trigger now shows the DE code (uppercase).
    await expect(page.locator(SEL.langTrigger)).toContainText('DE');

    // Back to English — verify the reverse path.
    await page.locator(SEL.langTrigger).click();
    await page.locator(SEL.langDropdown).getByText('English', { exact: true }).click();
    await expect(page.locator(SEL.langTrigger)).toContainText('EN');
  });

  test('clicking outside (overlay) closes the menu without selecting', async ({ app }) => {
    const { page } = app;

    await page.locator(SEL.langTrigger).click();
    await expect(page.locator(SEL.langDropdown)).toBeVisible();

    // Click the overlay (outside the menu) — top-left corner, deliberately missing the dropdown.
    await page.locator(SEL.langOverlay).click({ position: { x: 5, y: 5 } });

    await expect(page.locator(SEL.langDropdown)).toHaveCount(0);
    // The language did not change (still EN by default).
    await expect(page.locator(SEL.langTrigger)).toContainText('EN');
  });
});
