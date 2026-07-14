# Design: Tapered (Skitch-style) editor arrows

Date: 2026-07-14
Status: approved

## Purpose

Replace the editor's constant-width `Konva.Arrow` with a solid tapered arrow:
the tail starts near a point, the shaft widens smoothly toward the head, and
the head is a filled triangle. All dimensions scale with the arrow's length,
so the arrow visually "grows" out of the tail point while dragging. No
animation after release (DESIGN.md: no decorative motion).

## Approach

Custom `Konva.Shape` with a `sceneFunc` that fills a 7-point polygon computed
by a **pure, Konva-free geometry function** (unit-testable in vitest, like
`geometry.ts`/`history.ts`). A single factory creates the shape and is used by
both draw paths so drafts and undo-rehydrated arrows are identical.

Rejected alternatives:

- **Closed `Konva.Line` polygon** — serializes the derived 14-number polygon
  instead of the two endpoints, so re-deriving on stroke-width change is
  awkward and transformer scaling distorts the taper.
- **Two-node group (line + wedge)** — group management leaks into selection,
  undo, and export.

## Details

- **Geometry** (`src/editor/arrow.ts`, Konva-free):
  `taperedArrowPoints(x1, y1, x2, y2, strokeWidth)` returns the filled polygon
  vertices: pointed tail → widening shaft → triangular head → back.
  - Head length ≈ `6 + strokeWidth * 2.5` (today's pointer sizing), head
    half-width proportional; shaft half-width at the head-base derives from
    `strokeWidth`; tail half-width ≈ 15% of that (near-point).
  - **Length clamp**: head length and widths are capped by a fraction of the
    arrow's total length, so short arrows render small and proportionate —
    this produces the "grows while dragging" feel.
  - Degenerate (zero-length) input returns an empty polygon (draw nothing).
- **Factory**: creates a `Konva.Shape` named `"arrow"` whose `sceneFunc` reads
  `points` (two endpoints), `fill`, and `strokeWidth` attrs live and fills the
  polygon (`fillStrokeShape` with fill only; no stroke outline).
- **Editor wiring** (`src/editor/main.ts`): `startDraft` uses the factory;
  `onPointerMove` keeps updating only `points` (unchanged code path — the
  `"arrow"`/`"line"` case). The color-change path already sets `fill` on
  arrows and keeps working.
- **Serialization** (`src/editor/shapes.ts`): ATTRS for `arrow` becomes
  `[...COMMON, "points", "fill", "strokeWidth"]` (drop `stroke`,
  `pointerLength`, `pointerWidth`); `createNode`'s `"arrow"` case goes through
  the same factory so undo/redo rehydrates the identical shape.
- **Selection/export**: `sceneFunc` shapes participate in Konva's hit graph
  and `toDataURL` automatically; no changes needed.

## Testing

Vitest on the pure geometry function:

- polygon is symmetric about the shaft axis;
- direction invariance (rotating input rotates output);
- zero-length input yields an empty polygon;
- short arrows clamp head/width to the length fraction;
- longer arrows use the full `strokeWidth`-derived dimensions.

Manual verification: draw short/long arrows at several stroke widths and
colors, undo/redo them, recolor via the palette, and export — arrow must
survive all of these visually unchanged.

## Verification

`npm run build`, `npm test`, `cd src-tauri && cargo test` all pass; manual
check in `npm run tauri dev`.
