import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { startDrag } from "@crabnebula/tauri-plugin-drag";

import { saveCaptureAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import { applyTranslations, initI18n, t } from "../shared/i18n";
import type { CaptureEntry, Settings } from "../shared/ipc";
import { pushTop, removeEntry, trimStack } from "./stack";

type OverlaySettings = Pick<
  Settings,
  "auto_close_enabled" | "auto_close_action" | "auto_close_seconds" | "close_after_drag"
>;

interface Panel {
  entry: CaptureEntry;
  node: HTMLElement;
  hovering: boolean;
  hideTimer?: number;
}

let stack: CaptureEntry[] = [];
const panels = new Map<string, Panel>();
let settings: OverlaySettings = {
  auto_close_enabled: false,
  auto_close_action: "close",
  auto_close_seconds: 30,
  close_after_drag: true,
};

const appWindow = getCurrentWindow();

function armAutoHide(panel: Panel) {
  window.clearTimeout(panel.hideTimer);
  if (!settings.auto_close_enabled) return;
  panel.hideTimer = window.setTimeout(() => {
    if (panel.hovering) {
      armAutoHide(panel);
      return;
    }
    void (async () => {
      if (settings.auto_close_action === "save_and_close") {
        await invoke("save_capture_to_desktop", { id: panel.entry.id }).catch(() => {});
      }
      removePanel(panel.entry.id);
    })();
  }, settings.auto_close_seconds * 1000);
}

function toast(panel: Panel, message: string) {
  const node = panel.node.querySelector<HTMLDivElement>(".toast")!;
  node.textContent = message;
  node.classList.remove("hidden");
  window.setTimeout(() => node.classList.add("hidden"), 1500);
}

async function run(panel: Panel, action: () => Promise<void>, doneMessage?: string) {
  try {
    await action();
    if (doneMessage) toast(panel, doneMessage);
  } catch (err) {
    toast(panel, String(err));
  }
  armAutoHide(panel);
}

function buildPanel(entry: CaptureEntry): Panel {
  const template = el<HTMLTemplateElement>("panel-template");
  const node = (template.content.cloneNode(true) as DocumentFragment)
    .firstElementChild as HTMLElement;
  const video = entry.kind === "video";
  const panel: Panel = { entry, node, hovering: false };

  node.addEventListener("mouseenter", () => (panel.hovering = true));
  node.addEventListener("mouseleave", () => {
    panel.hovering = false;
    armAutoHide(panel);
  });

  const query = <T extends HTMLElement>(selector: string) =>
    node.querySelector<T>(selector)!;

  const thumb = query<HTMLImageElement>(".thumb");
  const preview = video ? entry.poster : entry.path;
  if (preview) thumb.src = `${convertFileSrc(preview)}?t=${entry.created_ms}`;
  else thumb.classList.add("hidden");
  if (video) thumb.setAttribute("data-i18n-alt", "overlay.latest_recording");
  query<HTMLElement>(".play-badge").classList.toggle("hidden", !video);
  query<HTMLButtonElement>(".copy").classList.toggle("hidden", video);
  query<HTMLButtonElement>(".annotate").classList.toggle("hidden", video);
  query<HTMLButtonElement>(".open").classList.toggle("hidden", !video);
  applyTranslations(node);

  query<HTMLButtonElement>(".dismiss").onclick = () => removePanel(entry.id);

  thumb.addEventListener("mousedown", (down) => {
    const cancel = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", cancel);
    };
    const onMove = (move: MouseEvent) => {
      if (Math.hypot(move.clientX - down.clientX, move.clientY - down.clientY) < 5) return;
      cancel();
      const keepOpen = move.altKey;

      const dragEnded = () => void invoke("set_overlay_drag_active", { active: false });
      void invoke("set_overlay_drag_active", { active: true });
      void startDrag({ item: [entry.path], icon: entry.poster ?? entry.path }, dragEnded)
        .then(() => {
          if (settings.close_after_drag && !keepOpen) removePanel(entry.id);
          else armAutoHide(panel);
        })
        .finally(dragEnded);
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", cancel);
  });

  query<HTMLButtonElement>(".copy").onclick = () =>
    run(panel, () => invoke("copy_capture", { id: entry.id }), t("overlay.toast_copied"));

  query<HTMLButtonElement>(".save").onclick = () =>
    run(panel, async () => {
      const dest = await saveCaptureAs(entry.id, entry.kind);
      if (dest) {
        await invoke("save_capture_to", { id: entry.id, dest });
        toast(panel, t("overlay.toast_saved"));
      }
    });

  query<HTMLButtonElement>(".reveal").onclick = () =>
    run(panel, () => invoke("reveal_capture", { id: entry.id }));

  query<HTMLButtonElement>(".annotate").onclick = () =>
    run(panel, () => invoke("open_editor", { id: entry.id }));

  query<HTMLButtonElement>(".open").onclick = () =>
    run(panel, () => invoke("open_capture", { id: entry.id }));

  return panel;
}

function dropPanel(id: string) {
  const panel = panels.get(id);
  if (!panel) return;
  window.clearTimeout(panel.hideTimer);
  panel.node.remove();
  panels.delete(id);
}

function removePanel(id: string) {
  stack = removeEntry(stack, id);
  void syncStack();
}

let syncing: Promise<void> = Promise.resolve();

function syncStack(): Promise<void> {
  syncing = syncing.then(runSync).catch(() => {});
  return syncing;
}

async function runSync() {
  for (const id of [...panels.keys()]) {
    if (!stack.some((e) => e.id === id)) dropPanel(id);
  }
  if (stack.length === 0) {
    await appWindow.hide();

    await invoke("set_overlay_panels", { count: 1 }).catch(() => {});
    return;
  }
  for (const entry of stack) {
    if (!panels.has(entry.id)) {
      const panel = buildPanel(entry);
      panels.set(entry.id, panel);
      armAutoHide(panel);
    }
  }

  el<HTMLDivElement>("stack").append(...stack.map((e) => panels.get(e.id)!.node));
  const max = await invoke<number>("set_overlay_panels", { count: stack.length }).catch(
    () => stack.length,
  );
  if (stack.length > max) {
    const dropped = stack.slice(max);
    stack = trimStack(stack, max);
    for (const entry of dropped) dropPanel(entry.id);
  }
}

function showCapture(entry: CaptureEntry) {
  stack = pushTop(stack, entry);
  void syncStack();
}

window.addEventListener("DOMContentLoaded", () => {
  void initI18n();

  void listen<CaptureEntry>("capture:new", (event) => showCapture(event.payload));
  void listen<CaptureEntry>("capture:restore", (event) => showCapture(event.payload));

  void invoke<Settings>("get_settings").then((s) => {
    settings = s;
    for (const panel of panels.values()) armAutoHide(panel);
  });
  void listen<Settings>("settings:changed", (event) => {
    settings = event.payload;
    for (const panel of panels.values()) armAutoHide(panel);
  });

  void (async () => {
    if (!(await appWindow.isVisible())) return;
    const captures = await invoke<CaptureEntry[]>("list_captures");
    if (captures.length > 0 && stack.length === 0) showCapture(captures[0]);
  })();
});
