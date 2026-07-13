import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hudPosition, isSelectable, normalizeRect, type Rect } from "./geometry";

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const appWindow = getCurrentWindow();
const hint = el<HTMLDivElement>("hint");
const selection = el<HTMLDivElement>("selection");
const hud = el<HTMLDivElement>("hud");
const pill = el<HTMLDivElement>("pill");
const progress = el<HTMLSpanElement>("progress");

// Keep pill in scope for CSS selectors
void pill;

type Phase = "select" | "staged" | "running";
let phase: Phase = "select";
let dragStart: { x: number; y: number } | null = null;
let rect: Rect | null = null;
let direction = "down";

function renderSelection(r: Rect) {
  selection.style.left = `${r.x}px`;
  selection.style.top = `${r.y}px`;
  selection.style.width = `${r.width}px`;
  selection.style.height = `${r.height}px`;
  selection.classList.remove("hidden");
}

function showHud(r: Rect) {
  hud.classList.remove("hidden");
  const { x, y } = hudPosition(
    r,
    hud.offsetWidth,
    hud.offsetHeight,
    window.innerWidth,
    window.innerHeight,
  );
  hud.style.left = `${x}px`;
  hud.style.top = `${y}px`;
}

function resetToSelect() {
  phase = "select";
  rect = null;
  selection.classList.add("hidden");
  hud.classList.add("hidden");
  hint.classList.remove("hidden");
}

document.addEventListener("mousedown", (event) => {
  if (phase === "running" || (event.target as HTMLElement).closest(".hud, .pill")) return;
  phase = "select";
  hud.classList.add("hidden");
  hint.classList.add("hidden");
  dragStart = { x: event.clientX, y: event.clientY };
});

document.addEventListener("mousemove", (event) => {
  if (!dragStart) return;
  renderSelection(normalizeRect(dragStart.x, dragStart.y, event.clientX, event.clientY));
});

document.addEventListener("mouseup", (event) => {
  if (!dragStart) return;
  const candidate = normalizeRect(dragStart.x, dragStart.y, event.clientX, event.clientY);
  dragStart = null;
  if (!isSelectable(candidate)) {
    resetToSelect();
    return;
  }
  rect = candidate;
  phase = "staged";
  renderSelection(candidate);
  showHud(candidate);
});

for (const button of hud.querySelectorAll<HTMLButtonElement>(".directions button")) {
  button.addEventListener("click", () => {
    direction = button.dataset.direction ?? "down";
    hud.querySelectorAll(".directions button").forEach((b) => b.classList.remove("active"));
    button.classList.add("active");
  });
}

el<HTMLButtonElement>("start").addEventListener("click", () => {
  if (phase !== "staged" || !rect) return;
  phase = "running";
  // Rust shrinks this window to the pill; swap the page to pill-only mode.
  selection.classList.add("hidden");
  hud.classList.add("hidden");
  document.body.classList.add("running");
  void invoke("run_scrolling_capture", { rect, direction }).catch((err) => {
    progress.textContent = String(err);
  });
});

el<HTMLButtonElement>("cancel").addEventListener("click", () => void appWindow.close());
el<HTMLButtonElement>("stop").addEventListener("click", () => void invoke("stop_scrolling_capture"));

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (phase === "running") {
    void invoke("stop_scrolling_capture");
  } else {
    void appWindow.close();
  }
});

void listen<number>("scroll:progress", (event) => {
  progress.textContent = `${event.payload} frames`;
});
