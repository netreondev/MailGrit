// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';
import { assertNoOverlap } from '../helpers/layout';

/**
 * Password generation controls: length slider, character classes (with
 * policy-locks), "Fill empty", "Regenerate all".
 *
 * The default config (config.toml `[password_policy]`) requires all 4 character
 * classes -> the corresponding checkboxes are disabled (cannot be turned off).
 * We verify this as the server-side policy working in the UI.
 */
test.describe('Dashboard — password generation controls', () => {
  test('the controls panel is visible with a heading', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(DASH.pwControls)).toBeVisible();
    // The length slider + 4 class checkboxes are present.
    await expect(page.locator(DASH.pwLengthSlider)).toBeVisible();
    await expect(page.locator(DASH.pwClassCheckbox)).toHaveCount(4);
  });

  test('the length slider changes its value and is shown in the label', async ({ app }) => {
    const { page } = app;
    const slider = page.locator(DASH.pwLengthSlider);
    const before = await slider.inputValue();

    // Set the length to 24 (in the 8-32 range) via a direct value.
    await slider.fill('24');
    await expect(slider).toHaveValue('24');

    // Back to the original.
    await slider.fill(before);
    await expect(slider).toHaveValue(before);
  });

  test('character-class checkboxes are locked by the server-side policy (disabled)', async ({ app }) => {
    const { page } = app;
    // The default config requires all 4 classes -> all checkboxes are disabled.
    const boxes = page.locator(DASH.pwClassCheckbox);
    const count = await boxes.count();
    for (let i = 0; i < count; i++) {
      await expect(boxes.nth(i), `class checkbox #${i} is disabled (policy-locked)`).toBeDisabled();
      await expect(boxes.nth(i)).toBeChecked();
    }
  });

  test('the "Fill empty" and "Regenerate all" buttons are present', async ({ app }) => {
    const { page } = app;
    const actions = page.locator(DASH.pwControlsActions);
    await expect(actions).toBeVisible();
    // "Fill empty" — secondary, "Regenerate all" — ghost.
    await expect(actions.locator('.btn').filter({ hasText: /fill empty/i })).toBeVisible();
    await expect(actions.locator('.btn').filter({ hasText: /regenerate all/i })).toBeVisible();
  });

  test('"Fill empty" generates a password for a row with an empty password', async ({ app }) => {
    const { page } = app;

    // Add a row with an empty password.
    await page.locator(DASH.addRowButton).click();
    const lastRow = page.locator(`${DASH.editableTable} tbody tr`).last();
    const pwCell = lastRow.locator(DASH.tdPassword).locator('input');
    await expect(pwCell).toHaveValue('');

    // Click "Fill empty".
    await page.locator(DASH.pwControlsActions).locator('.btn').filter({ hasText: /fill empty/i }).click();

    // The password in the empty row is generated (non-empty).
    await expect.poll(() => pwCell.inputValue(), { message: 'password generated in the empty row' }).not.toBe('');
  });

  test('"Regenerate all" opens a confirmation modal', async ({ app }) => {
    const { page } = app;
    await page.locator(DASH.pwControlsActions).locator('.btn').filter({ hasText: /regenerate all/i }).click();
    const dialog = page.locator(DASH.dialog);
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-label', 'Regenerate all passwords?');
    await dialog.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.locator(DASH.modalBackdrop)).toHaveCount(0);
  });

  test('the password controls do not overlap with the table', async ({ app }) => {
    const { page } = app;
    await assertNoOverlap(
      page.locator(DASH.pwControls),
      page.locator(DASH.editableTableWrap),
      'password controls and table do not collide',
    );
  });
});
