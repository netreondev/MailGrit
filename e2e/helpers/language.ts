// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/**
 * Language-selection helpers — the single implementation of "open the
 * selector → pick an endonym → wait for the switch to be APPLIED".
 *
 * Previously three spec files (language.spec.ts, i18n.spec.ts,
 * dashboard-i18n.spec.ts) each hand-rolled this flow with diverging rigor —
 * only the dashboard version waited for the localized trigger aria-label, so
 * the weaker copies could proceed before the locale switch had actually
 * rendered. Everyone gets the strict version now.
 */
import { expect, type Page } from '@playwright/test';
import { SEL, LANGUAGES, I18N_MARKERS } from './selectors';

/** Opens the language selector and picks an item by its endonym (exact match). */
export async function selectLanguage(page: Page, label: string): Promise<void> {
  await page.locator(SEL.langTrigger).first().click();
  await expect(page.locator(SEL.langDropdown)).toBeVisible();
  await page.locator(SEL.langDropdown).getByText(label, { exact: true }).click();
  // The menu closed — the indicator that switching finished.
  await expect(page.locator(SEL.langDropdown)).toHaveCount(0);
  // Deterministic wait for the re-render: the trigger's aria-label is itself
  // localized (lang.label) — when it matches the target locale, the switch has
  // actually been applied (no fixed sleeps).
  const code = LANGUAGES.find((l) => l.label === label)?.code;
  const expectedAria = code ? I18N_MARKERS[code]?.langLabel : undefined;
  if (expectedAria) {
    await expect(page.locator(SEL.langTrigger).first()).toHaveAttribute('aria-label', expectedAria);
  }
}
