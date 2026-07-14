import { describe, expect, it } from "vitest";

import {
  codeToToken,
  comboToAccelerator,
  formatAccelerator,
  hasRequiredModifier,
  isReservedCombo,
  DEFAULT_ACCELS,
} from "./accelerator";

const mods = (overrides: Partial<Parameters<typeof comboToAccelerator>[0]> = {}) => ({
  ctrl: false,
  alt: false,
  shift: false,
  meta: false,
  ...overrides,
});

describe("codeToToken", () => {
  it("maps digits, letters, F-keys, arrows and punctuation", () => {
    expect(codeToToken("Digit7")).toBe("7");
    expect(codeToToken("KeyA")).toBe("A");
    expect(codeToToken("F5")).toBe("F5");
    expect(codeToToken("F12")).toBe("F12");
    expect(codeToToken("ArrowUp")).toBe("Up");
    expect(codeToToken("Comma")).toBe("Comma");
    expect(codeToToken("BracketLeft")).toBe("BracketLeft");
  });

  it("rejects modifiers and unsupported keys", () => {
    for (const code of ["ShiftLeft", "MetaRight", "ControlLeft", "AltLeft", "Escape", "Tab", "Space", "F13", "MediaPlayPause"]) {
      expect(codeToToken(code), code).toBeNull();
    }
  });
});

describe("comboToAccelerator", () => {
  it("emits modifiers in canonical order", () => {
    expect(comboToAccelerator(mods({ meta: true, shift: true }), "Digit7")).toBe("Shift+Cmd+7");
    expect(comboToAccelerator(mods({ ctrl: true, alt: true }), "KeyA")).toBe("Ctrl+Alt+A");
  });

  it("returns null for unsupported keys", () => {
    expect(comboToAccelerator(mods({ meta: true }), "Escape")).toBeNull();
  });
});

describe("hasRequiredModifier", () => {
  it("requires a non-Shift modifier", () => {
    expect(hasRequiredModifier(mods({ shift: true }))).toBe(false);
    expect(hasRequiredModifier(mods())).toBe(false);
    expect(hasRequiredModifier(mods({ ctrl: true }))).toBe(true);
    expect(hasRequiredModifier(mods({ alt: true }))).toBe(true);
    expect(hasRequiredModifier(mods({ meta: true }))).toBe(true);
  });
});

describe("isReservedCombo", () => {
  it("flags Cmd+Shift+3/4/5 on macOS only", () => {
    const cmdShift = mods({ meta: true, shift: true });
    expect(isReservedCombo(cmdShift, "Digit3", "mac")).toBe(true);
    expect(isReservedCombo(cmdShift, "Digit5", "mac")).toBe(true);
    expect(isReservedCombo(cmdShift, "Digit7", "mac")).toBe(false);
    expect(isReservedCombo(cmdShift, "Digit3", "other")).toBe(false);
    expect(isReservedCombo(mods({ meta: true, shift: true, alt: true }), "Digit3", "mac")).toBe(false);
  });
});

describe("formatAccelerator", () => {
  it("uses macOS symbols in the standard order", () => {
    expect(formatAccelerator("CmdOrCtrl+Shift+7", "mac")).toBe("⇧⌘7");
    expect(formatAccelerator("Ctrl+Alt+Shift+Cmd+A", "mac")).toBe("⌃⌥⇧⌘A");
    expect(formatAccelerator("Cmd+Up", "mac")).toBe("⌘↑");
  });

  it("uses plus-separated words elsewhere", () => {
    expect(formatAccelerator("CmdOrCtrl+Shift+7", "other")).toBe("Ctrl+Shift+7");
    expect(formatAccelerator("Ctrl+Alt+A", "other")).toBe("Ctrl+Alt+A");
    expect(formatAccelerator("Cmd+K", "other")).toBe("Super+K");
  });

  it("formats every default", () => {
    expect(formatAccelerator(DEFAULT_ACCELS.area, "mac")).toBe("⇧⌘7");
    expect(formatAccelerator(DEFAULT_ACCELS.window, "other")).toBe("Ctrl+Shift+8");
    expect(formatAccelerator(DEFAULT_ACCELS.fullscreen, "mac")).toBe("⇧⌘9");
  });
});
