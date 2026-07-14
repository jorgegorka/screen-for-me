# Screen for me — landing page

A dependency-free static page: `index.html` + `site.css` + `site.js`. No build
step. Deployed to GitHub Pages at **https://screenforme.app** by
`.github/workflows/pages.yml`, which uploads this folder on every push to
`master` that touches `site/`.

```bash
# preview locally
cd site && python3 -m http.server 8000
# → http://localhost:8000
```

## Dropping in real screenshots and the demo video

Every product visual on the page is currently a hand-built CSS/SVG recreation.
Each one is also a **media slot**: put a file with the matching name into
`site/assets/`, add its filename to `site/assets/media.json`, and `site.js`
swaps the recreation for your real asset on load. No HTML edits needed.

```json
["demo.mp4", "feature-overlay.png"]
```

| File to add                      | Replaces                                        |
| -------------------------------- | ----------------------------------------------- |
| `assets/hero.png`                | Hero scene inside the neon frame                |
| `assets/demo.mp4`                | "The whole loop" video slot (English page)      |
| `assets/demo-{es,fr,de,it}.mp4`  | Same slot on the translated pages               |
| `assets/demo-poster.png`         | Poster frame for the video (optional)           |
| `assets/feature-capture.png`     | Area-capture visual (crosshair + marquee)       |
| `assets/feature-overlay.png`     | Quick-access overlay visual                     |
| `assets/feature-editor.png`      | Annotation editor visual                        |
| `assets/feature-scrollcap.png`   | Scrolling-capture visual                        |

Tips:

- Export screenshots at 2× (Retina) for crispness; the frames are ~660px wide
  at 1×, so ~1320px-wide images are ideal. The hero is ~960×560 at 1× (16:9.33).
- PNGs are shown with `object-fit` untouched, so shoot at roughly the frame's
  aspect ratio (features ≈ 6:4.4, hero ≈ 960:560, video 16:9).
- The swap fetches `media.json`, so it only works over HTTP(S), not `file://`.
  Use the local server above to check your assets.

## Languages

The page ships in the app's five languages: `/` (English), `/es/`, `/fr/`,
`/de/`, `/it/` — one static `index.html` per language, cross-linked via the
footer switcher and `hreflang` alternates. All pages share the root `site.css`,
`site.js`, and `assets/` (media slots included, so one set of screenshots
serves every language).

The four translated pages are **generated from the English page** by
`scripts/gen-i18n.py` (translation table + path rewrites; it aborts if any
English string no longer matches, so translations can't silently go stale).
After editing copy in `index.html`, update the table and re-run:

```bash
python3 scripts/gen-i18n.py
```

Text inside the CSS product recreations (Copy/Save/Edit, the recording pill)
deliberately stays English on every page: the media slots are shared, so the
real screenshots that replace them will be a single language too.

The `hreflang` alternates are absolute (`https://screenforme.app/…`) and
identical on every page; `canonical` and `og:url` are per-language and set by
the generator.

## Editing notes

- Colors, spacing, and type tokens live at the top of `site.css`.
- The blue→violet→magenta→red gradient is defined once as `#spectrum` in
  `index.html` and reused per DESIGN.md's Spectrum Rule (lines, never fills).
- Download CTAs point at `releases/latest` on GitHub; nothing to update per release.
