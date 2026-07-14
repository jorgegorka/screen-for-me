# Remotion Prompt — "Screen for me" launch video

Copy everything below the line into Remotion (or an AI agent generating a Remotion project).

---

## The brief

Build a **55-second, 1920×1080, 30 fps** Remotion video (TypeScript, one `<Composition>`) that launches **Screen for me** — a fast, native screenshot app for macOS and Linux. The video must feel like the brand: **a live neon signal running through a pitch-black room**. Confident, kinetic, zero corporate fluff. Think Apple keynote pacing crossed with a synthwave title sequence — but disciplined: the neon is always a **thin glowing line, never a filled gradient surface**.

## Brand system (use these exactly)

- **Ground:** pure black `#000000`. The entire video lives on black.
- **The Spectrum** (the brand's signature — a neon gradient used ONLY as thin glowing strokes, rings and underlines, 2–3px, with a soft outer glow):
  `#8794ff` (signal blue) → `#9172e7` (neon violet) → `#d05aa6` (magenta) → `#ee3138` (signal red)
- **Accent:** Signal Violet `#7c5ce6` for buttons/highlights; hover/glow companion `#9172e7`.
- **Glow:** `0 0 10px rgba(145,114,231,0.5)` on active elements; spectrum strokes get a stronger bloom.
- **Glass HUD material:** `#18181A` at 88% opacity, 12px radius, `0 8px 32px rgba(0,0,0,0.45)` shadow — every mock UI panel in the video is made of this.
- **Type:** system font stack (`-apple-system, system-ui, sans-serif`). Headlines 600–700 weight, tight tracking. Numbers use `tabular-nums`.
- **App icon:** `assets/icon.png` (a rounded-square screen outline traced as a single neon waveform on black). Never recolor it.
- Text is white; secondary text white at 60% opacity. No blues/greens/reds in chrome — the spectrum line and Signal Violet carry all the color.

## Motion language

- Springs, not linear easings (`spring({ damping: 200 })` for slides, snappier `damping: 20, stiffness: 200` for pops).
- The recurring motif: a **thin spectrum line that "draws" itself** (animated stroke-dashoffset on an SVG path) — it traces frames around screenshots, underlines headlines, and finally traces the app icon outline in the finale.
- Scene transitions: hard cuts on the beat or a quick 8-frame black dip. No cheesy wipes.
- Every scene has one hero element and lots of black negative space.

## Scene-by-scene (frames at 30 fps, total 1650 frames = 55 s)

### 1 — Hook (0–180, 6 s)
Black screen. A single point of blue light appears center-left and **draws a horizontal neon spectrum line** across the screen (blue→violet→magenta→red), leaving a glowing trail. As it completes, the headline snaps in above it, word by word (3-frame stagger, spring pop):

> **"Your screenshots deserve better."**

Beat. The line pulses once, then the whole lockup scales down 8% and cuts.

### 2 — Global explanation (180–390, 7 s)
Headline top-center: **"Screen for me — capture, annotate, share. In seconds."**
Below it, a stylized capture loop plays out: a crosshair cursor draws a violet dashed **selection marquee** over a blurred abstract "desktop" (dark glass rectangles suggesting windows — do NOT fake a real brand's UI). The selected region flashes white for 2 frames (shutter), lifts off as a floating screenshot card with the Stage Lift shadow, and a spectrum line traces its border once. Sub-line fades in: *"The fastest capture-to-share loop on your Mac."*

### 3 — Three capture modes (390–600, 7 s)
Headline: **"Area. Window. Full screen."**
Three glass HUD cards slide up in sequence (150 ms stagger), each with an icon (marquee / window / display) and a keyboard shortcut rendered as physical keycaps: **⌘⇧7**, **⌘⇧8**, **⌘⇧9**. As each card lands, its keycaps press down (2-frame dip) and the card's border lights up with a brief violet glow. Caption: *"From the menu bar or a global shortcut — you're never more than one keystroke away."*

### 4 — Quick-access overlay + drag-out (600–840, 8 s)
Headline: **"Captured. Now it's already where you need it."**
Bottom-left of frame: the app's signature **glass overlay panel** appears with a screenshot thumbnail and four HUD buttons: Copy · Save · Finder · Edit. Then the hero move — the thumbnail is **dragged out** of the panel by a cursor, flies with a spring arc across the screen, and drops into a chat-style input box (generic dark glass chat mock), landing with a subtle bounce and a violet glow ring. Caption: *"Copy, save, or drag it straight into any app."*

### 5 — Annotation editor (840–1140, 10 s) — the longest scene
Headline: **"Annotate like you mean it."**
A screenshot card sits center-stage on the dark editor ground (Stage Lift shadow). Tools fire in rapid, satisfying succession, each animating on:
1. A **violet arrow** draws itself onto the image (stroke animation, spring overshoot on the head).
2. A **rectangle** outlines a region; a **highlighter** stroke sweeps across a line of fake text.
3. **Numbered counter badges** ①②③ pop onto three spots (scale spring, 4-frame stagger).
4. A region gets **pixelated** live (animate a mosaic filter growing over it).
5. A **crop** closes in: dark scrim outside the crop marquee, then the canvas snaps to the cropped size.
Bottom of frame: a compact dark toolbar (glass, tiny 12px icons) where the active tool glows violet as each step fires. Caption cycles: *"Arrows · shapes · text · counter steps · pixelate · crop — undo/redo, native-resolution export."*

### 6 — Scrolling + timed capture (1140–1350, 7 s)
Split moment, two beats:
- **Beat 1 — Scrolling capture:** a tall page scrolls inside a viewport while a **spectrum-bordered recording pill** pulses at the bottom; the captured strip extends downward off-screen, then snaps together into one long stitched image that zooms to fit. Caption: *"Capture entire scrolling pages — stitched into one image."*
- **Beat 2 — Timed capture:** a glass **countdown disc** with a spectrum ring counts **3 → 2 → 1** (72px tabular digits, ring depleting), then a white shutter flash. Caption: *"Or put it on a timer."*

### 7 — Rapid-fire trust bar (1350–1440, 3 s)
Three short lines punch in and out fast (25 frames each), centered, big:
**"Customisable shortcuts."** → **"English · Español · Français · Deutsch · Italiano."** → **"Native on macOS & Linux."**
Each line gets a thin spectrum underline that draws in beneath it.

### 8 — Finale (1440–1650, 7 s)
Black. The spectrum line returns and **traces the app icon's rounded-square outline** (mirroring the real mark), then the actual icon (`assets/icon.png`) fades in inside the traced frame with a soft bloom. Below it, stacked and spring-staggered:
- **Screen for me** (700 weight, ~64px, white)
- **100% Free** — set in Signal Violet `#7c5ce6`, with a one-time glow pulse. Make it unmissable.
- **screenforme.app** (white, 60% opacity, ~24px)
Right of the lockup (or below on the centerline): a **QR code linking to `https://screenforme.app`** — white modules on black, ~220px, framed by a thin spectrum border, fading in last with the caption *"Scan to download."* Generate it with the `qrcode` npm package (render to data URL in a `useEffect`/`delayRender` pattern, or pre-generate as a static SVG asset). Hold the full lockup for the final 90 frames so the QR is comfortably scannable.

## Implementation notes

- Use `@remotion/transitions` only if it stays on hard cuts/fades; otherwise sequence manually with `<Series>` / `<Sequence>`.
- Build the spectrum line as a reusable `<SpectrumLine>` component: SVG path + `linearGradient` (the four hex stops) + `stroke-dashoffset` interpolation + an SVG gaussian-blur duplicate underneath for the glow.
- Keycaps, HUD panels, toolbar, countdown disc: plain styled divs with the glass material — no screenshots of the real app needed; the video is a stylized recreation, which keeps it crisp at 1080p.
- All timing math via `useCurrentFrame()` + `spring`/`interpolate`; no CSS animations.
- Add a subtle constant film-like vignette (radial black, 20% at corners) to keep focus center-frame.
- Optional audio hook: structure scenes on an implied 120 BPM grid (cuts every 15/30/45 frames) so a music track drops in cleanly later.

## Hard rules

- The spectrum gradient is **never a fill or a background** — lines, rings and underlines only.
- No real third-party app UI or logos (chat mock stays generic).
- No stock footage, no emoji, no lorem ipsum visible at readable sizes.
- Total runtime ≤ 55 seconds. The finale (logo / 100% Free / URL / QR) must hold at least 3 full seconds.
