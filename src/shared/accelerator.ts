import type { Settings } from "./ipc";

export type Platform = "mac" | "other";

export interface ComboModifiers {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
}

type Modifier = keyof ComboModifiers;

export const DEFAULT_ACCELS = {
  area: "CmdOrCtrl+Shift+7",
  window: "CmdOrCtrl+Shift+8",
  fullscreen: "CmdOrCtrl+Shift+9",
  record: "CmdOrCtrl+Shift+0",
} as const;

export type ShortcutAction = keyof typeof DEFAULT_ACCELS;

export const ACTIONS = Object.keys(DEFAULT_ACCELS) as ShortcutAction[];

export const accelOf = (settings: Settings, action: ShortcutAction): string =>
  settings[`shortcut_${action}`];

export const modsOf = (event: {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}): ComboModifiers => ({
  ctrl: event.ctrlKey,
  alt: event.altKey,
  shift: event.shiftKey,
  meta: event.metaKey,
});

export const anyModifier = (mods: ComboModifiers): boolean =>
  mods.ctrl || mods.alt || mods.shift || mods.meta;

export const hasRequiredModifier = (mods: ComboModifiers): boolean =>
  mods.ctrl || mods.alt || mods.meta;

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

export function codeToToken(code: string): string | null {
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  if (ARROWS.has(code)) return code.slice(5);
  if (PUNCTUATION.has(code)) return code;
  return null;
}

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

function normalizeModifier(token: string, platform: Platform): Modifier | null {
  switch (token.toLowerCase()) {
    case "cmdorctrl":
    case "cmdorcontrol":
    case "commandorcontrol":
    case "commandorctrl":
      return platform === "mac" ? "meta" : "ctrl";
    case "cmd":
    case "command":
    case "super":
      return "meta";
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

export function parseAccelerator(
  accel: string,
  platform: Platform,
): { mods: ComboModifiers; key: string } {
  const tokens = accel.split("+").map((token) => token.trim());
  const key = tokens.pop() ?? "";
  const mods: ComboModifiers = { ctrl: false, alt: false, shift: false, meta: false };
  for (const token of tokens) {
    const modifier = normalizeModifier(token, platform);
    if (modifier) mods[modifier] = true;
  }
  return { mods, key };
}

export type MacosScreenshotDigit = "3" | "4" | "5";

const MACOS_SCREENSHOT_DIGITS: readonly MacosScreenshotDigit[] = ["3", "4", "5"];

export const MACOS_SCREENSHOT_KEYS: Partial<Record<ShortcutAction, MacosScreenshotDigit>> = {
  fullscreen: "3",
  area: "4",
  window: "5",
};

export const MACOS_SCREENSHOT_ACTIONS = Object.keys(MACOS_SCREENSHOT_KEYS) as ShortcutAction[];

function isCmdShiftDigit(accel: string, digits: readonly MacosScreenshotDigit[]): boolean {
  const { mods, key } = parseAccelerator(accel, "mac");
  return (
    digits.some((digit) => key === digit || key === `Digit${digit}`) &&
    mods.meta &&
    mods.shift &&
    !mods.ctrl &&
    !mods.alt
  );
}

export function macosScreenshotKeyOf(accel: string): MacosScreenshotDigit | null {
  return MACOS_SCREENSHOT_DIGITS.find((digit) => isCmdShiftDigit(accel, [digit])) ?? null;
}

export function isMacosScreenshotAccelFor(accel: string, action: ShortcutAction): boolean {
  const digit = MACOS_SCREENSHOT_KEYS[action];
  return digit !== undefined && isCmdShiftDigit(accel, [digit]);
}

const MODIFIER_ORDER: readonly Modifier[] = ["ctrl", "alt", "shift", "meta"];

const MODIFIER_LABELS: Record<Platform, Record<Modifier, string>> = {
  mac: { ctrl: "⌃", alt: "⌥", shift: "⇧", meta: "⌘" },
  other: { ctrl: "Ctrl", alt: "Alt", shift: "Shift", meta: "Super" },
};

const MAC_KEY_SYMBOLS: Record<string, string> = {
  Up: "↑",
  Down: "↓",
  Left: "←",
  Right: "→",
};

export function formatModifiers(mods: ComboModifiers, platform: Platform): string {
  return MODIFIER_ORDER.filter((m) => mods[m])
    .map((m) => MODIFIER_LABELS[platform][m])
    .join(platform === "mac" ? "" : "+");
}

export function formatAccelerator(accel: string, platform: Platform): string {
  const { mods, key } = parseAccelerator(accel, platform);
  const keyLabel =
    platform === "mac" ? (MAC_KEY_SYMBOLS[key] ?? key.toUpperCase()) : key.toUpperCase();
  const modifiers = formatModifiers(mods, platform);
  if (platform === "mac") return modifiers + keyLabel;
  return [modifiers, keyLabel].filter(Boolean).join("+");
}
