import { describe, expect, it } from "vitest";

import {
  codeToToken,
  comboToAccelerator,
  formatAccelerator,
  hasRequiredModifier,
  isMacosScreenshotAccel,
  isMacosScreenshotAccelFor,
  macosScreenshotKeyOf,
  DEFAULT_ACCELS,
  MACOS_SCREENSHOT_KEYS,
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

describe("isMacosScreenshotAccel", () => {
  it("flags Cmd+Shift+3/4/5 in any spelling", () => {
    expect(isMacosScreenshotAccel("Cmd+Shift+3")).toBe(true);
    expect(isMacosScreenshotAccel("Shift+Cmd+4")).toBe(true);
    expect(isMacosScreenshotAccel("CmdOrCtrl+Shift+5")).toBe(true);
    expect(isMacosScreenshotAccel("Command+Shift+Digit3")).toBe(true);
  });

  it("ignores other combos", () => {
    expect(isMacosScreenshotAccel("Cmd+Shift+7")).toBe(false);
    expect(isMacosScreenshotAccel("Cmd+3")).toBe(false);
    expect(isMacosScreenshotAccel("Ctrl+Shift+3")).toBe(false);
    expect(isMacosScreenshotAccel("Cmd+Alt+Shift+3")).toBe(false);
  });
});

describe("isMacosScreenshotAccelFor", () => {
  it("accepts the expected combo per action (TARGETS in onboarding.rs)", () => {
    expect(isMacosScreenshotAccelFor("Cmd+Shift+3", "fullscreen")).toBe(true);
    expect(isMacosScreenshotAccelFor("Cmd+Shift+4", "area")).toBe(true);
    expect(isMacosScreenshotAccelFor("Cmd+Shift+5", "window")).toBe(true);
  });

  it("rejects a permuted assignment", () => {
    expect(isMacosScreenshotAccelFor("Cmd+Shift+4", "fullscreen")).toBe(false);
    expect(isMacosScreenshotAccelFor("Cmd+Shift+3", "area")).toBe(false);
    expect(isMacosScreenshotAccelFor("Cmd+Shift+3", "window")).toBe(false);
  });

  it("accepts token variants for the right digit only", () => {
    expect(isMacosScreenshotAccelFor("Command+Shift+Digit3", "fullscreen")).toBe(true);
    expect(isMacosScreenshotAccelFor("Shift+Cmd+4", "area")).toBe(true);
    expect(isMacosScreenshotAccelFor("CmdOrCtrl+Shift+5", "window")).toBe(true);
    expect(isMacosScreenshotAccelFor("Command+Shift+Digit4", "fullscreen")).toBe(false);
  });

  it("rejects non-screenshot combos and extra modifiers", () => {
    expect(isMacosScreenshotAccelFor("Cmd+Shift+7", "area")).toBe(false);
    expect(isMacosScreenshotAccelFor("Ctrl+Shift+4", "area")).toBe(false);
    expect(isMacosScreenshotAccelFor("Cmd+Alt+Shift+4", "area")).toBe(false);
  });

  it("covers every action in the expected-mapping constant", () => {
    expect(MACOS_SCREENSHOT_KEYS).toEqual({ fullscreen: "3", area: "4", window: "5" });
  });
});

describe("macosScreenshotKeyOf", () => {
  it("returns the digit of a Cmd+Shift+3/4/5 combo in any spelling", () => {
    expect(macosScreenshotKeyOf("Cmd+Shift+3")).toBe("3");
    expect(macosScreenshotKeyOf("Shift+Cmd+4")).toBe("4");
    expect(macosScreenshotKeyOf("CmdOrCtrl+Shift+5")).toBe("5");
    expect(macosScreenshotKeyOf("Command+Shift+Digit3")).toBe("3");
  });

  it("returns null for non-screenshot combos", () => {
    expect(macosScreenshotKeyOf("Cmd+Shift+7")).toBe(null);
    expect(macosScreenshotKeyOf("Cmd+3")).toBe(null);
    expect(macosScreenshotKeyOf("Ctrl+Shift+4")).toBe(null);
    expect(macosScreenshotKeyOf("Cmd+Alt+Shift+5")).toBe(null);
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
