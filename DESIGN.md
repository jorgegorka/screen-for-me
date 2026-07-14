---
name: Screen for me
description: A screenshot utility that feels like part of the OS
colors:
  accent-violet: "#7c5ce6"
  accent-violet-lift: "#9172e7"
  accent-glow: "#9172e780"
  confirm-green: "#34c759"
  error-red: "#d9342b"
  ink: "#1d1d1f"
  surface-dark: "#2a2a2c"
  control-dark: "#3a3a3c"
  control-dark-hover: "#48484a"
  border-dark: "#4a4a4c"
  text-on-dark: "#eeeeee"
  fog: "#f5f5f7"
  card-white: "#ffffff"
  border-light: "#e2e2e4"
  control-light: "#fafafa"
  text-muted: "#888888"
  text-muted-dark: "#999999"
  hairline-black: "#000000"
  glass-panel: "#18181aeb"
  glass-panel-light: "#18181ad9"
  brand-signal-blue: "#8794ff"
  brand-violet: "#9172e7"
  brand-magenta: "#d05aa6"
  brand-red: "#ee3138"
  brand-black: "#000000"
typography:
  display:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "72px"
    fontWeight: 600
    lineHeight: 1
  title:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 700
  body:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
  label:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  caption:
    fontFamily: "-apple-system, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 400
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "12px"
  pill: "999px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent-violet}"
    textColor: "#ffffff"
    rounded: "{rounded.sm}"
    padding: "5px 10px"
  button-primary-hover:
    backgroundColor: "{colors.accent-violet-lift}"
  button-toolbar:
    backgroundColor: "{colors.control-dark}"
    textColor: "#dddddd"
    rounded: "{rounded.sm}"
    padding: "5px 10px"
  button-toolbar-hover:
    backgroundColor: "{colors.control-dark-hover}"
  button-hud:
    backgroundColor: "#ffffff1a"
    textColor: "{colors.text-on-dark}"
    rounded: "{rounded.sm}"
    padding: "5px 10px"
  card:
    backgroundColor: "{colors.card-white}"
    rounded: "{rounded.lg}"
---

# Design System: Screen for me

## 1. Overview

**Creative North Star: "The Glass HUD"**

Every surface in Screen for me is a lightweight heads-up display floating over the user's own work. The quick-access overlay, the countdown disc, the scrolling-capture pill — they are translucent dark glass panels (#18181A at 85–92% opacity) that appear the instant they're needed and vanish the instant they're not. Even the windowed surfaces (editor, settings, history) inherit that temperament: quiet macOS-native chrome, system font everywhere, controls that look like the OS drew them. The design's job is to keep the capture-to-share loop fast; nothing on screen may draw attention to itself instead of the screenshot.

The system explicitly rejects Electron-app heaviness (oversized padding, web-page-in-a-window controls), feature-bloat pro-tool chrome (icon-crowded ribbons), and startup-SaaS styling (gradients, marketing polish inside a utility). Components are quiet and native: small, subdued, familiar.

**Key Characteristics:**
- Translucent dark glass panels for transient HUD surfaces; opaque native grays for windows
- One accent violet (#7c5ce6, from the brand mark), used only for primary actions and active state
- The blue→violet→magenta→red brand spectrum appears at exactly three earned moments (§6)
- System font stack at compact sizes (11–13px UI); no display or brand fonts
- Flat within windows; soft large shadows only where a panel floats over the screen; active states may glow
- Settings and History respect the OS light/dark preference; HUDs and the editor are always dark

## 2. Colors

A macOS-native gray ramp with a single violet voice drawn from the brand mark; color otherwise appears only as state semantics.

### Primary
- **Signal Violet** (#7c5ce6): the only accent — the brand mark's violet deepened one step so white 12px text passes AA (4.6:1). Primary action buttons, the active tool in the editor toolbar, the active direction in scrolling capture. Hover lightens to **Violet Lift** (#9172e7) — the exact violet sampled from the icon.
- **Violet Lift** (#9172e7): hover state of violet fills, all focus-visible outlines (≥3:1 on both Fog and Graphite), and selection lines drawn over the user's screen (scrolling-capture selection border, crop marquee, text-editing dashed outline).
- **Signal Glow** (`box-shadow: 0 0 10px rgba(145, 114, 231, 0.5)`): the subtle halo on active states — the active editor tool and the active scroll direction. The one place the neon temperament touches chrome.

### Neutral
- **Ink** (#1d1d1f): body text in light mode; the base background of the always-dark editor and dark-mode history.
- **Graphite** (#2a2a2c): the second dark layer — editor toolbar, dark-mode settings background, dark-mode cards.
- **Control Gray** (#3a3a3c): dark-mode button fill, with **#48484a** on hover and **#4a4a4c** hairline borders.
- **Fog** (#f5f5f7): the light-mode window background (settings, history), with white cards and **#e2e2e4** hairline borders.
- **Glass Panel** (#18181A at 92% / 85%): the HUD material — overlay card, countdown disc, capture hint pill, scrolling-capture HUD.
- **Muted** (#6e6e70 on light surfaces, #999 on dark): metadata, hints, empty states. Never for primary content. #6e6e70 is the floor on light backgrounds — anything lighter fails AA at UI sizes.
- **Scrim** (pure black at 12–60% opacity): screen-dimming veils (scrolling-capture backdrop, selection hole-punch) and every shadow. Black transparencies are reserved for depth and dimming, never for surfaces or text.
- **Hairline Black** (#000): the single crisp divider between the editor toolbar and the canvas well.

### Semantic
- **Confirm Green** (#34c759): confirm-a-destructive-step actions only (crop apply). Apple's system green.
- **Error Red** (#d9342b): the error bar and failure toasts. Nothing else is red.
- **Marquee Orange** (#ff9500): the transient drag-selection stroke on the editor canvas (pixelate region). Never appears in chrome.
- **Annotation Inks** (#ff3b30, #ffcc00, #34c759, #4f8ef7, #af52de, #ffffff, #000000): the user's drawing palette — content colors, not UI colors. The blue #4f8ef7 (the pre-rebrand accent) now lives only here.

### Named Rules
**The One Voice Rule.** Signal Violet is the only decorative-adjacent color and it appears exclusively on primary actions, active tools, and selection state — never on labels, borders-at-rest, or backgrounds.

**The Spectrum Rule.** The full brand gradient (Signal Blue → Neon Violet → Magenta Crossing → Signal Red) appears at exactly three moments, always as a thin neon line on dark glass, never as a fill: the overlay stack badge border, the countdown disc ring, and the scrolling-capture pill border while recording. Adding a fourth requires removing one.

**The Glass Rule.** Transient surfaces (things that float over the user's screen) are translucent dark glass. Windowed surfaces are opaque native gray. Never mix the two materials on one surface.

## 3. Typography

**UI Font:** -apple-system, system-ui (with sans-serif fallback) — the platform's own face, everywhere.

**Character:** Invisible. Type in this app should read as the operating system speaking, not a brand. One family, compact sizes, weight doing all the hierarchy work.

### Hierarchy
- **Display** (600, 72px, tabular-nums): the countdown digit only.
- **Title** (700, 18px): window headings (History's "h1"). Rare.
- **Body** (400, 13px): default UI text — settings rows, HUD labels, error bar.
- **Label** (400–600, 12px): buttons, hints, toasts; 600 for the overlay badge at 11px.
- **Caption** (400, 11px): card metadata, timestamps.

### Named Rules
**The Tabular Rule.** Any number that changes in place (countdown, dimensions readout) uses `font-variant-numeric: tabular-nums` so it never jitters.

## 4. Elevation

Flat within windows; floating HUD shadows at the edges. Inside a window, structure comes from background steps (Ink → Graphite → Control Gray) and 1px hairline borders — no shadows between sibling elements. Shadows exist only where a surface genuinely floats over something else: the overlay card over the desktop, the editor stage over its canvas well, toasts and the error bar over content.

### Shadow Vocabulary
- **HUD Float** (`box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45)`): the overlay card and any panel hovering over the user's screen.
- **Stage Lift** (`box-shadow: 0 4px 24px rgba(0, 0, 0, 0.6)`): the screenshot itself in the editor — the one lit object in the room.
- **Toast** (`box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4)`): transient notices (error bar).
- **Thumb** (`box-shadow: 0 1px 6px rgba(0, 0, 0, 0.5)`): the draggable thumbnail inside the overlay.

### Named Rules
**The Floats-Only Rule.** If an element doesn't float over another surface, it gets no shadow. Sibling separation is borders and background steps, never depth. One exception: Signal Glow (§2) on active states — a state indicator, not elevation.

## 5. Components

Quiet and native: controls should look like macOS drew them — small, subdued, no invented styling.

### Buttons
- **Shape:** gently rounded (6px), compact padding (5px 10px), 12px text.
- **Primary:** Signal Violet fill, white text; hover lightens to Violet Lift (#9172e7).
- **Active (tool/direction):** Signal Violet fill plus Signal Glow halo.
- **Toolbar/Secondary (dark):** Control Gray fill (#3a3a3c), #ddd text, #4a4a4c hairline border; hover #48484a.
- **HUD:** 10% white fill over glass, white text, no border; the lightest possible affordance.
- **Confirm:** Confirm Green (#34c759) only for crop-apply-style confirmations.
- **Focus:** visible keyboard focus everywhere — 2px focus-visible outline in Violet Lift.

### Cards / Containers
- **Corner Style:** 10px (cards), 12px (floating HUD panels).
- **Background:** white with #e2e2e4 border in light mode; Graphite with #3a3a3c border in dark.
- **Shadow Strategy:** none at rest (see The Floats-Only Rule).
- **Internal Padding:** 8–10px, dense.

### Inputs / Fields
- Native form controls (select, range, checkbox) unstyled beyond sizing — the OS-native look is the design.
- **Disabled:** 45% opacity, pointer-events off.

### Swatches (editor color picker)
- 18px circles, 2px transparent border; the active swatch gets a white border ring.

### Signature Component: The Glass HUD Panel
The overlay card, countdown disc, hint pill, and scrollcap HUD are one family: #18181A glass at 85–92% opacity, 10–12px radius (999px for pills, 50% for the disc), white text with 55–85% opacity for secondary lines, HUD buttons inside. This is the app's most recognizable surface — keep the material identical across all of them.

## 6. Brand Mark

The app icon is the brand's voice at full volume: a rounded-square screen outline traced as a single neon waveform, running a spectrum from blue through violet and magenta to red, glowing on pure black. It reads as "a screen drawn by a signal" — the capture loop as a live wire. The UI speaks the same identity at a whisper: the mark's violet as its one accent, and the spectrum as a thin line at three earned moments (see the rules below).

### Palette (sampled from `assets/icon.png`)
- **Signal Blue** (#8794ff): the waveform's left tail — where the spectrum begins.
- **Neon Violet** (#9172e7): the left half of the frame and waveform.
- **Magenta Crossing** (#d05aa6): the transition at the top of the frame.
- **Signal Red** (#ee3138): the right half of the frame and waveform.
- **Void Black** (#000000): the icon field. Matches Hairline Black; the darkest value in the system.

### Named Rules
**The Signal Accent.** The UI speaks the mark's violet: Signal Violet (#7c5ce6, the mark's violet deepened for AA text contrast) is the single accent, Violet Lift (#9172e7, the sampled icon violet) its hover/outline companion. Only the violet crosses over — Signal Red stays out of chrome because red means error (#d9342b), and Magenta Crossing appears only inside the spectrum line.

**The Neon Is a Line, Not a Fill.** Wherever the brand shows up in the UI it takes the icon's own form: a thin glowing line on dark glass (the spectrum borders and ring of The Spectrum Rule, the Signal Glow halo, the selection border over the screen). Never a filled gradient surface, never a glow on a resting element. The utility stays quiet; the brand is the live wire running through it.

### Brand surfaces proper
- **Landing page** (brand register per PRODUCT.md): the palette's full voice — Void Black ground, the spectrum as hero energy.
- **About dialog**: the icon at rest; no gradient re-creations around it.

## 7. Do's and Don'ts

### Do:
- **Do** use Signal Violet (#7c5ce6) for exactly one thing per surface: the primary action or the active state.
- **Do** keep UI text at 11–13px in the system font; density is native, not cramped.
- **Do** build transient surfaces from the glass material (#18181A at 85–92%) and windows from opaque native grays.
- **Do** respect `prefers-color-scheme` on windowed surfaces (settings, history) and keep HUDs/editor always dark.
- **Do** convey structure with hairline borders and background steps; reserve shadows for floating panels.
- **Do** keep interactions instant; any future transition stays in the 150ms range, conveys state, and respects `prefers-reduced-motion`.

### Don't:
- **Don't** introduce Electron-app heaviness: oversized paddings, web-styled controls, or window chrome that doesn't feel native (PRODUCT.md anti-reference).
- **Don't** grow feature-bloat pro-tool chrome: no icon-crowded toolbars, ribbons, or option overload (PRODUCT.md anti-reference).
- **Don't** add startup-SaaS styling: no decorative gradients, onboarding tours, or marketing polish inside the utility (PRODUCT.md anti-reference). The brand spectrum's three line-moments (The Spectrum Rule) are the sanctioned exception.
- **Don't** use blue as chrome. #4f8ef7 and #4a9eff are legacy accents; on contact, migrate to Signal Violet / Violet Lift. #4f8ef7 survives only as an annotation ink.
- **Don't** restyle native form controls or scrollbars.
- **Don't** use red or green for anything but errors and confirmations respectively — Signal Red belongs to the mark, not to chrome.
- **Don't** add decorative motion, load choreography, or hover effects beyond a background-step change.
- **Don't** use the spectrum as a fill or add glows to resting elements (see The Neon Is a Line, Not a Fill).
