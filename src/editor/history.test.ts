import { describe, expect, it } from "vitest";
import { UndoStack } from "./history";

describe("UndoStack", () => {
  it("undoes and redoes through committed states", () => {
    const stack = new UndoStack("a");
    stack.commit("b");
    stack.commit("c");
    expect(stack.undo()).toBe("b");
    expect(stack.undo()).toBe("a");
    expect(stack.undo()).toBeNull();
    expect(stack.redo()).toBe("b");
    expect(stack.redo()).toBe("c");
    expect(stack.redo()).toBeNull();
  });

  it("clears the redo branch on a new commit", () => {
    const stack = new UndoStack("a");
    stack.commit("b");
    stack.undo();
    stack.commit("c");
    expect(stack.canRedo).toBe(false);
    expect(stack.undo()).toBe("a");
  });

  it("ignores commits of the identical state", () => {
    const stack = new UndoStack("a");
    stack.commit("a");
    expect(stack.canUndo).toBe(false);
  });

  it("drops the oldest snapshots beyond the depth cap", () => {
    const stack = new UndoStack("s0");
    for (let i = 1; i <= 60; i++) stack.commit(`s${i}`);
    // undo bottoms out after 50 steps: the oldest 10 states were dropped
    let last: string | null = null;
    let steps = 0;
    for (let next = stack.undo(); next !== null; next = stack.undo()) {
      last = next;
      steps++;
    }
    expect(steps).toBe(50);
    expect(last).toBe("s10");
  });
});
