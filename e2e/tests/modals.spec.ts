// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 netreon and contributors

import { testDashboard as test, expect } from '../fixtures/app';
import { DASH } from '../helpers/selectors';
import { assertCenteredBoth, assertInViewport } from '../helpers/layout';

/**
 * Dashboard modal dialogs: delete confirmation, password regeneration, master
 * password (audit unlock).
 *
 * Verifying "usability and clarity": the modal opens on an action, is centered
 * (flex), has correct ARIA (role=dialog, aria-modal, aria-label=title), the
 * confirm/cancel buttons are present, and closing via a background click or
 * CANCEL hides it.
 *
 * Texts are English (the default locale): see app.en.yml.
 */
test.describe('Dashboard — modal dialogs', () => {
  test('Delete: opening, centering, ARIA, closing via Cancel', async ({ app }) => {
    const { page } = app;

    // Trigger: the Delete button (.btn-danger) in the ops card.
    await page.locator(DASH.opsCard).locator('.btn-danger').click();

    // Backdrop + dialog are visible.
    await expect(page.locator(DASH.modalBackdrop)).toBeVisible();
    const dialog = page.locator(DASH.dialog);
    await expect(dialog).toBeVisible();

    // ARIA: role/aria-modal/aria-label.
    await expect(dialog).toHaveAttribute('aria-modal', 'true');
    await expect(dialog).toHaveAttribute('aria-label', 'Confirm deletion');

    // Title and warning icon (danger).
    await expect(page.locator(DASH.modalTitle)).toHaveText('Confirm deletion');
    await expect(dialog.locator(DASH.modalIconDanger)).toBeVisible();

    // Centering (modal-backdrop — flex align/justify center).
    await assertCenteredBoth(dialog, page, 12, 'delete dialog is centered');

    // Buttons in the footer: "Yes, delete" (danger) + "Cancel" (ghost).
    await expect(dialog.getByRole('button', { name: 'Yes, delete' })).toBeVisible();
    const cancel = dialog.getByRole('button', { name: 'Cancel' });
    await expect(cancel).toBeVisible();

    // Closing via Cancel hides the modal.
    await cancel.click();
    await expect(page.locator(DASH.modalBackdrop)).toHaveCount(0);
  });

  test('Delete: closing via a background click (backdrop)', async ({ app }) => {
    const { page } = app;
    await page.locator(DASH.opsCard).locator('.btn-danger').click();
    await expect(page.locator(DASH.dialog)).toBeVisible();

    // Click the background (outside the dialog) — top-left corner of the backdrop.
    await page.locator(DASH.modalBackdrop).click({ position: { x: 8, y: 8 } });
    await expect(page.locator(DASH.modalBackdrop)).toHaveCount(0);
  });

  test('Regenerate all: opening and closing', async ({ app }) => {
    const { page } = app;

    // Password controls are visible (pre-filled rows). The "Regenerate all"
    // button (ghost) is in the password-control actions.
    await expect(page.locator(DASH.pwControls)).toBeVisible();
    const regenBtn = page.locator(DASH.pwControlsActions).locator('.btn').filter({ hasText: /regenerate/i }).first();
    await expect(regenBtn).toBeVisible();
    await regenBtn.click();

    const dialog = page.locator(DASH.dialog);
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-label', 'Regenerate all passwords?');
    await expect(page.locator(DASH.modalTitle)).toHaveText('Regenerate all passwords?');
    await expect(dialog.getByRole('button', { name: 'Yes, regenerate' })).toBeVisible();

    // Closing via Cancel.
    await dialog.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.locator(DASH.modalBackdrop)).toHaveCount(0);
  });

  test('Master password (create): opening from Audit, centering, fields', async ({ app }) => {
    const { page } = app;

    // Go to the Audit section — audit is locked, the Unlock button.
    await page.locator(DASH.sectionRadio).nth(1).click();
    await expect(page.locator(DASH.auditLocked)).toBeVisible();
    await page.locator(DASH.auditLocked).getByRole('button', { name: /unlock/i }).click();

    const dialog = page.locator(DASH.dialog);
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAttribute('aria-label', 'Master password');

    // Create mode (no audit key) -> two password fields.
    const pwInputs = dialog.locator('input[type="password"]');
    await expect(pwInputs).toHaveCount(2);

    // Centering.
    await assertCenteredBoth(dialog, page, 12, 'master-password dialog is centered');

    // Buttons: "Create and unlock" + "Cancel".
    await expect(dialog.getByRole('button', { name: /create and unlock/i })).toBeVisible();
    const cancel = dialog.getByRole('button', { name: 'Cancel' });
    await cancel.click();
    await expect(page.locator(DASH.modalBackdrop)).toHaveCount(0);
  });

  test('the modal is inside the viewport (not clipped)', async ({ app }) => {
    const { page } = app;
    await page.locator(DASH.opsCard).locator('.btn-danger').click();
    await expect(page.locator(DASH.dialog)).toBeVisible();
    await assertInViewport(page, DASH.dialog, 'delete dialog is within the viewport');
    await page.locator(DASH.dialog).getByRole('button', { name: 'Cancel' }).click();
  });
});
