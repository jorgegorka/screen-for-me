import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { startDrag } from "@crabnebula/tauri-plugin-drag";

import { savePngAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import type { CaptureEntry, Settings } from "../shared/ipc";

/** The overlay only cares about the auto-close/drag subset of Settings. */
type OverlaySettings = Pick<
  Settings,
  "auto_close_enabled" | "auto_close_action" | "auto_close_seconds" | "close_after_drag"
>;

let current: CaptureEntry | null = null;
let hideTimer: number | undefined;
let hovering = false;
let settings: OverlaySettings = {
  auto_close_enabled: false,
  auto_close_action: "close",
  auto_close_seconds: 30,
  close_after_drag: true,
};

const appWindow = getCurrentWindow();

function armAutoHide() {
  window.clearTimeout(hideTimer);
  if (!settings.auto_close_enabled) return;
  hideTimer = window.setTimeout(() => {
    if (hovering) {
      armAutoHide();
      return;
    }
    void (async () => {
      if (settings.auto_close_action === "save_and_close" && current) {
        await invoke("save_capture_to_desktop", { id: current.id }).catch(() => {});
      }
      await appWindow.hide();
    })();
  }, settings.auto_close_seconds * 1000);
}

function toast(message: string) {
  const node = el<HTMLDivElement>("toast");
  node.textContent = message;
  node.classList.remove("hidden");
  window.setTimeout(() => node.classList.add("hidden"), 1500);
}

async function refreshBadge() {
  const captures = await invoke<CaptureEntry[]>("list_captures");
  const badge = el<HTMLSpanElement>("stack-badge");
  if (captures.length > 1) {
    badge.textContent = `+${captures.length - 1}`;
    badge.classList.remove("hidden");
  } else {
    badge.classList.add("hidden");
  }
}

function showCapture(entry: CaptureEntry) {
  current = entry;
  // cache-bust: the same window re-shows different files
  el<HTMLImageElement>("thumb").src =
    `${convertFileSrc(entry.path)}?t=${entry.created_ms}`;
  el<HTMLDivElement>("overlay-card").classList.remove("hidden");
  void refreshBadge();
  armAutoHide();
}

async function run(action: () => Promise<void>, doneMessage?: string) {
  if (!current) return;
  try {
    await action();
    if (doneMessage) toast(doneMessage);
  } catch (err) {
    toast(String(err));
  }
  armAutoHide();
}

window.addEventListener("DOMContentLoaded", () => {
  const card = el<HTMLDivElement>("overlay-card");
  card.addEventListener("mouseenter", () => (hovering = true));
  card.addEventListener("mouseleave", () => {
    hovering = false;
    armAutoHide();
  });

  el<HTMLButtonElement>("dismiss").onclick = () => void appWindow.hide();

  // Drag the capture out to other apps: native drag starts once the pointer
  // moves a few pixels with the button held on the thumbnail.
  const thumb = el<HTMLImageElement>("thumb");
  thumb.addEventListener("mousedown", (down) => {
    if (!current) return;
    const entry = current;
    const cancel = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", cancel);
    };
    const onMove = (move: MouseEvent) => {
      if (Math.hypot(move.clientX - down.clientX, move.clientY - down.clientY) < 5) return;
      cancel();
      const keepOpen = move.altKey; // ⌥ at drag start keeps the overlay
      void startDrag({ item: [entry.path], icon: entry.path }).then(() => {
        if (settings.close_after_drag && !keepOpen) void appWindow.hide();
        else armAutoHide();
      });
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", cancel);
  });

  el<HTMLButtonElement>("copy").onclick = () =>
    run(() => invoke("copy_capture", { id: current!.id }), "Copied");

  el<HTMLButtonElement>("save").onclick = () =>
    run(async () => {
      const dest = await savePngAs(current!.id);
      if (dest) {
        await invoke("save_capture_to", { id: current!.id, dest });
        toast("Saved");
      }
    });

  el<HTMLButtonElement>("reveal").onclick = () =>
    run(() => invoke("reveal_capture", { id: current!.id }));

  el<HTMLButtonElement>("annotate").onclick = () =>
    run(() => invoke("open_editor", { id: current!.id }));

  void listen<CaptureEntry>("capture:new", (event) => {
    showCapture(event.payload);
  });

  void invoke<Settings>("get_settings").then((s) => {
    settings = s;
    armAutoHide();
  });
  void listen<Settings>("settings:changed", (event) => {
    settings = event.payload;
    armAutoHide();
  });

  // If the window was shown before the page finished loading, catch up.
  void invoke<CaptureEntry[]>("list_captures").then((captures) => {
    if (captures.length > 0) showCapture(captures[0]);
  });
});
