// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';
import { parseColor, contrastRatio } from '../helpers/layout';

/**
 * Dashboard theme and contrast: dark/light via design tokens on <html>.
 *
 * Assessment of "theme/contrast quality": tokens apply correctly when the theme
 * is switched (data-theme), the accent is present in both themes, the fg/bg
 * contrast meets WCAG AA (>=4.5 for normal text), and cards and text remain
 * visible (no "white on white").
 *
 * Implementation note: several sequential `page.evaluate` calls after clicking
 * theme-toggle can fail with "execution context destroyed" (Dioxus re-renders
 * the document on state.write). Therefore all CSS variables are read in a
 * SINGLE evaluate call after the theme is stabilized via the attribute.
 */

/** Reads a set of CSS variables in a single evaluate call (stable against re-renders). */
async function readVars(page: import('@playwright/test').Page, names: string[]): Promise<Record<string, string>> {
  return page.evaluate((ns) => {
    const cs = getComputedStyle(document.documentElement);
    const out: Record<string, string> = {};
    for (const n of ns) out[n] = cs.getPropertyValue(n).trim();
    return out;
  }, names);
}

/** Waits until data-theme on <html> becomes target (stabilization after a click). */
async function waitForTheme(page: import('@playwright/test').Page, target: 'dark' | 'light'): Promise<void> {
  // The attribute itself is the deterministic condition: the app sets it in the
  // same effect that flips the theme Signal, and the CSS variables are keyed to
  // [data-theme=...] — getComputedStyle (used right after) forces synchronous
  // recalculation, so no fixed sleep is needed.
  await expect(page.locator('html')).toHaveAttribute('data-theme', target);
}

test.describe('Dashboard — theme and contrast', () => {
  test('dark theme by default: dark background, light text, contrast >= 4.5', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const v = await readVars(page, ['--bg', '--fg']);
    const bg = parseColor(v['--bg']!);
    const fg = parseColor(v['--fg']!);
    expect(bg, '--bg is a valid color').not.toBeNull();
    expect(fg, '--fg is a valid color').not.toBeNull();
    expect(contrastRatio(fg!, bg!), 'fg/bg contrast in dark >= 4.5 (WCAG AA)').toBeGreaterThanOrEqual(4.5);
  });

  test('switching to light changes the bg/fg tokens, contrast is preserved', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const dark = await readVars(page, ['--bg', '--fg']);

    await page.locator(DASH.contextThemeToggle).click();
    await waitForTheme(page, 'light');
    const light = await readVars(page, ['--bg', '--fg']);

    expect(light['--bg'], 'bg changed on theme switch').not.toBe(dark['--bg']);
    expect(light['--fg'], 'fg changed on theme switch').not.toBe(dark['--fg']);
    const ratio = contrastRatio(parseColor(light['--fg']!)!, parseColor(light['--bg']!)!);
    expect(ratio, 'fg/bg contrast in light >= 4.5 (WCAG AA)').toBeGreaterThanOrEqual(4.5);
  });

  test('the accent is defined and non-zero in both themes', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const dark = await readVars(page, ['--accent']);
    expect(parseColor(dark['--accent']!), '--accent in dark is a valid color').not.toBeNull();

    await page.locator(DASH.contextThemeToggle).click();
    await waitForTheme(page, 'light');
    const light = await readVars(page, ['--accent']);
    expect(parseColor(light['--accent']!), '--accent in light is a valid color').not.toBeNull();
  });

  test('the accent gradient is defined in both themes', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const dark = await readVars(page, ['--accent-grad']);
    expect(dark['--accent-grad'], 'accent-grad in dark').toMatch(/linear-gradient/i);

    await page.locator(DASH.contextThemeToggle).click();
    await waitForTheme(page, 'light');
    const light = await readVars(page, ['--accent-grad']);
    expect(light['--accent-grad'], 'accent-grad in light').toMatch(/linear-gradient/i);
  });

  test('secondary text contrast against the background is >= 4.5 in both themes', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const dark = await readVars(page, ['--fg-secondary', '--bg']);
    expect(contrastRatio(parseColor(dark['--fg-secondary']!)!, parseColor(dark['--bg']!)!), 'dark: fg-secondary/bg >= 4.5').toBeGreaterThanOrEqual(4.5);

    await page.locator(DASH.contextThemeToggle).click();
    await waitForTheme(page, 'light');
    const light = await readVars(page, ['--fg-secondary', '--bg']);
    expect(contrastRatio(parseColor(light['--fg-secondary']!)!, parseColor(light['--bg']!)!), 'light: fg-secondary/bg >= 4.5').toBeGreaterThanOrEqual(4.5);
  });

  test('cards (surface) differ from the background (bg) in both themes', async ({ app }) => {
    const { page } = app;
    await waitForTheme(page, 'dark');
    const dark = await readVars(page, ['--bg-surface', '--bg', '--fg']);
    const dSurf = parseColor(dark['--bg-surface']!)!;
    const dBg = parseColor(dark['--bg']!)!;
    expect(Math.abs(contrastRatio(dSurf, dBg) - 1), 'dark: surface != bg').toBeGreaterThan(0.01);
    expect(contrastRatio(parseColor(dark['--fg']!)!, dSurf), 'dark: text against surface >= 4.5').toBeGreaterThanOrEqual(4.5);

    await page.locator(DASH.contextThemeToggle).click();
    await waitForTheme(page, 'light');
    const light = await readVars(page, ['--bg-surface', '--bg', '--fg']);
    const lSurf = parseColor(light['--bg-surface']!)!;
    const lBg = parseColor(light['--bg']!)!;
    expect(Math.abs(contrastRatio(lSurf, lBg) - 1), 'light: surface != bg').toBeGreaterThan(0.01);
    expect(contrastRatio(parseColor(light['--fg']!)!, lSurf), 'light: text against surface >= 4.5').toBeGreaterThanOrEqual(4.5);
  });

  // Theme persistence in config.toml is covered ONCE, on the login screen
  // (theme.spec.ts "theme persists across restart") — the dashboard toggle
  // writes the same config key through the same settings path.
});
