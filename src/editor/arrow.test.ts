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
