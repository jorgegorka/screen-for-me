import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save } from "@tauri-apps/plugin-dialog";

interface CaptureEntry {
  path: string;
  id: string;
  created_ms: number;
}

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function formatTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

function actionButton(label: string, onClick: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.textContent = label;
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
    actionButton("Annotate", async () => {
      await invoke("open_editor", { id: entry.id });
      await getCurrentWindow().hide();
    }),
    actionButton("Copy", () => void invoke("copy_capture", { id: entry.id })),
    actionButton("Save", async () => {
      const dest = await save({
        defaultPath: entry.id,
        filters: [{ name: "PNG image", extensions: ["png"] }],
      });
      if (dest) await invoke("save_capture_to", { id: entry.id, dest });
    }),
    actionButton("Finder", () => void invoke("reveal_capture", { id: entry.id })),
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

window.addEventListener("DOMContentLoaded", () => {
  void render();
  void listen("capture:new", () => void render());
});
