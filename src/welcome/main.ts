import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import iconUrl from "../../src-tauri/icons/128x128@2x.png";
import { el, PLATFORM } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { Settings } from "../shared/ipc";
import {
  accelOf,
  formatAccelerator,
  isMacosScreenshotAccelFor,
  MACOS_SCREENSHOT_ACTIONS,
  ACTIONS,
} from "../shared/accelerator";

let current: Settings | null = null;

function renderShortcuts() {
  if (!current) return;
  for (const action of ACTIONS) {
    el<HTMLElement>(`accel-${action}`).textContent = formatAccelerator(
      accelOf(current, action),
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

async function refreshMacosStatus() {
  const owned = await invoke<string[]>("macos_screenshot_hotkeys_owned");
  if (owned.length > 0) {
    showStatus("owns");
    return;
  }
  const settings = current;
  const assigned =
    settings !== null &&
    MACOS_SCREENSHOT_ACTIONS.every((action) =>
      isMacosScreenshotAccelFor(accelOf(settings, action), action),
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
  void listen<Settings>("settings:changed", (event) => {
    current = event.payload;
    renderShortcuts();
  });
});
