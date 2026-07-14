import { describe, expect, it } from "vitest";

import { nextCounterNumber } from "./counter";

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
