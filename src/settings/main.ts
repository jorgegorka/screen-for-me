import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  position: "left" | "center" | "right";
  move_to_active_screen: boolean;
  overlay_size: number;
  auto_close_enabled: boolean;
  auto_close_action: "close" | "save_and_close";
  auto_close_seconds: number;
  close_after_drag: boolean;
}

/** Slider stop index ↔ overlay size multiplier. */
const SIZE_STEPS = [0.75, 1.0, 1.25, 1.5, 2.0];

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

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
  syncAutoCloseState();
}

function syncAutoCloseState() {
  const enabled = el<HTMLInputElement>("auto-close").checked;
  for (const sub of document.querySelectorAll<HTMLElement>(".sub")) {
    sub.classList.toggle("disabled", !enabled);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  fillForm(await invoke<Settings>("get_settings"));
  document.querySelectorAll<HTMLElement>("select, input").forEach((control) => {
    control.addEventListener("change", async () => {
      syncAutoCloseState();
      fillForm(await invoke<Settings>("set_settings", { settings: readForm() }));
    });
  });
});
