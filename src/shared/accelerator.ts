/**
 * Pure accelerator helpers for the Settings shortcuts tab (no DOM/Tauri
 * imports so vitest can load it).
 *
 * Accelerator strings must stay parseable by BOTH Rust parsers — the
 * global-shortcut plugin (registration) and the tray's menu-item accelerator
 * labels — so key tokens are limited to their shared grammar: digits, letters,
 * F1–F12, arrows, and common punctuation. Validation rules mirror
 * src-tauri/src/shortcuts.rs, which is the authority; these exist only for
 * instant feedback while recording.
 */

export type Platform = "mac" | "other";

export interface ComboModifiers {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
}

/** Per-action defaults; keep in sync with `default_accel` in shortcuts.rs. */
export const DEFAULT_ACCELS = {
  area: "CmdOrCtrl+Shift+7",
  window: "CmdOrCtrl+Shift+8",
  fullscreen: "CmdOrCtrl+Shift+9",
} as const;

export type ShortcutAction = keyof typeof DEFAULT_ACCELS;

/** Punctuation codes whose KeyboardEvent.code name both Rust parsers accept. */
const PUNCTUATION = new Set([
  "Comma",
  "Period",
  "Slash",
  "Semicolon",
  "Quote",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Backquote",
  "Minus",
  "Equal",
]);

const ARROWS = new Set(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"]);

/**
 * Map a KeyboardEvent.code to an accelerator key token, or null when the key
 * can't be (part of) a shortcut — modifiers themselves, media keys, etc.
 */
export function codeToToken(code: string): string | null {
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  if (ARROWS.has(code)) return code.slice(5); // Up/Down/Left/Right
  if (PUNCTUATION.has(code)) return code;
  return null;
}

/** Canonical accelerator string for a recorded combo, or null for bare keys
 * that can't form one. Modifier order is fixed so equal combos compare equal
 * as strings on the frontend too. */
export function comboToAccelerator(mods: ComboModifiers, code: string): string | null {
  const token = codeToToken(code);
  if (!token) return null;
  const parts: string[] = [];
  if (mods.ctrl) parts.push("Ctrl");
  if (mods.alt) parts.push("Alt");
  if (mods.shift) parts.push("Shift");
  if (mods.meta) parts.push("Cmd");
  parts.push(token);
  return parts.join("+");
}

/** Shortcuts need a non-Shift modifier (mirrors shortcuts.rs::validate). */
export function hasRequiredModifier(mods: ComboModifiers): boolean {
  return mods.ctrl || mods.alt || mods.meta;
}

/** Whether a stored accelerator is one of macOS's own screenshot shortcuts
 * (Cmd+Shift+3/4/5). Assigning these is allowed, but while the system still
 * handles them the keypress never reaches the app — callers use this to show
 * a warning, mirroring `is_macos_screenshot_combo` in shortcuts.rs. */
export function isMacosScreenshotAccel(accel: string): boolean {
  const tokens = accel.split("+").map((token) => token.trim());
  const key = tokens.pop() ?? "";
  if (!["3", "4", "5", "Digit3", "Digit4", "Digit5"].includes(key)) return false;
  const present = { ctrl: false, alt: false, shift: false, cmd: false };
  for (const token of tokens) {
    const modifier = normalizeModifier(token, "mac");
    if (modifier) present[modifier as keyof typeof present] = true;
  }
  return present.cmd && present.shift && !present.ctrl && !present.alt;
}

const MAC_MODIFIER_SYMBOLS: Record<string, string> = {
  ctrl: "⌃",
  alt: "⌥",
  shift: "⇧",
  cmd: "⌘",
};

const MAC_KEY_SYMBOLS: Record<string, string> = {
  Up: "↑",
  Down: "↓",
  Left: "←",
  Right: "→",
};

function normalizeModifier(token: string, platform: Platform): string | null {
  switch (token.toLowerCase()) {
    case "cmdorctrl":
    case "cmdorcontrol":
    case "commandorcontrol":
    case "commandorctrl":
      return platform === "mac" ? "cmd" : "ctrl";
    case "cmd":
    case "command":
    case "super":
      return "cmd";
    case "ctrl":
    case "control":
      return "ctrl";
    case "alt":
    case "option":
      return "alt";
    case "shift":
      return "shift";
    default:
      return null;
  }
}

/** Human-readable label for a stored accelerator: symbol runs like ⌃⇧7 on
 * macOS, plus-separated words like Ctrl+Shift+7 elsewhere. */
export function formatAccelerator(accel: string, platform: Platform): string {
  const tokens = accel.split("+").map((token) => token.trim());
  const key = tokens.pop() ?? "";
  const present = { ctrl: false, alt: false, shift: false, cmd: false };
  for (const token of tokens) {
    const modifier = normalizeModifier(token, platform);
    if (modifier) present[modifier as keyof typeof present] = true;
  }
  const keyLabel =
    platform === "mac" ? (MAC_KEY_SYMBOLS[key] ?? key.toUpperCase()) : key.toUpperCase();
  if (platform === "mac") {
    // Standard macOS display order: ⌃ ⌥ ⇧ ⌘.
    const symbols = (["ctrl", "alt", "shift", "cmd"] as const)
      .filter((m) => present[m])
      .map((m) => MAC_MODIFIER_SYMBOLS[m]);
    return [...symbols, keyLabel].join("");
  }
  const words = [
    present.ctrl && "Ctrl",
    present.alt && "Alt",
    present.shift && "Shift",
    present.cmd && "Super",
  ].filter(Boolean) as string[];
  return [...words, keyLabel].join("+");
}
