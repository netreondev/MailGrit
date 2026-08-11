<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 netreon and contributors -->

# MailGrit landing site

This directory is the source of the public GitHub Pages landing site for
**MailGrit**, served at <https://netreondev.github.io/MailGrit>.

It is intentionally a set of **static files** — hand-written HTML and CSS, with
**no Node/npm/build step**. Everything you need to publish is right here in
`/docs`.

## Contents

| Path | Purpose |
|------|---------|
| `index.html` | English landing page (canonical, `hreflang="en"`). |
| `uk/index.html` | Ukrainian translation, 1:1 structure (`hreflang="uk"`). |
| `assets/style.css` | Shared, self-contained, responsive, dark-friendly CSS. |
| `robots.txt` | Allow-all, points to the sitemap. |
| `sitemap.xml` | Two locale URLs with `hreflang` alternates. |
| `site.webmanifest` | PWA manifest (name, theme, icons). |

## Assets referenced but not in this repo

The HTML references the following binary assets, which you should drop into
`docs/assets/` when ready (they are referenced by URL only — the site works
without them, they just won't resolve until added):

- `assets/og.png` — Open Graph / Twitter card image.
- `assets/icon-192.png` and `assets/icon-512.png` — favicon / PWA icons.

## Enable the site

1. Push the `/docs` folder to the `main` branch.
2. In the repository on GitHub, open **Settings → Pages**.
3. Under **Build and deployment → Source**, choose **Deploy from a branch**.
4. Select branch **`main`** and folder **`/docs`**.
5. Save. The site goes live at `https://netreondev.github.io/MailGrit/` within a
   minute or two.

There is nothing to install and nothing to build — GitHub Pages serves the
files as-is.

## Updating

Edit the HTML/CSS directly and commit. Each page carries its own SEO metadata
(canonical, Open Graph, Twitter Card, JSON-LD). When you change a page, keep
the two locales in sync: the visible content of `uk/index.html` mirrors
`index.html` one-to-one.
