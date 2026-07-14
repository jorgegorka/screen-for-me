# Follow-ups deferred from the 2026-07-13 simplification pass

A four-angle cleanup review (reuse / simplification / efficiency / altitude) of the
tauri-rewrite branch was applied on 2026-07-13. These findings were deliberately
**not** applied and are parked here as future work.

## 1. In-process frame grabs for scrolling capture (biggest remaining win)

**Today:** every iteration of the scrolling-capture loop
(`src-tauri/src/capture/scrolling.rs::grab`) spawns `/usr/sbin/screencapture -x -R`,
which PNG-encodes the frame, writes it to app-data on disk, then we re-read and
PNG-decode it. That's process spawn + encode + disk write + read + decode per frame —
plausibly 100–300 ms each — and together with the settle delay it dominates the
wall clock of the whole feature.

**Plan:** capture the region in-process via `CGDisplayCreateImage(rect)` (or
ScreenCaptureKit), yielding raw pixels directly and eliminating all five steps.

**Why it was deferred:** it's a real feature change, not a cleanup —
- Screen Recording (TCC) permission behavior may differ between spawning
  `screencapture` and calling CG/SCK APIs directly; verify the dev-terminal grant
  and the packaged-.app prompt still behave as documented in CLAUDE.md.
- Pixel format/color-space handling (and Retina scale) becomes our job instead of
  the OS encoder's.
- The shared `run_screencapture` helper and `validate_output` semantics
  (missing file = cancelled, <1 KiB = permission problem) don't map 1:1; the
  scrolling path would need its own error taxonomy.
- Keep the one-shot capture paths (`capture/macos.rs`) on `screencapture -i` —
  the interactive crosshair has no in-process equivalent; only the scrolling
  loop's non-interactive grabs should migrate.

## 2. Shrinking the stitch offset-search range (behavior-changing optimization)

**Today:** `stitch.rs::find_scroll_offset` scans every candidate offset
`0..=(h - STRIP_ROWS - strip_start)`. The 2026-07-13 pass already made the scan
cheap (raw-slice row diffs + early exit on a perfect match) **without** changing
results, so the pressure here is low.

**Idea (if profiling ever says it matters):** the true offset is bounded by the
commanded step (`nominal_step_px`, now capped by `stitch::max_detectable_offset`)
plus smooth-scroll slack — search `0..=2×nominal` first and fall back to the full
range on a miss.

**Why it was deferred:** the fallback changes matching behavior on frames where a
distant offset happens to score better than anything near the nominal step; any
change here needs new synthetic-image tests covering the miss-then-fallback path.

## 3. Superseded design specs/plans under docs/superpowers/

`specs/` and `plans/` contain documents describing superseded iterations
(top-strip matching → textured-strip; mean-diff auto-stop → changed-pixel
fraction; fixed steps → glide). They accurately record how the code got here and
were kept as intentional history. Only act if we ever decide the specs dir should
reflect current design only (e.g. move superseded docs to an `archive/` subdir).

## Note: review false positive, already resolved

A reviewer flagged `MIN_SELECTION` in `src/scrollcap/geometry.ts` as dead — it is
actually used by `isSelectable` in the same file. It was un-exported, not deleted.
No further action needed.
