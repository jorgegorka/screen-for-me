# Product

## Register

product

## Platform

web

## Users

General Mac and Linux users — anyone who wants better screenshots than the OS default. They're mid-task in some other app when they reach for this tool: they hit a shortcut, grab a region, and want the image somewhere useful (clipboard, a chat window, a document) within seconds. Occasional and casual use is the norm, so nothing can depend on learned muscle memory beyond the three capture shortcuts.

Note: the app UI is the primary surface and this file's register reflects it, but a marketing landing page is planned. When working on that surface, treat it as the brand register per-task.

## Product Purpose

A fast, polished screenshot utility for macOS and Linux: area/window/fullscreen capture from a menu-bar icon or global shortcuts, a quick-access overlay after every capture (copy, save, drag-out), and a built-in annotation editor with native-resolution export. Success is public release traction — a polished 1.0 that people download, star, and recommend.

## Positioning

The fastest capture-to-share loop. Shortcut → overlay → (optionally annotate) → drag the image straight into the app you were already using. Speed of that loop is the product; every screen either shortens it or gets out of its way.

## Brand Personality

Invisible, native, fast — carrying a signal. The brand mark is a screen outline traced as a neon waveform (blue → violet → magenta → red on black): the capture loop as a live wire. Inside the app that identity speaks at a whisper — the mark's violet is the single accent, and the full spectrum appears only as a thin neon line at a few earned moments (see DESIGN.md's Spectrum Rule). Everything else still feels like part of the operating system: interactions are instant, chrome is minimal, the tool disappears into the task. The loud front door, the quiet room.

## Anti-references

- **Electron-app heaviness**: web-page-in-a-window feel, non-native controls, oversized paddings, sluggish interactions.
- **Feature-bloat pro tools** (Snagit-style): toolbars with dozens of icons, ribbons, option overload on every screen.
- **Startup-SaaS styling**: decorative gradients, marketing polish, onboarding tours, badges or upsells inside a utility. (The brand spectrum's three sanctioned line-moments in DESIGN.md are the only exception.)

## Design Principles

1. **Every screen serves the loop.** Capture → share is the whole product; UI that doesn't shorten that loop must not lengthen it.
2. **Feel native, not embedded.** Match macOS (and the Linux desktop) in materials, density, and control vocabulary, even though the surfaces are webviews.
3. **Casual-user legible.** No hidden gestures or pro-tool assumptions; every affordance readable on first encounter, with shortcuts as accelerators, not requirements.
4. **Restraint over features.** One good way to do each thing; resist toolbar sprawl as capabilities grow.
5. **Instant or nothing.** Interactions respond immediately; motion only conveys state, never decoration.

## Accessibility & Inclusion

Sensible defaults, no formal compliance target: WCAG AA contrast throughout, full keyboard operability, and `prefers-reduced-motion` respected on any animation.
