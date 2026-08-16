// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';
import { viewport, viewportWithScreen } from '../helpers/layout';

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

  test('maximize toggles the real window state (metrics, not liveness)', async ({ app }) => {
    const { page } = app;

    const maxBtn = page.locator(`${SEL.winBtn}[aria-label="Maximize"]`);
    await expect(maxBtn).toBeEnabled();

    // Real window metrics (innerWidth/innerHeight track the OS window; the
    // old version of this test asserted only that the UI stayed alive — a
    // no-op maximize handler passed it). The resize is asynchronous (winit →
    // OS → WebView2 → JS metrics), so the reads are POLLED, never one-shot.
    const before = await viewport(page);

    await maxBtn.click();
    // On a normal desktop maximize grows the window to fill the screen; on a
    // CI runner that already clamps the window to the full desktop nothing
    // can grow, and the fill condition simply holds from the start.
    await expect
      .poll(
        async () => {
          const v = await viewportWithScreen(page);
          return v.width >= v.availWidth - 16 && v.height >= v.availHeight - 64;
        },
        { message: 'maximize must fill the available desktop', timeout: 15_000 },
      )
      .toBe(true);
    const maximized = await viewport(page);

    // A second click — restore: the window returns to its restored size.
    await maxBtn.click();
    await expect
      .poll(
        async () => {
          const v = await viewport(page);
          return v.width <= before.width && v.height <= before.height;
        },
        {
          message:
            `restore must return to the restored size ` +
            `(before ${before.width}x${before.height}, maximized ${maximized.width}x${maximized.height})`,
          timeout: 15_000,
        },
      )
      .toBe(true);
  });
});
