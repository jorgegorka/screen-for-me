import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { savePngAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { CaptureEntry } from "../shared/ipc";

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function actionButton(key: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  // data-i18n so a live language switch re-labels these via applyTranslations.
  button.setAttribute("data-i18n", key);
  button.textContent = t(key);
  button.onclick = onClick;
  return button;
}

function card(entry: CaptureEntry): HTMLElement {
  const node = document.createElement("div");
  node.className = "card";

  const thumb = document.createElement("div");
  thumb.className = "thumb";
  const img = document.createElement("img");
  img.src = `${convertFileSrc(entry.path)}?t=${entry.created_ms}`;
  img.alt = entry.id;
  thumb.appendChild(img);

  const meta = document.createElement("div");
  meta.className = "meta";
  meta.textContent = formatTime(entry.created_ms);

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(
    actionButton("history.annotate", async () => {
      await invoke("open_editor", { id: entry.id });
      await getCurrentWindow().hide();
    }),
    actionButton("history.copy", () => void invoke("copy_capture", { id: entry.id })),
    actionButton("history.save", async () => {
      const dest = await savePngAs(entry.id);
      if (dest) await invoke("save_capture_to", { id: entry.id, dest });
    }),
    actionButton("history.reveal", () => void invoke("reveal_capture", { id: entry.id })),
  );

  node.append(thumb, meta, actions);
  return node;
}

async function render() {
  const captures = await invoke<CaptureEntry[]>("list_captures");
  const grid = el<HTMLDivElement>("grid");
  grid.replaceChildren(...captures.map(card));
  el<HTMLParagraphElement>("empty").classList.toggle("hidden", captures.length > 0);
}

window.addEventListener("DOMContentLoaded", async () => {
  await initI18n();
  void render();
  void listen("capture:new", () => void render());
});
