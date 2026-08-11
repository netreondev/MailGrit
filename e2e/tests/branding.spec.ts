// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';

/**
 * "MailGrit" branding: the SVG logo (the "Forged M" mark — an M monogram of
 * forged metal) and the brand-name text are present in the titlebar and on the
 * login hero screen.
 */
test.describe('Brand mark', () => {
  test('renders the SVG logo in titlebar and login hero', async ({ app }) => {
    const { page } = app;

    // Titlebar logo.
    const tbLogo = page.locator(`${SEL.titlebar} svg.logo`).first();
    await expect(tbLogo).toBeVisible();
    // The mark viewBox is 24x24.
    await expect(tbLogo).toHaveAttribute('viewBox', '0 0 24 24');

    // Hero logo on the login screen.
    const heroLogo = page.locator(`${SEL.loginScreen} svg.logo`).first();
    await expect(heroLogo).toBeVisible();
    await expect(heroLogo).toHaveAttribute('viewBox', '0 0 24 24');
  });

  test('logo carries the brand gradient and the forged-M glyph', async ({ app }) => {
    const { page } = app;

    // Inner SVG content of the logo: gradient defs + the M monogram.
    // The logo is inlined via dangerous_inner_html, so we check the markup.
    const hero = page.locator(`${SEL.loginScreen} svg.logo`).first();
    const markup = await hero.innerHTML();

    // The gradient is present (the design system's anchor stops).
    expect(markup, 'logo must contain the gradient #1D4ED8..#38BDF8').toMatch(/1D4ED8/i);
    expect(markup).toMatch(/38BDF8/i);

    // The M (Mail) monogram — a white path stroke.
    expect(markup, 'logo must contain the white monogram stroke').toMatch(/#fff|#ffffff/i);

    // The white-hot glow underlayer (forged metal at heat) — cyan #7DD3FC.
    expect(markup, 'logo must contain the white-hot glow #7DD3FC').toMatch(/7DD3FC/i);
  });

  test('brand name text is the single word "MailGrit"', async ({ app }) => {
    const { page } = app;

    // On the login screen — an h1 with the name.
    const h1 = page.locator(`${SEL.loginScreen} h1`);
    await expect(h1).toHaveText('MailGrit');

    // In the titlebar — a span with the name.
    await expect(page.locator(SEL.titlebarName)).toHaveText('MailGrit');
  });
});
