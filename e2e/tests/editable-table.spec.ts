import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';
import { assertInViewport, assertNoOverlap } from '../helpers/layout';

/**
 * Editable row table: adding/removing/editing cells, format validation
 * (per-cell highlighting), per-row password generation.
 *
 * The pre-filled test rows (env hook) provide 2 rows — enough to exercise the
 * interactions without the `rfd` CSV dialog (which is not accessible to
 * Playwright).
 */
test.describe('Dashboard — editable table', () => {
  test('the table is visible with 2 pre-filled rows and column headers', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(DASH.editableTable)).toBeVisible();

    // Heading with the row count.
    await expect(page.locator(DASH.editableTableWrap).locator('h3')).toContainText('2');

    // 6 column headers.
    await expect(page.locator(`${DASH.editableTable} thead th`)).toHaveCount(6);

    // 2 data rows.
    await expect(page.locator(`${DASH.editableTable} tbody tr`)).toHaveCount(2);

    // Pre-filled domain values (example.com) in the first column.
    const domains = page.locator(`${DASH.editableTable} tbody tr`).locator('td').nth(0).locator('input');
    await expect(domains.nth(0)).toHaveValue('example.com');
  });

  test('"Add row" increases the row count', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(`${DASH.editableTable} tbody tr`)).toHaveCount(2);

    const addBtn = page.locator(DASH.addRowButton);
    await expect(addBtn).toBeVisible();
    await addBtn.click();

    // Now 3 rows, the heading updated.
    await expect(page.locator(`${DASH.editableTable} tbody tr`)).toHaveCount(3);
    await expect(page.locator(DASH.editableTableWrap).locator('h3')).toContainText('3');
  });

  test('editing a cell changes the value', async ({ app }) => {
    const { page } = app;
    const firstDomain = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator('td').nth(0).locator('input');
    await expect(firstDomain).toHaveValue('example.com');

    // Clear and enter a new value.
    await firstDomain.fill('acme.test');
    await expect(firstDomain).toHaveValue('acme.test');
  });

  test('an invalid domain (email) highlights the cell and the row', async ({ app }) => {
    const { page } = app;
    const firstDomain = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator('td').nth(0).locator('input');

    // Enter a domain with @ (EmailProvided — invalid).
    await firstDomain.fill('user@example.com');

    // The cell gets the input-cell-invalid class.
    await expect(firstDomain).toHaveClass(/input-cell-invalid/);
    // The row — row-invalid with a title tooltip.
    const row = page.locator(`${DASH.editableTable} tbody tr`).nth(0);
    await expect(row).toHaveClass(/row-invalid/);
    const title = await row.getAttribute('title');
    expect(title, 'the row contains the error text in title').toBeTruthy();
    expect(title!.length).toBeGreaterThan(0);
  });

  test('a valid domain removes the highlighting', async ({ app }) => {
    const { page } = app;
    const firstDomain = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator('td').nth(0).locator('input');

    // First invalid, then valid.
    await firstDomain.fill('user@example.com');
    await expect(firstDomain).toHaveClass(/input-cell-invalid/);
    await firstDomain.fill('acme.test');
    await expect(firstDomain).not.toHaveClass(/input-cell-invalid/);
  });

  test('deleting a row decreases the count', async ({ app }) => {
    const { page } = app;
    await expect(page.locator(`${DASH.editableTable} tbody tr`)).toHaveCount(2);

    // The delete button in the first row (.td-actions .btn-icon).
    await page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.deleteRowButton).click();

    await expect(page.locator(`${DASH.editableTable} tbody tr`)).toHaveCount(1);
    await expect(page.locator(DASH.editableTableWrap).locator('h3')).toContainText('1');
  });

  test('per-row password generation fills the cell', async ({ app }) => {
    const { page } = app;
    // The password in the first row is initially non-empty (from the test CSV).
    const pwCell = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.tdPassword).locator('input');
    const before = await pwCell.inputValue();

    // Click the password generation button in the row.
    await page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.genPwButton).click();
    await expect.poll(() => pwCell.inputValue(), { message: 'password generated (non-empty)' }).not.toBe('');

    // The password changed (a new random value).
    const after = await pwCell.inputValue();
    expect(after, 'the generated password differs from the original').not.toBe(before);
  });

  test('row action buttons do not overlap and are within the viewport', async ({ app }) => {
    const { page } = app;
    const genBtn = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.genPwButton);
    const delBtn = page.locator(`${DASH.editableTable} tbody tr`).nth(0).locator(DASH.deleteRowButton);
    await assertNoOverlap(genBtn, delBtn, 'gen-pw and delete-row buttons do not overlap');
  });
});
