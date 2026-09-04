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
const cancelButton = el<HTMLButtonElement>("cancel");
const hint = el<HTMLDivElement>("hint");
const stopButton = el<HTMLButtonElement>("stop");
const elapsed = el<HTMLSpanElement>("elapsed");

type Phase = "armed" | "starting" | "running" | "stopping";
let phase: Phase = "armed";
let microphone = false;
let ticker: number | undefined;
let stopPending = false;

function renderMic() {
  micButton.setAttribute("aria-pressed", String(microphone));
  micButton.classList.toggle("active", microphone);
}

function enterRunning() {
  phase = "running";
  document.body.classList.remove("starting");
  document.body.classList.add("running");
  if (ticker === undefined) {
    const startedAt = Date.now();
    elapsed.textContent = formatElapsed(0);
    ticker = window.setInterval(() => {
      elapsed.textContent = formatElapsed((Date.now() - startedAt) / 1000);
    }, 250);
  }
  if (stopPending) enterStopping();
}

function enterStopping() {
  if (phase === "stopping") return;
  if (phase !== "running") {
    stopPending = true;
    return;
  }
  phase = "stopping";
  document.body.classList.add("stopping");
  stopButton.disabled = true;
  if (ticker !== undefined) {
    window.clearInterval(ticker);
    ticker = undefined;
  }
  elapsed.textContent = t("recorder.stopping");
}

function start() {
  if (phase !== "armed") return;
  phase = "starting";
  document.body.classList.add("starting");
  hint.textContent = t("recorder.starting");
  recordButton.disabled = true;
  cancelButton.disabled = true;
  micButton.disabled = true;
  void invoke("start_recording", { microphone }).catch((err) => {
    phase = "armed";
    document.body.classList.remove("starting");
    recordButton.disabled = false;
    cancelButton.disabled = false;
    micButton.disabled = false;
    hint.textContent = String(err);
  });
}

micButton.addEventListener("click", () => {
  microphone = !microphone;
  renderMic();
});
recordButton.addEventListener("click", start);
cancelButton.addEventListener("click", () => {
  if (phase !== "armed") return;
  void appWindow.close();
});
stopButton.addEventListener("click", () => {
  if (phase !== "running") return;
  void invoke("stop_recording").catch(() => {});
});

document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (phase === "running") {
    void invoke("stop_recording").catch(() => {});
  } else if (phase === "armed") {
    void appWindow.close();
  }
});

void listen("record:running", () => enterRunning());
void listen("record:stopping", () => enterStopping());

void invoke<RecorderPrefs>("recorder_prefs").then((prefs) => {
  microphone = prefs.microphone;
  renderMic();
});

recordButton.focus();

void initI18n();
