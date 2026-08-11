// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';

/**
 * Window control buttons in the titlebar (minimize/maximize). Close is NOT
 * tested — it terminates the application (with_exits_when_last_window_closes).
 */
test.describe('Window controls', () => {
  test('titlebar has minimize and maximize buttons', async ({ app }) => {
    const { page } = app;
    const btns = page.locator(SEL.winBtn);
    // Three buttons: minimize, maximize, close.
    await expect(btns).toHaveCount(3);

    // aria-label from the catalog (titlebar.minimize/maximize/close).
    await expect(page.locator(`${SEL.winBtn}[aria-label="Minimize"]`)).toBeVisible();
    await expect(page.locator(`${SEL.winBtn}[aria-label="Maximize"]`)).toBeVisible();
    await expect(page.locator(SEL.winBtnClose)).toBeVisible();
  });

  test('maximize toggles window state (button stays interactive)', async ({ app }) => {
    const { page } = app;

    const maxBtn = page.locator(`${SEL.winBtn}[aria-label="Maximize"]`);
    await expect(maxBtn).toBeEnabled();

    // The click must not crash the application — the UI stays alive after maximize.
    await maxBtn.click();
    await expect(page.locator(SEL.titlebar)).toBeVisible();

    // A second click — restore. The window returns. The UI is still responsive.
    await maxBtn.click();
    await expect(page.locator(SEL.titlebarName)).toHaveText('MailGrit');
  });
});
