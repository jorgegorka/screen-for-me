import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

import iconUrl from "../../src-tauri/icons/128x128@2x.png";
import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { Settings } from "../shared/ipc";
import {
  comboToAccelerator,
  formatAccelerator,
  hasRequiredModifier,
  isMacosScreenshotAccel,
  DEFAULT_ACCELS,
  type ComboModifiers,
  type Platform,
  type ShortcutAction,
} from "../shared/accelerator";

/** Slider stop index ↔ overlay size multiplier. */
const SIZE_STEPS = [0.75, 1.0, 1.25, 1.5, 2.0];

const PLATFORM: Platform = /mac/i.test(navigator.platform) ? "mac" : "other";
const ACTIONS: ShortcutAction[] = ["area", "window", "fullscreen"];

/** Last settings received from the backend; `readForm` echoes fields (like
 * the shortcuts) that the General form doesn't own so `set_settings` can't
 * clobber them. */
let current: Settings | null = null;

function sizeToStep(size: number): number {
  let best = 1;
  for (let i = 0; i < SIZE_STEPS.length; i++) {
    if (Math.abs(SIZE_STEPS[i] - size) < Math.abs(SIZE_STEPS[best] - size)) best = i;
  }
  return best;
}

function readForm(): Settings {
  return {
    position: el<HTMLSelectElement>("position").value as Settings["position"],
    move_to_active_screen: el<HTMLInputElement>("move-active").checked,
    overlay_size: SIZE_STEPS[Number(el<HTMLInputElement>("overlay-size").value)],
    auto_close_enabled: el<HTMLInputElement>("auto-close").checked,
    auto_close_action: el<HTMLSelectElement>("auto-action").value as Settings["auto_close_action"],
    auto_close_seconds: Number(el<HTMLSelectElement>("auto-interval").value),
    close_after_drag: el<HTMLInputElement>("close-after-drag").checked,
    language: el<HTMLSelectElement>("language").value as Settings["language"],
    shortcut_area: current?.shortcut_area ?? DEFAULT_ACCELS.area,
    shortcut_window: current?.shortcut_window ?? DEFAULT_ACCELS.window,
    shortcut_fullscreen: current?.shortcut_fullscreen ?? DEFAULT_ACCELS.fullscreen,
  };
}

function fillForm(s: Settings) {
  current = s;
  el<HTMLSelectElement>("position").value = s.position;
  el<HTMLInputElement>("move-active").checked = s.move_to_active_screen;
  el<HTMLInputElement>("overlay-size").value = String(sizeToStep(s.overlay_size));
  el<HTMLInputElement>("auto-close").checked = s.auto_close_enabled;
  el<HTMLSelectElement>("auto-action").value = s.auto_close_action;
  el<HTMLSelectElement>("auto-interval").value = String(s.auto_close_seconds);
  el<HTMLInputElement>("close-after-drag").checked = s.close_after_drag;
  el<HTMLSelectElement>("language").value = s.language;
  syncAutoCloseState();
  renderShortcuts();
  void refreshSystemOwnsHint();
}

function syncAutoCloseState() {
  const enabled = el<HTMLInputElement>("auto-close").checked;
  for (const sub of document.querySelectorAll<HTMLElement>(".sub")) {
    sub.classList.toggle("disabled", !enabled);
  }
}

/** Launch-on-start reflects the real OS login-item state, not settings.json. */
async function initAutostart() {
  const box = el<HTMLInputElement>("launch-on-start");
  try {
    box.checked = await isEnabled();
  } catch (err) {
    console.error("autostart state unavailable", err);
    box.disabled = true;
    return;
  }
  box.addEventListener("change", async () => {
    try {
      if (box.checked) await enable();
      else await disable();
    } catch (err) {
      console.error("failed to toggle autostart", err);
      box.checked = !box.checked;
    }
  });
}

function initTabs() {
  const tabs = Array.from(document.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
  const select = (tab: HTMLButtonElement) => {
    for (const other of tabs) {
      const selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      const panel = document.getElementById(other.getAttribute("aria-controls")!);
      if (panel) panel.hidden = !selected;
    }
  };
  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => select(tab));
    tab.addEventListener("keydown", (event) => {
      const last = tabs.length - 1;
      let target: number | null = null;
      if (event.key === "ArrowRight") target = index === last ? 0 : index + 1;
      else if (event.key === "ArrowLeft") target = index === 0 ? last : index - 1;
      else if (event.key === "Home") target = 0;
      else if (event.key === "End") target = last;
      if (target === null) return;
      event.preventDefault();
      tabs[target].focus();
      select(tabs[target]);
    });
  });
}

const accelOf = (s: Settings, action: ShortcutAction) =>
  s[`shortcut_${action}` as const] as string;

function renderShortcuts() {
  if (!current) return;
  for (const action of ACTIONS) {
    if (action === recording) continue;
    el<HTMLButtonElement>(`shortcut-${action}`).textContent = formatAccelerator(
      accelOf(current, action),
      PLATFORM,
    );
  }
}

let recording: ShortcutAction | null = null;

/** End the active recording (if any) and restore that field's label. */
function stopRecording() {
  if (!recording) return;
  const field = el<HTMLButtonElement>(`shortcut-${recording}`);
  recording = null;
  field.classList.remove("recording");
  renderShortcuts();
}

function showShortcutError(action: ShortcutAction, message: string) {
  const line = el<HTMLParagraphElement>(`shortcut-${action}-error`);
  line.textContent = message;
  line.hidden = false;
}

function clearShortcutErrors() {
  for (const action of ACTIONS) {
    el<HTMLParagraphElement>(`shortcut-${action}-error`).hidden = true;
  }
}

async function applyShortcut(action: ShortcutAction, accelerator: string) {
  try {
    fillForm(await invoke<Settings>("set_shortcut", { action, accelerator }));
    clearShortcutErrors();
    // Registering ⌘⇧3/4/5 succeeds even while macOS still handles them (the
    // keypress never reaches the app), so warn inline when that's the case.
    if (
      PLATFORM === "mac" &&
      isMacosScreenshotAccel(accelerator) &&
      (await invoke<boolean>("macos_screenshot_hotkeys_enabled"))
    ) {
      showShortcutError(action, t("settings.shortcut_warning_system"));
    }
  } catch (err) {
    showShortcutError(action, String(err));
    renderShortcuts();
  }
}

/** Section-level hint: any bound combo is a macOS screenshot shortcut that
 * the system still owns, so it can't fire here yet. */
async function refreshSystemOwnsHint() {
  if (PLATFORM !== "mac" || !current) return;
  const bound = ACTIONS.some((action) => isMacosScreenshotAccel(accelOf(current!, action)));
  const owns = bound && (await invoke<boolean>("macos_screenshot_hotkeys_enabled"));
  el<HTMLParagraphElement>("system-owns-hint").hidden = !owns;
}

function initSystemShortcutsHelp() {
  if (PLATFORM !== "mac") return;
  el<HTMLElement>("system-shortcuts").hidden = false;
  el<HTMLButtonElement>("open-system-shortcuts").addEventListener("click", () => {
    void invoke("open_system_shortcut_settings").catch((err) => {
      console.error("failed to open System Settings", err);
    });
  });
}

function initShortcuts() {
  // WKWebView doesn't reliably focus <button>s on click, so a blur listener
  // alone never fires and a field could stay in "Press shortcut…" forever:
  // any press outside the active field ends the recording (mousedown runs
  // before another field's click handler starts its own).
  document.addEventListener("mousedown", (event) => {
    if (!recording) return;
    const field = el<HTMLButtonElement>(`shortcut-${recording}`);
    if (event.target instanceof Node && !field.contains(event.target)) stopRecording();
  });

  for (const action of ACTIONS) {
    const field = el<HTMLButtonElement>(`shortcut-${action}`);

    field.addEventListener("click", () => {
      if (recording === action) return;
      stopRecording();
      recording = action;
      clearShortcutErrors();
      field.classList.add("recording");
      field.textContent = t("settings.shortcut_press");
      // Deterministic keyboard capture + a real blur when focus moves on.
      field.focus();
    });

    field.addEventListener("blur", () => {
      if (recording === action) stopRecording();
    });

    field.addEventListener("keydown", (event) => {
      if (recording !== action) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape" || (event.key === "Backspace" && !hasAnyModifier(event))) {
        stopRecording();
        return;
      }
      const mods: ComboModifiers = {
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
      };
      const accelerator = comboToAccelerator(mods, event.code);
      if (!accelerator) {
        // A lone modifier press: show a live preview of what's held so far.
        if (isModifierCode(event.code)) previewModifiers(field, mods);
        return;
      }
      if (!hasRequiredModifier(mods)) {
        stopRecording();
        showShortcutError(action, t("settings.shortcut_error_modifier"));
        return;
      }
      stopRecording();
      void applyShortcut(action, accelerator);
    });

    field.addEventListener("keyup", (event) => {
      if (recording !== action || !isModifierCode(event.code)) return;
      previewModifiers(field, {
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        meta: event.metaKey,
      });
    });

    el<HTMLButtonElement>(`shortcut-${action}-reset`).addEventListener("click", () => {
      clearShortcutErrors();
      void applyShortcut(action, DEFAULT_ACCELS[action]);
    });
  }
}

const hasAnyModifier = (e: KeyboardEvent) => e.ctrlKey || e.altKey || e.shiftKey || e.metaKey;

const isModifierCode = (code: string) =>
  /^(Control|Alt|Shift|Meta)(Left|Right)$/.test(code);

/** Live preview of held modifiers while recording (no main key yet): format a
 * placeholder combo, then trim the placeholder key off the label. */
function previewModifiers(field: HTMLButtonElement, mods: ComboModifiers) {
  if (!(mods.ctrl || mods.alt || mods.shift || mods.meta)) {
    field.textContent = t("settings.shortcut_press");
    return;
  }
  const label = formatAccelerator(comboToAccelerator(mods, "Digit1")!, PLATFORM);
  field.textContent = PLATFORM === "mac" ? label.slice(0, -1) : label.replace(/\+?1$/, "+");
}

async function initAbout() {
  el<HTMLImageElement>("about-icon").src = iconUrl;
  try {
    el<HTMLParagraphElement>("about-version").textContent = t("about.version", {
      version: await getVersion(),
    });
  } catch (err) {
    console.error("app version unavailable", err);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await initI18n();
  initTabs();
  initShortcuts();
  initSystemShortcutsHelp();
  void initAutostart();
  void initAbout();
  fillForm(await invoke<Settings>("get_settings"));
  document
    .querySelectorAll<HTMLElement>("#panel-general select, #panel-general input:not(#launch-on-start)")
    .forEach((control) => {
      control.addEventListener("change", async () => {
        syncAutoCloseState();
        fillForm(await invoke<Settings>("set_settings", { settings: readForm() }));
      });
    });
});
