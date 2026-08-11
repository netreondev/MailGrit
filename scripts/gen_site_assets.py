#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 netreon and contributors

"""Generate placeholder site assets (favicon icons + OG image) for MailGrit.

Produces clean, on-brand placeholders so referenced assets are no longer 404:
  docs/assets/icon-192.png  (PWA / apple-touch-icon)
  docs/assets/icon-512.png  (favicon / PWA / JSON-LD logo)
  docs/assets/og.png        (1200x630 Open Graph / Twitter card banner)

Replace these with polished brand artwork later; this only removes the broken
links that hurt SEO/social-preview/PWA rendering today.
"""
from PIL import Image, ImageDraw, ImageFont

# Brand palette (matches meta theme-color + site CSS tokens)
BG = (11, 16, 32)          # #0b1020
BORDER = (31, 42, 74)      # #1f2a4a
ACCENT = (88, 166, 255)    # #58a6ff
FG = (232, 236, 248)       # #e8ecf8
MUTED = (139, 148, 178)    # #8b94b2

ASSETS = r"C:\ISO\MailGrit\docs\assets"


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


def make_og(path):
    W, H = 1200, 630
    img = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(img)

    # decorative top accent line
    d.rectangle([0, 0, W, 6], fill=ACCENT)

    # brand square + mark
    box = 150
    bx, by = 90, 110
    d.rounded_rectangle([bx, by, bx + box, by + box], radius=30, fill=(20, 28, 56))
    d.rounded_rectangle(
        [bx, by, bx + box, by + box], radius=30, outline=BORDER, width=2
    )
    m_font = _font(100, bold=True)
    bbox = d.textbbox((0, 0), "M", font=m_font)
    mw, mh = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(
        (bx + (box - mw) / 2 - bbox[0], by + (box - mh) / 2 - bbox[1] - 2),
        "M",
        font=m_font,
        fill=FG,
    )

    # wordmark
    name_font = _font(86, bold=True)
    d.text((bx + box + 40, by + 18), "MailGrit", font=name_font, fill=FG)

    # tagline
    tag_font = _font(34, bold=False)
    d.text((bx + box + 46, by + 118), "Bulk iRedAdmin automation, from the desktop.",
           font=tag_font, fill=ACCENT)

    # feature bullets
    feats = [
        "Cross-platform Rust desktop app  -  Windows / Linux / macOS ARM",
        "Bulk create, edit and delete mail accounts on iRedAdmin servers",
        "Works behind FortiWeb / WAF via an embedded authenticated browser",
        "Encrypted, tamper-evident audit log  -  9 UI languages",
    ]
    feat_font = _font(28, bold=False)
    y = 330
    for line in feats:
        d.text((bx, y), "  -  " + line, font=feat_font, fill=MUTED)
        y += 52

    # footer
    foot_font = _font(26, bold=False)
    d.text((bx, H - 70), "Free & open source  -  MIT OR Apache-2.0",
           font=foot_font, fill=FG)
    d.text((W - 410, H - 70), "github.com/netreondev/MailGrit",
           font=foot_font, fill=MUTED)

    img.save(path, "PNG", optimize=True)
    print(f"wrote {path}  ({W}x{H})")


if __name__ == "__main__":
    import os
    os.makedirs(ASSETS, exist_ok=True)
    make_icon(192, f"{ASSETS}\\icon-192.png")
    make_icon(512, f"{ASSETS}\\icon-512.png")
    make_og(f"{ASSETS}\\og.png")
    print("done.")
