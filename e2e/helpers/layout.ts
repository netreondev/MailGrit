// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Netreon™ and contributors

/**
 * Visual-quality assessment utilities: symmetry/layout, theme/contrast,
 * accessibility (a11y). Pure reusable functions with no external dependencies
 * (only Playwright Page/Locator/expect).
 *
 * Used in the dashboard-layout / dashboard-theme / a11y spec files for an
 * objective, data-driven assessment of UI "quality, symmetry, usability,
 * clarity" (rather than just "visible/not visible").
 */
import { expect, type Locator, type Page } from '@playwright/test';

/** Element rectangle (as returned by Playwright `boundingBox`). */
export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * The real window size. `page.viewportSize()` returns null for pages attached
 * via `connectOverCDP` (Playwright did not launch this browser), so a hardcoded
 * fallback silently mismatched whenever the OS clamps the window — e.g. GitHub
 * runners clamp the 1120x780 request to the 1024x768 desktop, which skewed the
 * centering assertions by exactly half the difference (46px, 2026-08-16).
 * `window.innerWidth/innerHeight` is the truth for the actual window.
 */
export async function viewport(page: Page): Promise<{ width: number; height: number }> {
  return page.evaluate(() => ({ width: window.innerWidth, height: window.innerHeight }));
}

/**
 * Viewport metrics plus the available screen area, for "fills the desktop"
 * assertions: on CI runners that clamp the window to the full desktop a
 * maximize cannot grow either dimension, so the honest postcondition is
 * "covers the available screen", not "larger than before".
 */
export async function viewportWithScreen(
  page: Page,
): Promise<{ width: number; height: number; availWidth: number; availHeight: number }> {
  return page.evaluate(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
    availWidth: window.screen.availWidth,
    availHeight: window.screen.availHeight,
  }));
}

/**
 * The element is fully inside the viewport (not clipped, not pushed off-edge).
 * Allows small negative x/y values for elements with shadow/glow.
 */
export async function assertInViewport(
  page: Page,
  selector: string,
  message?: string,
): Promise<void> {
  const loc = page.locator(selector).first();
  const box = await loc.boundingBox();
  expect(box, `element "${selector}" must have a boundingBox (${message ?? ''})`).not.toBeNull();
  const vp = await viewport(page);
  expect(box!.x, `"${selector}" x >= 0 (${message ?? ''})`).toBeGreaterThanOrEqual(-2);
  expect(box!.y, `"${selector}" y >= 0 (${message ?? ''})`).toBeGreaterThanOrEqual(-2);
  expect(box!.x + box!.width, `"${selector}" right edge <= viewport width (${message ?? ''})`).toBeLessThanOrEqual(vp.width);
  expect(box!.y + box!.height, `"${selector}" bottom edge <= viewport height (${message ?? ''})`).toBeLessThanOrEqual(vp.height);
}

/**
 * The element is centered horizontally relative to the viewport.
 * The element center must match the viewport center (within tolerance).
 */
export async function assertCenteredHorizontally(
  loc: Locator,
  page: Page,
  tolerancePx = 6,
  message?: string,
): Promise<void> {
  const box = await loc.boundingBox();
  expect(box, `element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  const vp = await viewport(page);
  const center = box!.x + box!.width / 2;
  const vpCenter = vp.width / 2;
  expect(
    Math.abs(center - vpCenter),
    `element center (${center}) ~= viewport center (${vpCenter}), tolerance ${tolerancePx}px (${message ?? ''})`,
  ).toBeLessThanOrEqual(tolerancePx);
}

/**
 * The element is centered both horizontally and vertically (for modals —
 * `.modal-backdrop` uses flex align/justify center).
 */
export async function assertCenteredBoth(
  loc: Locator,
  page: Page,
  tolerancePx = 10,
  message?: string,
): Promise<void> {
  const box = await loc.boundingBox();
  expect(box, `element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  const vp = await viewport(page);
  const cx = box!.x + box!.width / 2;
  const cy = box!.y + box!.height / 2;
  expect(Math.abs(cx - vp.width / 2), `horizontal center (${message ?? ''})`).toBeLessThanOrEqual(tolerancePx);
  expect(Math.abs(cy - vp.height / 2), `vertical center (${message ?? ''})`).toBeLessThanOrEqual(tolerancePx);
}

/**
 * Two elements do not overlap (there is a horizontal OR vertical gap between
 * them). Useful to verify that cards/buttons do not collide with each other.
 */
export async function assertNoOverlap(
  a: Locator,
  b: Locator,
  message?: string,
): Promise<void> {
  const [boxA, boxB] = await Promise.all([a.boundingBox(), b.boundingBox()]);
  expect(boxA, `first element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  expect(boxB, `second element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  const horizontalGap = Math.max(0, Math.max(boxA!.x - (boxB!.x + boxB!.width), boxB!.x - (boxA!.x + boxA!.width)));
  const verticalGap = Math.max(0, Math.max(boxA!.y - (boxB!.y + boxB!.height), boxB!.y - (boxA!.y + boxA!.height)));
  const noOverlap = horizontalGap > 0 || verticalGap > 0;
  expect(noOverlap, `elements must not overlap (${message ?? ''})`).toBe(true);
}

/**
 * Two elements are aligned on the left edge (same x) — a check for grid/column
 * symmetry.
 */
export async function assertAlignedLeft(
  a: Locator,
  b: Locator,
  tolerancePx = 2,
  message?: string,
): Promise<void> {
  const [boxA, boxB] = await Promise.all([a.boundingBox(), b.boundingBox()]);
  expect(boxA, `first element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  expect(boxB, `second element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  expect(Math.abs(boxA!.x - boxB!.x), `left edges aligned (${message ?? ''})`).toBeLessThanOrEqual(tolerancePx);
}

/**
 * Symmetry of the left/right margins between the element and the viewport edges.
 * For centered containers (`margin: 0 auto`): left-margin ~= right-margin.
 */
export async function assertSymmetricMargins(
  loc: Locator,
  page: Page,
  tolerancePx = 4,
  message?: string,
): Promise<void> {
  const box = await loc.boundingBox();
  expect(box, `element must have a boundingBox (${message ?? ''})`).not.toBeNull();
  const vp = await viewport(page);
  const leftMargin = box!.x;
  const rightMargin = vp.width - (box!.x + box!.width);
  expect(
    Math.abs(leftMargin - rightMargin),
    `left margin (${leftMargin}) ~= right (${rightMargin}), tolerance ${tolerancePx}px (${message ?? ''})`,
  ).toBeLessThanOrEqual(tolerancePx);
}

/**
 * Reads the value of a CSS variable from `:root` (a design token).
 * Returns the raw string (e.g. "#08080C", "rgba(255,255,255,0.08)").
 */
export async function getCssVar(page: Page, name: string): Promise<string> {
  return page.evaluate(
    ([n]) => getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
    [name],
  );
}

/**
 * Parses a CSS color into RGB (0-255). Supports #hex and rgb()/rgba().
 * Returns null for empty/unparseable values.
 */
export function parseColor(raw: string): { r: number; g: number; b: number } | null {
  const s = raw.trim().toLowerCase();
  if (s.startsWith('#')) {
    const hex = s.slice(1);
    if (hex.length === 3) {
      return {
        r: parseInt(hex[0]! + hex[0]!, 16),
        g: parseInt(hex[1]! + hex[1]!, 16),
        b: parseInt(hex[2]! + hex[2]!, 16),
      };
    }
    if (hex.length === 6) {
      return {
        r: parseInt(hex.slice(0, 2), 16),
        g: parseInt(hex.slice(2, 4), 16),
        b: parseInt(hex.slice(4, 6), 16),
      };
    }
    return null;
  }
  const rgbMatch = s.match(/rgba?\(([^)]+)\)/);
  if (rgbMatch) {
    const parts = rgbMatch[1]!.split(',').map((p) => parseFloat(p.trim()));
    if (parts.length >= 3) {
      return { r: parts[0]!, g: parts[1]!, b: parts[2]! };
    }
  }
  return null;
}

/** Relative luminance of a color per WCAG 2.1 (for contrast calculation). */
function relativeLuminance(c: { r: number; g: number; b: number }): number {
  const channel = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

/** Contrast ratio of two colors (WCAG 2.1), 1-21. */
export function contrastRatio(
  fg: { r: number; g: number; b: number },
  bg: { r: number; g: number; b: number },
): number {
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Checks that the fg/bg contrast (from CSS variables) is >= minRatio (WCAG).
 * For normal text AA = 4.5, AAA = 7; for large text AA = 3.
 */
export async function assertContrast(
  page: Page,
  fgVar: string,
  bgVar: string,
  minRatio: number,
  message?: string,
): Promise<void> {
  const [fgRaw, bgRaw] = await Promise.all([getCssVar(page, fgVar), getCssVar(page, bgVar)]);
  const fg = parseColor(fgRaw);
  const bg = parseColor(bgRaw);
  expect(fg, `CSS var ${fgVar} must be a valid color (got "${fgRaw}")`).not.toBeNull();
  expect(bg, `CSS var ${bgVar} must be a valid color (got "${bgRaw}")`).not.toBeNull();
  const ratio = contrastRatio(fg!, bg!);
  expect(ratio, `contrast ${fgVar}/${bgVar} = ${ratio.toFixed(2)} >= ${minRatio} (${message ?? ''})`).toBeGreaterThanOrEqual(minRatio);
}

/**
 * Checks that the visible text of an element contains no "broken" i18n keys
 * (raw keys like `nav.operations` instead of translated text) and is not empty.
 */
export function assertNoRawKey(text: string, message?: string): void {
  expect(text.trim(), `text must not be empty (${message ?? ''})`).not.toBe('');
  // A raw key contains a dot and Latin letters with no spaces (e.g. nav.audit).
  const looksLikeKey = /^[a-z_]+\.[a-z_.]+$/i.test(text.trim());
  expect(looksLikeKey, `text "${text}" must not be a raw i18n key (${message ?? ''})`).toBe(false);
}
