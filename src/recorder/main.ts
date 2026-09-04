import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { RecorderPrefs } from "../shared/ipc";
import { formatElapsed } from "./format";

const appWindow = getCurrentWindow();
const micButton = el<HTMLButtonElement>("mic");
const recordButton = el<HTMLButtonElement>("record");
const hint = el<HTMLDivElement>("hint");
const elapsed = el<HTMLSpanElement>("elapsed");

type Phase = "armed" | "starting" | "running";
let phase: Phase = "armed";
let microphone = false;
let ticker: number | undefined;

function renderMic() {
  micButton.setAttribute("aria-pressed", String(microphone));
  micButton.classList.toggle("active", microphone);
}

function enterRunning() {
  phase = "running";
  document.body.classList.remove("starting");
  document.body.classList.add("running");
  if (ticker !== undefined) return;
  const startedAt = Date.now();
  elapsed.textContent = formatElapsed(0);
  ticker = window.setInterval(() => {
    elapsed.textContent = formatElapsed((Date.now() - startedAt) / 1000);
  }, 250);
}

function start() {
  if (phase !== "armed") return;
  phase = "starting";
  document.body.classList.add("starting");
  hint.textContent = t("recorder.starting");
  recordButton.disabled = true;
  void invoke("start_recording", { microphone }).catch((err) => {
    phase = "armed";
    document.body.classList.remove("starting");
    recordButton.disabled = false;
    hint.textContent = String(err);
  });
}

micButton.addEventListener("click", () => {
  microphone = !microphone;
  renderMic();
});
recordButton.addEventListener("click", start);
el<HTMLButtonElement>("cancel").addEventListener("click", () => void appWindow.close());
el<HTMLButtonElement>("stop").addEventListener("click", () => void invoke("stop_recording"));

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (phase === "running") {
    void invoke("stop_recording");
  } else if (phase === "armed") {
    void appWindow.close();
  }
});

void listen("record:running", () => enterRunning());

void invoke<RecorderPrefs>("recorder_prefs").then((prefs) => {
  microphone = prefs.microphone;
  renderMic();
});

recordButton.focus();

void initI18n();
