// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';

/**
 * Theme toggle: dark <-> light. Applied via `data-theme` on `<html>`
 * (theme.rs::apply_theme -> evaluate_script). Default is dark.
 */
test.describe('Theme toggle', () => {
  test('defaults to dark theme', async ({ app }) => {
    const { page } = app;
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  });

  test('toggles to light and back to dark', async ({ app }) => {
    const { page } = app;
    const html = page.locator('html');

    await expect(html).toHaveAttribute('data-theme', 'dark');

    await page.locator(SEL.themeToggle).click();
    await expect(html).toHaveAttribute('data-theme', 'light');

    await page.locator(SEL.themeToggle).click();
    await expect(html).toHaveAttribute('data-theme', 'dark');
  });

  test('theme persists across restart', async ({ app }) => {
    const { page, dataDir } = app;

    // Switch to light.
    await page.locator(SEL.themeToggle).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

    // config.toml is written next to the binary (mailgrit-data/).
    // Verify persistence: the LIGHT theme value must be IN THE FILE. The save
    // is asynchronous (spawn_blocking in save_theme) and config.toml already
    // exists from startup with the OLD theme — polling mere existence races
    // the write; poll the CONTENT instead.
    const { readFileSync } = await import('node:fs');
    const { join } = await import('node:path');
    const cfg = join(dataDir, 'mailgrit-data', 'config.toml');
    await expect.poll(
      () => {
        try {
          return readFileSync(cfg, 'utf8');
        } catch {
          return '';
        }
      },
      { timeout: 5000, message: 'config.toml stores the light theme' },
    ).toMatch(/theme\s*=\s*"light"/i);
  });
});
