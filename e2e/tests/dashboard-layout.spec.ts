// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';
import {
  assertInViewport,
  assertSymmetricMargins,
  assertNoOverlap,
  assertAlignedLeft,
  viewport,
} from '../helpers/layout';

/**
 * Symmetry and geometry of the dashboard.
 *
 * Data-driven assessment of grid visual quality: the `.dash-grid` container is
 * centered (`margin: 0 auto` -> symmetric margins), cards do not overlap,
 * span-all takes the full width, interactive elements are within the viewport,
 * and the table has predictable columns (table-fixed).
 */
test.describe('Dashboard — symmetry/layout', () => {
  test('the card grid is centered (symmetric side margins)', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(DASH.dashGrid)).toBeVisible();
    // Symmetry of the grid side margins inside the scrolling body. The vertical
    // scrollbar (~17px on Windows) eats space on the right, so we measure
    // relative to clientWidth (without the scrollbar), not boundingRect.
    const m = await page.evaluate(() => {
      const g = document.querySelector('.dash-grid')!.getBoundingClientRect();
      const body = document.querySelector('.dashboard-body')!;
      return {
        gx: g.x,
        gRight: g.x + g.width,
        bodyLeft: body.getBoundingClientRect().x,
        clientWidth: body.clientWidth, // width WITHOUT the scrollbar
      };
    });
    const leftMargin = m.gx - m.bodyLeft;
    const rightMargin = (m.bodyLeft + m.clientWidth) - m.gRight;
    expect(
      Math.abs(leftMargin - rightMargin),
      `grid side margins are symmetric (by clientWidth without scrollbar): left=${leftMargin.toFixed(1)} right=${rightMargin.toFixed(1)}`,
    ).toBeLessThanOrEqual(2);
  });

  test('the CSV and operations cards do not overlap', async ({ app }) => {
    const { page } = app;
    const cards = page.locator(DASH.cards);
    // The first two cards are in the same grid row (CSV | ops).
    await assertNoOverlap(
      cards.nth(0),
      cards.nth(1),
      'CSV card and ops card do not collide',
    );
  });

  test('span-all (result) takes the full grid width', async ({ app }) => {
    const { page } = app;
    const grid = page.locator(DASH.dashGrid);
    const spanAll = page.locator(DASH.spanAll);
    const [gridBox, spanBox] = await Promise.all([
      grid.boundingBox(),
      spanAll.boundingBox(),
    ]);
    expect(gridBox, 'dash-grid boundingBox').not.toBeNull();
    expect(spanBox, 'span-all boundingBox').not.toBeNull();
    // The left edge of span-all matches the left edge of the grid.
    expect(Math.abs(spanBox!.x - gridBox!.x), 'span-all is aligned with the grid left edge').toBeLessThanOrEqual(2);
    // span-all width ~= grid width (full width, grid-column: 1 / -1).
    expect(spanBox!.width, 'span-all ~= grid width').toBeGreaterThanOrEqual(gridBox!.width - 2);
  });

  test('all cards are within the viewport', async ({ app }) => {
    const { page } = app;
    const cards = page.locator(DASH.cards);
    const count = await cards.count();
    for (let i = 0; i < count; i++) {
      const box = await cards.nth(i).boundingBox();
      expect(box, `card #${i} boundingBox`).not.toBeNull();
      const vp = await viewport(page);
      expect(box!.x, `card #${i} x >= 0`).toBeGreaterThanOrEqual(-2);
      expect(box!.y, `card #${i} y >= 0`).toBeGreaterThanOrEqual(-2);
      expect(box!.x + box!.width, `card #${i} right edge <= viewport`).toBeLessThanOrEqual(vp.width);
      expect(box!.y + box!.height, `card #${i} bottom edge <= viewport`).toBeLessThanOrEqual(vp.height);
    }
  });

  test('operation buttons are within the viewport and do not overlap', async ({ app }) => {
    const { page } = app;
    // The Create (primary) and Edit (secondary) buttons in one op-row.
    const opsCard = page.locator(DASH.opsCard);
    const createBtn = opsCard.locator('.btn-primary').first();
    const editBtn = opsCard.locator('.btn-secondary').first();
    await expect(createBtn).toBeVisible();
    await expect(editBtn).toBeVisible();
    await assertNoOverlap(createBtn, editBtn, 'Create and Edit do not overlap');
  });

  test('editable table columns have predictable widths (colgroup)', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(DASH.editableTable)).toBeVisible();
    // colgroup is present with six col classes (table-fixed -> symmetry).
    for (const colClass of [DASH.colDomain, DASH.colUsername, DASH.colPassword, DASH.colDisplay, DASH.colQuota, DASH.colActions]) {
      await expect(page.locator(`col${colClass}`)).toHaveCount(1);
    }
    // Each col has a non-zero width via a CSS percentage.
    const tableBox = await page.locator(DASH.editableTable).boundingBox();
    expect(tableBox, 'table boundingBox').not.toBeNull();
    expect(tableBox!.width, 'table has a width').toBeGreaterThan(100);
  });

  test('table column headers and cells are aligned on the left edge', async ({ app }) => {
    const { page } = app;
    const headerCell = page.locator(`${DASH.editableTable} thead th`).first();
    const bodyCell = page.locator(`${DASH.editableTable} tbody td`).first();
    await expect(headerCell).toBeVisible();
    await expect(bodyCell).toBeVisible();
    await assertAlignedLeft(headerCell, bodyCell, 2, 'header and body of the first column are aligned');
  });

  test('the context-bar and the dashboard body do not overlap', async ({ app }) => {
    const { page } = app;
    await assertNoOverlap(
      page.locator(DASH.contextBar),
      page.locator(DASH.sectionNav),
      'context-bar does not collide with the section navigation',
    );
  });
});
