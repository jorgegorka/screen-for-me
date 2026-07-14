import { describe, expect, it } from "vitest";
import { normalizeRect } from "./geometry";

describe("normalizeRect", () => {
  it("orders coordinates regardless of drag direction", () => {
    expect(normalizeRect(100, 80, 20, 200)).toEqual({
      x: 20,
      y: 80,
      width: 80,
      height: 120,
    });
    expect(normalizeRect(20, 80, 100, 200)).toEqual({
      x: 20,
      y: 80,
      width: 80,
      height: 120,
    });
  });
});
