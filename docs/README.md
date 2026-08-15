<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 Netreon™ and contributors -->

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
| `assets/style.css` | Shared, self-contained, responsive, dark-friendly CSS (system font stacks only — no third-party font CDN). |
| `assets/app.js` | Progressive enhancement (scroll reveal, counters, copy buttons). |
| `assets/logo.svg`, `assets/favicon.svg` | Brand marks. |
| `assets/og.png` | Open Graph / Twitter card image (committed). |
| `assets/icon-192.png`, `assets/icon-512.png` | Favicon / PWA icons (committed). |
| `assets/example.csv` | The sample CSV served by the "try it" section. |
| `llms.txt` | Machine-readable summary for LLM crawlers. |
| `robots.txt` | Allow-all, points to the sitemap. |
| `sitemap.xml` | Two locale URLs with `hreflang` alternates. |
| `site.webmanifest` | PWA manifest (name, theme, icons). |

All referenced assets ARE committed in `docs/assets/` — the site is fully
self-contained.

## Deployment

Deployment is automated by `.github/workflows/pages.yml`: on every push to
`main` that touches `docs/**`, the workflow builds the Pages artifact and
deploys it via the official Pages Actions (configure-pages →
upload-pages-artifact → deploy-pages). No manual Settings→Pages branch
configuration is used — the Source is **GitHub Actions**.

## Updating

Edit the HTML/CSS directly and commit. Each page carries its own SEO metadata
(canonical, Open Graph, Twitter Card, JSON-LD). When you change a page, keep
the two locales in sync: the visible content of `uk/index.html` mirrors
`index.html` one-to-one. Keep `softwareVersion` in the JSON-LD of both pages in
sync with the workspace version in the root `Cargo.toml` (and refresh
`sitemap.xml`'s `lastmod` dates on content changes).
