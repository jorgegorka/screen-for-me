# Tapered Arrow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the editor's constant-width `Konva.Arrow` with a solid Skitch-style tapered arrow (pointed tail → widening shaft → triangular head) whose dimensions scale with arrow length.

**Architecture:** A pure, Konva-free geometry module (`src/editor/arrow.ts`) computes the 7-vertex filled polygon and its bounding box. A factory in `src/editor/shapes.ts` wraps it in a custom `Konva.Shape` (`sceneFunc` fills the polygon, `getSelfRect` overridden so the selection transformer gets a real bounding box). Both the live-draw path (`main.ts:startDraft`) and undo rehydration (`shapes.ts:specToNode`) use the factory. Serialized attrs: `points` (two endpoints), `fill`, `strokeWidth`.

**Tech Stack:** TypeScript, Konva, vitest.

## Global Constraints

- Shape `name` stays `"arrow"` (tool ids, undo whitelist, and color-apply all key off it).
- Serialized attrs for arrow: `[...COMMON, "points", "fill", "strokeWidth"]` — no `stroke`, `pointerLength`, `pointerWidth`.
- `src/editor/arrow.ts` must not import Konva (vitest runs without a DOM).
- No decorative motion (DESIGN.md): the "grow" effect comes purely from length-clamped geometry during drag.
- Before calling the change done: `npm run build`, `npm test`, `cd src-tauri && cargo test` all pass.

---

### Task 1: Pure geometry module (`arrow.ts`)

**Files:**
- Create: `src/editor/arrow.ts`
- Test: `src/editor/arrow.test.ts`

**Interfaces:**
- Produces: `taperedArrowPoints(x1: number, y1: number, x2: number, y2: number, strokeWidth: number): number[]` — flat `[x,y,...]` list of 7 polygon vertices in order tail+ / shaftBase+ / headBase+ / tip / headBase− / shaftBase− / tail−; empty array for zero length.
- Produces: `polygonBounds(points: number[]): { x: number; y: number; width: number; height: number }` — axis-aligned bounding box; zeros for an empty polygon.

- [ ] **Step 1: Write the failing test**

Create `src/editor/arrow.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import { polygonBounds, taperedArrowPoints } from "./arrow";

describe("taperedArrowPoints", () => {
  it("returns an empty polygon for zero length", () => {
    expect(taperedArrowPoints(5, 5, 5, 5, 4)).toEqual([]);
  });

  it("places the tip exactly at the end point", () => {
    const poly = taperedArrowPoints(0, 0, 100, 0, 4);
    expect(poly).toHaveLength(14);
    expect(poly[6]).toBeCloseTo(100);
    expect(poly[7]).toBeCloseTo(0);
  });

  it("is symmetric about the shaft axis", () => {
    // Horizontal arrow along y=0: mirrored vertices have opposite y.
    const poly = taperedArrowPoints(0, 0, 100, 0, 4);
    expect(poly[1]).toBeCloseTo(-poly[13]); // tail
    expect(poly[3]).toBeCloseTo(-poly[11]); // shaft at head base
    expect(poly[5]).toBeCloseTo(-poly[9]); // head base
    expect(poly[7]).toBeCloseTo(0); // tip on axis
  });

  it("widens from tail to head", () => {
    const poly = taperedArrowPoints(0, 0, 100, 0, 4);
    const tailHalf = Math.abs(poly[1]);
    const shaftHalf = Math.abs(poly[3]);
    const headHalf = Math.abs(poly[5]);
    expect(tailHalf).toBeLessThan(shaftHalf);
    expect(shaftHalf).toBeLessThan(headHalf);
  });

  it("uses full stroke-derived dimensions on long arrows", () => {
    // strokeWidth 4: headLen = 6 + 4 * 2.5 = 16, so head base sits at x = 200 - 16.
    const poly = taperedArrowPoints(0, 0, 200, 0, 4);
    expect(poly[4]).toBeCloseTo(184);
    expect(Math.abs(poly[5])).toBeCloseTo(8); // headHalf = 3 + 4 * 1.25
  });

  it("clamps head and width on short arrows", () => {
    // Length 10, strokeWidth 8: headLen clamps to 10 * 0.4 = 4, headHalf to 10 * 0.2 = 2.
    const poly = taperedArrowPoints(0, 0, 10, 0, 8);
    expect(poly[4]).toBeCloseTo(6); // head base x
    expect(Math.abs(poly[5])).toBeCloseTo(2);
  });

  it("is direction invariant", () => {
    // Same arrow drawn right-to-left mirrors the x geometry.
    const right = taperedArrowPoints(0, 0, 100, 0, 4);
    const left = taperedArrowPoints(100, 0, 0, 0, 4);
    expect(left[6]).toBeCloseTo(0); // tip at end point
    expect(Math.abs(left[5])).toBeCloseTo(Math.abs(right[5])); // same head size
  });
});

describe("polygonBounds", () => {
  it("returns zeros for an empty polygon", () => {
    expect(polygonBounds([])).toEqual({ x: 0, y: 0, width: 0, height: 0 });
  });

  it("returns the axis-aligned bounding box", () => {
    const poly = taperedArrowPoints(10, 20, 110, 20, 4);
    const box = polygonBounds(poly);
    expect(box.x).toBeCloseTo(10);
    expect(box.x + box.width).toBeCloseTo(110);
    expect(box.y).toBeCloseTo(20 - 8);
    expect(box.height).toBeCloseTo(16);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/editor/arrow.test.ts`
Expected: FAIL — cannot resolve `./arrow`.

- [ ] **Step 3: Write the implementation**

Create `src/editor/arrow.ts`:

```ts
/**
 * Tapered (Skitch-style) arrow geometry. Konva-free so vitest can run it.
 *
 * The polygon: pointed tail -> shaft widening toward the head -> triangular
 * head. All dimensions derive from strokeWidth but are clamped by a fraction
 * of the arrow's length, so short arrows stay proportionate (the arrow
 * visually "grows" out of the tail while dragging).
 */

/**
 * Filled polygon for an arrow from (x1,y1) to (x2,y2), as a flat [x,y,...]
 * list of 7 vertices: tail+, shaftBase+, headBase+, tip, headBase-,
 * shaftBase-, tail-. Empty for a zero-length arrow.
 */
export function taperedArrowPoints(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  strokeWidth: number,
): number[] {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len === 0) return [];
  const ux = dx / len;
  const uy = dy / len;
  const px = -uy;
  const py = ux;

  const headLen = Math.min(6 + strokeWidth * 2.5, len * 0.4);
  const headHalf = Math.min(3 + strokeWidth * 1.25, len * 0.2);
  const shaftHalf = Math.min(strokeWidth * 0.6, headHalf * 0.6);
  const tailHalf = shaftHalf * 0.15;

  const bx = x2 - ux * headLen;
  const by = y2 - uy * headLen;

  return [
    x1 + px * tailHalf, y1 + py * tailHalf,
    bx + px * shaftHalf, by + py * shaftHalf,
    bx + px * headHalf, by + py * headHalf,
    x2, y2,
    bx - px * headHalf, by - py * headHalf,
    bx - px * shaftHalf, by - py * shaftHalf,
    x1 - px * tailHalf, y1 - py * tailHalf,
  ];
}

/** Axis-aligned bounding box of a flat [x,y,...] polygon; zeros if empty. */
export function polygonBounds(points: number[]): {
  x: number;
  y: number;
  width: number;
  height: number;
} {
  if (points.length < 2) return { x: 0, y: 0, width: 0, height: 0 };
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (let i = 0; i < points.length; i += 2) {
    minX = Math.min(minX, points[i]);
    maxX = Math.max(maxX, points[i]);
    minY = Math.min(minY, points[i + 1]);
    maxY = Math.max(maxY, points[i + 1]);
  }
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npx vitest run src/editor/arrow.test.ts`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add src/editor/arrow.ts src/editor/arrow.test.ts
git commit -m "Add tapered-arrow polygon geometry (Konva-free)"
```

---

### Task 2: Arrow factory + serialization (`shapes.ts`)

**Files:**
- Modify: `src/editor/shapes.ts:25` (ATTRS), `src/editor/shapes.ts:96-97` (specToNode), add factory after `buildCounter`

**Interfaces:**
- Consumes: `taperedArrowPoints`, `polygonBounds` from `./arrow` (Task 1).
- Produces: `buildArrow(attrs: Konva.ShapeConfig): Konva.Shape` — custom shape named `"arrow"`; reads `points` ([x1,y1,x2,y2]), `fill`, `strokeWidth` attrs live; overrides `getSelfRect` so the transformer gets a real box. Used by Task 3.

- [ ] **Step 1: Update the ATTRS whitelist**

In `src/editor/shapes.ts`, replace line 25:

```ts
  arrow: [...COMMON, "points", "fill", "strokeWidth"],
```

- [ ] **Step 2: Add the factory**

Add the import at the top of `src/editor/shapes.ts`:

```ts
import { polygonBounds, taperedArrowPoints } from "./arrow";
```

Add after `buildCounter` (after line 88):

```ts
/**
 * Solid tapered arrow (thin tail, wide head). A custom Shape so the taper can
 * be drawn as one filled polygon; Konva.Arrow only supports constant-width
 * shafts. Geometry lives in arrow.ts; `points` holds the two endpoints.
 */
export function buildArrow(attrs: Konva.ShapeConfig): Konva.Shape {
  const shape = new Konva.Shape({
    ...attrs,
    name: "arrow",
    sceneFunc(ctx, node) {
      const pts = node.getAttr("points") as number[] | undefined;
      if (!pts || pts.length < 4) return;
      const poly = taperedArrowPoints(pts[0], pts[1], pts[2], pts[3], node.strokeWidth());
      if (poly.length === 0) return;
      ctx.beginPath();
      ctx.moveTo(poly[0], poly[1]);
      for (let i = 2; i < poly.length; i += 2) ctx.lineTo(poly[i], poly[i + 1]);
      ctx.closePath();
      ctx.fillStrokeShape(node);
    },
  });
  shape.getSelfRect = () => {
    const pts = shape.getAttr("points") as number[] | undefined;
    if (!pts || pts.length < 4) return { x: 0, y: 0, width: 0, height: 0 };
    return polygonBounds(
      taperedArrowPoints(pts[0], pts[1], pts[2], pts[3], shape.strokeWidth()),
    );
  };
  return shape;
}
```

- [ ] **Step 3: Rehydrate through the factory**

Replace the `"arrow"` case in `specToNode` (line 96-97):

```ts
    case "arrow":
      return buildArrow(attrs as Konva.ShapeConfig);
```

- [ ] **Step 4: Verify type-check and tests**

Run: `npm run build && npm test`
Expected: build succeeds, all vitest suites pass.

- [ ] **Step 5: Commit**

```bash
git add src/editor/shapes.ts
git commit -m "Build arrows as custom tapered Shape in undo rehydration"
```

---

### Task 3: Wire the editor draw path (`main.ts`) + changelog

**Files:**
- Modify: `src/editor/main.ts:185-193` (startDraft), `src/editor/main.ts:580-583` (selectColor)
- Modify: `CHANGELOG.md` (Unreleased)

**Interfaces:**
- Consumes: `buildArrow` from `./shapes` (Task 2).

- [ ] **Step 1: Create drafts via the factory**

In `src/editor/main.ts`, add `buildArrow` to the existing `./shapes` import, then replace the `"arrow"` case in `startDraft` (lines 185-193). No `stroke` attr — a stroked outline would fatten the fill:

```ts
    case "arrow":
      return buildArrow({
        name: "arrow",
        points: [pos.x, pos.y, pos.x, pos.y],
        fill: color,
        strokeWidth,
      });
```

- [ ] **Step 2: Recolor arrows via fill only**

In `selectColor` (lines 580-583), arrows have no stroke anymore; setting one would draw an outline. Replace the `else` branch:

```ts
      } else if (node.name() === "arrow") {
        node.setAttr("fill", value);
      } else {
        node.setAttr("stroke", value);
      }
```

- [ ] **Step 3: Changelog entry**

In `CHANGELOG.md` under `## [Unreleased]`, add a `### Changed` section before `### Fixed`:

```markdown
### Changed

- Editor arrows are now solid and tapered (thin tail widening into the
  head, Skitch-style) and scale with their length, so they grow naturally
  out of the tail point while dragging.
```

- [ ] **Step 4: Verify build and tests**

Run: `npm run build && npm test && (cd src-tauri && cargo test)`
Expected: all pass.

- [ ] **Step 5: Manual verification in the running app**

With `npm run tauri dev` running, capture something, open the editor, and check:
- short drag → small proportionate arrow; long drag → full taper ("grows" while dragging);
- several stroke widths and colors;
- select an arrow → transformer box hugs it; recolor via palette works;
- undo/redo an arrow → identical rendering;
- Copy/export → arrow present in output image.

- [ ] **Step 6: Commit**

```bash
git add src/editor/main.ts CHANGELOG.md
git commit -m "Draw editor arrows as tapered Skitch-style shapes"
```
