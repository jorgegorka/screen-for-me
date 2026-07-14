import { invoke } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

import { el } from "../shared/dom";
import { initI18n } from "../shared/i18n";
import type { Settings } from "../shared/ipc";

/** Slider stop index ↔ overlay size multiplier. */
const SIZE_STEPS = [0.75, 1.0, 1.25, 1.5, 2.0];

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
  };
}

function fillForm(s: Settings) {
  el<HTMLSelectElement>("position").value = s.position;
  el<HTMLInputElement>("move-active").checked = s.move_to_active_screen;
  el<HTMLInputElement>("overlay-size").value = String(sizeToStep(s.overlay_size));
  el<HTMLInputElement>("auto-close").checked = s.auto_close_enabled;
  el<HTMLSelectElement>("auto-action").value = s.auto_close_action;
  el<HTMLSelectElement>("auto-interval").value = String(s.auto_close_seconds);
  el<HTMLInputElement>("close-after-drag").checked = s.close_after_drag;
  el<HTMLSelectElement>("language").value = s.language;
  syncAutoCloseState();
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

window.addEventListener("DOMContentLoaded", async () => {
  await initI18n();
  void initAutostart();
  fillForm(await invoke<Settings>("get_settings"));
  document
    .querySelectorAll<HTMLElement>("select, input:not(#launch-on-start)")
    .forEach((control) => {
      control.addEventListener("change", async () => {
        syncAutoCloseState();
        fillForm(await invoke<Settings>("set_settings", { settings: readForm() }));
      });
    });
});
