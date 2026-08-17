# `.meta/` — site asset tooling

This folder holds **source templates** for site assets that ship as generated
artifacts. It is not deployed to GitHub Pages (only `docs/` is).

## `og-render.html` → `docs/assets/og.png`

`og-render.html` is a self-contained 1200×630 HTML canvas that renders the
Open Graph / Twitter Card preview image for the landing site. It uses the
app's dark palette and inline CSS, so it renders identically anywhere.

### Regenerating `og.png`

The PNG is produced by rendering the canvas at exactly **1200×630** and
saving the viewport screenshot. No build tool or image editor is needed —
any headless browser works.

**Option A — Playwright (one-liner), from the repo root:**

```bash
# serve docs/ + .meta/ is already at repo root, so serve the repo root
python -m http.server 8765 --bind 127.0.0.1 &
npx playwright screenshot --viewport-size=1200,630 \
  http://127.0.0.1:8765/.meta/og-render.html docs/assets/og.png
```

**Option B — browser devtools:**

1. Open `.meta/og-render.html` in a Chromium browser.
2. DevTools → Toggle device toolbar → set dimensions to **1200 × 630**.
3. Command palette → "Capture screenshot" → save as `docs/assets/og.png`.

**After editing copy/colors in the canvas**, regenerate the PNG and commit
both the template and the PNG together. The OG metadata in
`docs/index.html` and `docs/uk/index.html` already points at
`/assets/og.png`, so no markup change is needed.
