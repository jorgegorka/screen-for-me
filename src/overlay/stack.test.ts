import { describe, expect, it } from "vitest";

import type { CaptureEntry } from "../shared/ipc";
import { pushTop, removeEntry, trimStack } from "./stack";

const entry = (id: string): CaptureEntry => ({
  id,
  path: `/captures/${id}`,
  created_ms: 0,
});

describe("pushTop", () => {
  it("puts a new entry at the top", () => {
    const stack = pushTop([entry("a.png")], entry("b.png"));
    expect(stack.map((e) => e.id)).toEqual(["b.png", "a.png"]);
  });

  it("moves an existing entry to the top instead of duplicating", () => {
    const stack = [entry("a.png"), entry("b.png"), entry("c.png")];
    const next = pushTop(stack, entry("b.png"));
    expect(next.map((e) => e.id)).toEqual(["b.png", "a.png", "c.png"]);
  });

  it("does not mutate the input", () => {
    const stack = [entry("a.png")];
    pushTop(stack, entry("b.png"));
    expect(stack.map((e) => e.id)).toEqual(["a.png"]);
  });
});

describe("removeEntry", () => {
  it("removes only the matching panel", () => {
    const stack = [entry("a.png"), entry("b.png")];
    expect(removeEntry(stack, "a.png").map((e) => e.id)).toEqual(["b.png"]);
  });

  it("is a no-op for an unknown id", () => {
    const stack = [entry("a.png")];
    expect(removeEntry(stack, "x.png").map((e) => e.id)).toEqual(["a.png"]);
  });
});

describe("trimStack", () => {
  it("keeps the newest entries (top of the stack)", () => {
    const stack = [entry("c.png"), entry("b.png"), entry("a.png")];
    expect(trimStack(stack, 2).map((e) => e.id)).toEqual(["c.png", "b.png"]);
  });

  it("returns the stack unchanged when it already fits", () => {
    const stack = [entry("a.png")];
    expect(trimStack(stack, 5).map((e) => e.id)).toEqual(["a.png"]);
  });
});
