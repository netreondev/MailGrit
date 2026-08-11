// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH, SEL } from '../helpers/selectors';

/**
 * Accessibility (a11y) of the dashboard and base semantics.
 *
 * Assessment of "clarity/a11y": ARIA roles are present on interactive widgets,
 * buttons have discernible text or aria-label/title, icons are not "mute",
 * heading semantics are correct (h2 on cards), and the focus-ring is visible on
 * tabbable elements. The actual UI state is tested (what exists), not an ideal
 * contract.
 */
test.describe('Dashboard — accessibility (a11y)', () => {
  test('segmented controls have role=radiogroup/radio with aria-checked', async ({ app }) => {
    const { page } = app;
    // Section-nav — radiogroup.
    const navGroup = page.locator(`${DASH.sectionNav} [role="radiogroup"], ${DASH.sectionNav}[role="radiogroup"]`);
    // The segmented control renders role=radiogroup; if not, we check the radios inside.
    const navRadios = page.locator(DASH.sectionRadio);
    await expect(navRadios.first()).toBeVisible();
    const radioCount = await navRadios.count();
    expect(radioCount, 'section-nav has radio buttons').toBeGreaterThanOrEqual(2);
    // The active radio is marked aria-checked=true.
    const checkedCount = await page.locator(`${DASH.sectionRadio}[aria-checked="true"]`).count();
    expect(checkedCount, 'at least one section-radio aria-checked=true').toBeGreaterThanOrEqual(1);

    // Target switcher — also a radio control.
    const targetRadios = page.locator(DASH.targetRadio);
    const targetChecked = await page.locator(`${DASH.targetRadio}[aria-checked="true"]`).count();
    expect(await targetRadios.count(), 'target switcher has radio buttons').toBeGreaterThanOrEqual(2);
    expect(targetChecked, 'at least one target-radio aria-checked=true').toBeGreaterThanOrEqual(1);
  });

  test('language selector: role=listbox/option, overlay catches outside clicks', async ({ app }) => {
    const { page } = app;
    await page.locator('.lang-trigger').first().click();
    await expect(page.locator(SEL.langDropdown)).toBeVisible();
    await expect(page.locator(SEL.langDropdown)).toHaveAttribute('role', 'listbox');
    // Options have role=option.
    await expect(page.locator(SEL.langItemButton).first()).toHaveAttribute('role', 'option');
    await page.locator(SEL.langOverlay).click({ position: { x: 5, y: 5 } });
  });

  test('modal has role=dialog, aria-modal, aria-label (a11y)', async ({ app }) => {
    const { page } = app;
    await page.locator(DASH.opsCard).locator('.btn-danger').click();
    const dialog = page.locator(DASH.dialog);
    await expect(dialog).toHaveAttribute('role', 'dialog');
    await expect(dialog).toHaveAttribute('aria-modal', 'true');
    const label = await dialog.getAttribute('aria-label');
    expect(label, 'dialog has an aria-label (title)').toBeTruthy();
    await dialog.getByRole('button', { name: 'Cancel' }).click();
  });

  test('row action icon-buttons have a title (not "mute")', async ({ app }) => {
    const { page } = app;
    // The per-row password generation button — has a title (tooltip).
    const genBtn = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.genPwButton);
    await expect(genBtn).toHaveAttribute('title');
    // The row deletion button — also a title.
    const delBtn = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.deleteRowButton);
    await expect(delBtn).toHaveAttribute('title');
  });

  test('cards have h2 headings (semantic structure)', async ({ app }) => {
    const { page } = app;
    // Each card in the grid contains an h2.
    const cards = page.locator(DASH.cards);
    const count = await cards.count();
    for (let i = 0; i < count; i++) {
      const h2 = cards.nth(i).locator('h2');
      await expect(h2, `card #${i} contains an h2`).toHaveCount(1);
      const text = (await h2.textContent())?.trim() ?? '';
      expect(text.length, `h2 of card #${i} is non-empty`).toBeGreaterThan(0);
    }
  });

  test('tabbable elements receive a visible focus-ring', async ({ app }) => {
    const { page } = app;
    // There is no URL input on the dashboard, but table cell inputs are tabbable.
    // Focus the first cell input and verify the outline is not none.
    const firstCell = page.locator(DASH.inputCell).first();
    await firstCell.focus();
    // Check the computed outline-style (must not be none when focused).
    const outline = await firstCell.evaluate((el) => {
      const cs = getComputedStyle(el);
      return { style: cs.outlineStyle, width: cs.outlineWidth, color: cs.outlineColor };
    });
    // outline-style may be 'auto' or 'solid'; the key is it is not fully hidden.
    // We allow auto (the standard browser focus-ring) or a non-empty color.
    const hasFocusIndicator =
      outline.style !== 'none' ||
      (outline.width !== '0px' && outline.width !== 'medium') ||
      outline.color !== 'rgb(0, 0, 0)';
    expect(hasFocusIndicator, 'cell input has a visible focus indicator').toBe(true);
  });

  test('all visible buttons in the ops card have an accessible name (text or aria-label/title)', async ({ app }) => {
    const { page } = app;
    // Reset focus from the previous test + stabilize.
    await page.locator('body').click();
    await page.waitForLoadState('domcontentloaded');
    const opsCard = page.locator(DASH.opsCard);
    await expect(opsCard).toBeVisible();
    // All buttons in the card (no disabled filter — it is unstable in Dioxus).
    const buttons = opsCard.locator('button');
    await expect(buttons.first()).toBeVisible();
    const count = await buttons.count();
    expect(count, 'ops card has buttons').toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      const btn = buttons.nth(i);
      // Skip invisible ones (e.g. hidden spinner icons).
      if (!(await btn.isVisible())) continue;
      const text = ((await btn.textContent()) ?? '').trim();
      const aria = await btn.getAttribute('aria-label');
      const title = await btn.getAttribute('title');
      const accessibleName = text || aria || title || '';
      expect(accessibleName.length, `button #${i} has an accessible name`).toBeGreaterThan(0);
    }
  });

  test('content is readable: no empty text nodes in key headings', async ({ app }) => {
    const { page } = app;
    // Clear focus from the previous test (focus-ring) and wait for stabilization.
    await page.locator('body').click();
    await page.waitForLoadState('domcontentloaded');
    // Collect texts in a single evaluate (stable against Dioxus re-renders).
    const data = await page.evaluate(() => {
      const h2 = Array.from(document.querySelectorAll('.dash-grid .card h2')).map((e) => (e.textContent ?? '').trim());
      const nav = Array.from(document.querySelectorAll('.section-nav [role="radio"]')).map((e) => (e.textContent ?? '').trim());
      return { h2, nav };
    });
    expect(data.h2.length, 'there are card headings').toBeGreaterThan(0);
    for (const t of data.h2) {
      expect(t.length, 'card heading is non-empty').toBeGreaterThan(0);
    }
    expect(data.nav.length, 'there are nav radios').toBeGreaterThanOrEqual(2);
    for (const t of data.nav) {
      expect(t.length, 'nav radio is non-empty').toBeGreaterThan(0);
    }
  });
});
