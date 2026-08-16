#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Netreon™ and contributors

"""Generate placeholder site assets (favicon icons) for MailGrit.

Produces clean, on-brand placeholders so referenced assets are no longer 404:
  docs/assets/icon-192.png  (PWA / apple-touch-icon)
  docs/assets/icon-512.png  (favicon / PWA / JSON-LD logo)

The OG banner (docs/assets/og.png) is NOT generated here: .meta/og-render.html
is the canonical, hand-tuned OG generator (palette #08080C/#2563EB) — this
script's old og.png generator had drifted apart from it (palette
#0b1020/#58a6ff), and two generators for one asset eventually overwrite each
other's output.

Replace these with polished brand artwork later; this only removes the broken
links that hurt SEO/social-preview/PWA rendering today.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# Brand palette (matches meta theme-color + site CSS tokens) — only the colors
# make_icon actually uses; the OG render has its own palette in .meta/.
BG = (11, 16, 32)          # #0b1020
BORDER = (31, 42, 74)      # #1f2a4a
FG = (232, 236, 248)       # #e8ecf8

# Resolved relative to THIS FILE so the script works from any checkout / cwd
# (previously a hardcoded C:\ISO\MailGrit\... absolute path — it only ever
# worked on one specific dev machine).
ASSETS = Path(__file__).resolve().parents[1] / "docs" / "assets"


def _font(size, bold=False):
    """Return a system UI font, tolerating platforms where some are absent."""
    candidates = [
        "segoeuib.ttf" if bold else "segoeui.ttf",
        "segoeuib.ttf" if bold else "SegoeUI.ttf",
        "arialbd.ttf" if bold else "arial.ttf",
        "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf",
    ]
    for name in candidates:
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def make_icon(size, path):
    img = Image.new("RGB", (size, size), BG)
    d = ImageDraw.Draw(img)
    radius = int(size * 0.19)
    # subtle inner border
    margin = int(size * 0.0625)
    d.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=int(radius * 0.83),
        outline=BORDER,
        width=max(1, int(size * 0.006)),
    )
    # brand mark "M"
    font = _font(int(size * 0.6), bold=True)
    text = "M"
    bbox = d.textbbox((0, 0), text, font=font)
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(
        ((size - w) / 2 - bbox[0], (size - h) / 2 - bbox[1] - int(size * 0.01)),
        text,
        font=font,
        fill=FG,
    )
    img.save(path, "PNG", optimize=True)
    print(f"wrote {path}  ({size}x{size})")


if __name__ == "__main__":
    ASSETS.mkdir(parents=True, exist_ok=True)
    make_icon(192, ASSETS / "icon-192.png")
    make_icon(512, ASSETS / "icon-512.png")
    print("done. (og.png is generated from .meta/og-render.html — see the module docstring)")
