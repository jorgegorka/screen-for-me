import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hudPosition, isSelectable, normalizeRect, type Rect } from "./geometry";

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const appWindow = getCurrentWindow();
const hint = el<HTMLDivElement>("hint");
// A start-failure overwrites this with an error message; resetToSelect
// restores the original instruction rather than leaving the error stuck.
const hintDefaultText = hint.textContent;
const selection = el<HTMLDivElement>("selection");
const hud = el<HTMLDivElement>("hud");
const progress = el<HTMLSpanElement>("progress");

type Phase = "select" | "staged" | "running";
type Direction = "up" | "down" | "left" | "right";
const DIRECTIONS: readonly string[] = ["up", "down", "left", "right"];
let phase: Phase = "select";
let dragStart: { x: number; y: number } | null = null;
let rect: Rect | null = null;
let direction: Direction = "down";

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
  hint.textContent = hintDefaultText;
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
    const d = button.dataset.direction;
    direction = d && DIRECTIONS.includes(d) ? (d as Direction) : "down";
    hud.querySelectorAll(".directions button").forEach((b) => b.classList.remove("active"));
    button.classList.add("active");
  });
}

// Pill-only mode: Rust shrinks this window to the pill. Idempotent — used
// both for the optimistic switch on Start and when Rust confirms via
// `scroll:running`.
function enterRunning() {
  phase = "running";
  selection.classList.add("hidden");
  hud.classList.add("hidden");
  hint.classList.add("hidden");
  document.body.classList.add("running");
}

el<HTMLButtonElement>("start").addEventListener("click", () => {
  if (phase !== "staged" || !rect) return;
  enterRunning();
  void invoke("run_scrolling_capture", { rect, direction }).catch((err) => {
    // Roll back to the staged state so the user can retry or cancel.
    phase = "staged";
    document.body.classList.remove("running");
    if (rect) {
      renderSelection(rect);
      showHud(rect);
    }
    hint.textContent = String(err);
    hint.classList.remove("hidden");
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

// Rust is the authority on when the run actually starts.
void listen("scroll:running", () => {
  enterRunning();
});

void listen<number>("scroll:progress", (event) => {
  progress.textContent = `${event.payload} frames`;
});
