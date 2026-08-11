// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { test, expect } from '../fixtures/app';
import { SEL } from '../helpers/selectors';

test.describe('App launch', () => {
  test('window opens with titlebar brand and login screen', async ({ app }) => {
    const { page } = app;

    // Titlebar with the "MailGrit" brand.
    await expect(page.locator(SEL.titlebar)).toBeVisible();
    await expect(page.locator(SEL.titlebarName)).toHaveText('MailGrit');

    // The SVG logo is present in the titlebar.
    await expect(page.locator(SEL.titlebarLogo).first()).toBeVisible();

    // The login screen is rendered.
    await expect(page.locator(SEL.loginScreen)).toBeVisible();

    // The URL field and the form-open button — the key elements of the login screen.
    await expect(page.locator(SEL.serverInput)).toBeVisible();
    await expect(page.locator(SEL.openFormButton)).toBeVisible();
  });

  test('window title is the brand name', async ({ app }) => {
    const { page } = app;
    // main.rs: with_title(brand::APP_NAME) -> the OS window title.
    await expect(page).toHaveTitle('MailGrit');
  });
});
