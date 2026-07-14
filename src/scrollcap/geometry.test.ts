import { describe, expect, it } from "vitest";
import { hudPosition, isSelectable } from "./geometry";

describe("isSelectable", () => {
  it("rejects rects under the minimum size", () => {
    expect(isSelectable({ x: 0, y: 0, width: 39, height: 400 })).toBe(false);
    expect(isSelectable({ x: 0, y: 0, width: 400, height: 39 })).toBe(false);
  });
  it("accepts rects at or over the minimum size", () => {
    expect(isSelectable({ x: 0, y: 0, width: 40, height: 40 })).toBe(true);
  });
});

describe("hudPosition", () => {
  const hud = { w: 260, h: 56 };
  it("sits below the rect when there is room", () => {
    const pos = hudPosition({ x: 100, y: 100, width: 300, height: 200 }, hud.w, hud.h, 1440, 900);
    expect(pos).toEqual({ x: 140, y: 312 });
  });
  it("flips above the rect near the bottom edge", () => {
    const pos = hudPosition({ x: 100, y: 600, width: 300, height: 260 }, hud.w, hud.h, 1440, 900);
    expect(pos.y).toBe(600 - 56 - 12);
  });
  it("clamps x inside the viewport", () => {
    const pos = hudPosition({ x: 1300, y: 100, width: 200, height: 100 }, hud.w, hud.h, 1440, 900);
    expect(pos.x).toBe(1440 - 260 - 12);
  });
});
