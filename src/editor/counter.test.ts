import { describe, expect, it } from "vitest";

import { counterTextColor, nextCounterNumber } from "./counter";

describe("nextCounterNumber", () => {
  it("starts at 1 when there are no badges", () => {
    expect(nextCounterNumber([])).toBe(1);
  });

  it("continues past the highest number", () => {
    expect(nextCounterNumber([1, 2, 3])).toBe(4);
  });

  it("does not reuse numbers freed by mid-sequence deletion", () => {
    expect(nextCounterNumber([1, 3])).toBe(4);
  });

  it("handles unordered input", () => {
    expect(nextCounterNumber([3, 1, 2])).toBe(4);
  });
});

describe("counterTextColor", () => {
  it("uses black text on light fills", () => {
    expect(counterTextColor("#ffffff")).toBe("#000000");
    expect(counterTextColor("#ffcc00")).toBe("#000000");
  });

  it("keeps white text on dark and saturated fills", () => {
    expect(counterTextColor("#000000")).toBe("#ffffff");
    expect(counterTextColor("#ff3b30")).toBe("#ffffff");
    expect(counterTextColor("#34c759")).toBe("#ffffff");
    expect(counterTextColor("#4f8ef7")).toBe("#ffffff");
    expect(counterTextColor("#af52de")).toBe("#ffffff");
  });

  it("supports shorthand hex", () => {
    expect(counterTextColor("#fff")).toBe("#000000");
    expect(counterTextColor("#000")).toBe("#ffffff");
  });

  it("falls back to white when the colour is unparseable", () => {
    expect(counterTextColor("tomato")).toBe("#ffffff");
  });
});
