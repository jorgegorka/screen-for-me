import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import iconUrl from "../../src-tauri/icons/128x128@2x.png";
import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { Settings } from "../shared/ipc";
import {
  formatAccelerator,
  isMacosScreenshotAccel,
  type Platform,
  type ShortcutAction,
} from "../shared/accelerator";

const PLATFORM: Platform = /mac/i.test(navigator.platform) ? "mac" : "other";
const ACTIONS: ShortcutAction[] = ["area", "window", "fullscreen"];

let current: Settings | null = null;

function renderShortcuts() {
  if (!current) return;
  for (const action of ACTIONS) {
    el<HTMLElement>(`accel-${action}`).textContent = formatAccelerator(
      current[`shortcut_${action}` as const] as string,
      PLATFORM,
    );
  }
}

type MacosStatus = "hidden" | "owns" | "success" | "failed";

function showStatus(status: MacosStatus) {
  const line = el<HTMLParagraphElement>("macos-status");
  line.hidden = status === "hidden";
  line.classList.remove("warn", "ok", "error");
  if (status === "owns") {
    line.classList.add("warn");
    line.textContent = t("welcome.system_owns");
  } else if (status === "success") {
    line.classList.add("ok");
    line.textContent = t("welcome.assign_success");
  } else if (status === "failed") {
    line.classList.add("error");
    line.textContent = t("welcome.assign_failed");
  }
}

/** Reflect the live system state: warn while macOS still owns ⌘⇧3/4/5, show
 * the success line once they're freed AND assigned here. Re-run on focus so
 * returning from System Settings updates the card. */
async function refreshMacosStatus() {
  const owns = await invoke<boolean>("macos_screenshot_hotkeys_enabled");
  if (owns) {
    showStatus("owns");
    return;
  }
  const assigned =
    current !== null &&
    ACTIONS.every((action) =>
      isMacosScreenshotAccel(current![`shortcut_${action}` as const] as string),
    );
  showStatus(assigned ? "success" : "hidden");
}

function initMacosCard() {
  if (PLATFORM !== "mac") return;
  el<HTMLElement>("macos-card").hidden = false;

  el<HTMLButtonElement>("open-system-settings").addEventListener("click", () => {
    void invoke("open_system_shortcut_settings").catch((err) => {
      console.error("failed to open System Settings", err);
    });
  });

  el<HTMLButtonElement>("assign-macos").addEventListener("click", async () => {
    try {
      current = await invoke<Settings>("apply_macos_screenshot_shortcuts");
      renderShortcuts();
      await refreshMacosStatus();
    } catch (err) {
      console.error("failed to assign macOS shortcuts", err);
      showStatus("failed");
    }
  });

  window.addEventListener("focus", () => void refreshMacosStatus());
  void refreshMacosStatus();
}

window.addEventListener("DOMContentLoaded", async () => {
  await initI18n();
  el<HTMLImageElement>("welcome-icon").src = iconUrl;
  current = await invoke<Settings>("get_settings");
  renderShortcuts();
  initMacosCard();
  // Shortcuts can change while this window is open (Settings tab, the assign
  // button); initI18n already re-translates, this keeps the combos fresh.
  void listen<Settings>("settings:changed", (event) => {
    current = event.payload;
    renderShortcuts();
  });
});
