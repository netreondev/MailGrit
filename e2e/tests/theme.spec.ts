// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

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
    // Verify persistence: the theme value must be saved in the config.
    // dataDir is the temporary directory the .exe was copied into (mailgrit-data/ sits next to it).
    const { existsSync, readFileSync } = await import('node:fs');
    const { join } = await import('node:path');
    const cfg = join(dataDir, 'mailgrit-data', 'config.toml');
    await expect.poll(async () => existsSync(cfg), { timeout: 5000 }).toBe(true);
    const content = readFileSync(cfg, 'utf8');
    expect(content, 'config.toml stores the theme').toContain('theme');
    expect(content).toMatch(/light/i);
  });
});
