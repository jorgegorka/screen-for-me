import { describe, expect, it } from "vitest";

import { formatElapsed } from "./format";

describe("formatElapsed", () => {
  it("shows minutes and zero-padded seconds", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(7)).toBe("0:07");
    expect(formatElapsed(754)).toBe("12:34");
  });

  it("adds hours once the recording passes an hour", () => {
    expect(formatElapsed(3600)).toBe("1:00:00");
    expect(formatElapsed(3723)).toBe("1:02:03");
  });

  it("clamps fractional and negative input", () => {
    expect(formatElapsed(59.9)).toBe("0:59");
    expect(formatElapsed(-5)).toBe("0:00");
  });
});
