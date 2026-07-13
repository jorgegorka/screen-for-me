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
});
