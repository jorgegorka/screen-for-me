import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { CaptureEntry } from "../shared/ipc";

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function actionButton(key: string, action: () => Promise<unknown>): HTMLButtonElement {
  const button = document.createElement("button");

  button.setAttribute("data-i18n", key);
  button.textContent = t(key);

  button.onclick = () => void action().then(() => getCurrentWindow().hide());
  return button;
}

function card(entry: CaptureEntry): HTMLElement {
  const node = document.createElement("div");
  node.className = "card";

  const video = entry.kind === "video";
  const thumb = document.createElement("div");
  thumb.className = "thumb";
  const preview = video ? entry.poster : entry.path;
  if (preview) {
    const img = document.createElement("img");
    img.src = `${convertFileSrc(preview)}?t=${entry.created_ms}`;
    img.alt = entry.id;
    thumb.appendChild(img);
  }
  if (video) {
    const badge = document.createElement("span");
    badge.className = "play-badge";
    thumb.appendChild(badge);
  }

  const meta = document.createElement("div");
  meta.className = "meta";
  meta.textContent = formatTime(entry.created_ms);

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.append(
    video
      ? actionButton("history.open", () => invoke("open_capture", { id: entry.id }))
      : actionButton("history.copy", () => invoke("copy_capture", { id: entry.id })),
    actionButton("history.restore", () => invoke("restore_capture", { id: entry.id })),
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
