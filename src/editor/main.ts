import Konva from "konva";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { savePngAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import { normalizeRect } from "../shared/geometry";
import type { CaptureEntry } from "../shared/ipc";
import { clampRect, fitScale, imageToScreen, type Rect, type Size } from "./geometry";
import { nextCounterNumber } from "./counter";
import { UndoStack } from "./history";
import { pixelateRegion } from "./pixelate";
import { buildCounter, rebuildLayer, serializeLayer, type ShapeType } from "./shapes";

type Tool = ShapeType | "select" | "crop";

const COLORS = ["#ff3b30", "#ffcc00", "#34c759", "#4f8ef7", "#af52de", "#ffffff", "#000000"];

// ---------------------------------------------------------------------------
// State

let captureId = "";
let sourceImage: HTMLImageElement;
let imageSize: Size = { width: 0, height: 0 };
let scale = 1;
let crop: Rect | null = null;
let pendingCrop: Rect | null = null;

let tool: Tool = "select";
let color = COLORS[0];
let strokeWidth = 4;

let stage: Konva.Stage;
let bgLayer: Konva.Layer;
let annLayer: Konva.Layer;
let uiLayer: Konva.Layer;
let transformer: Konva.Transformer;
let undoStack: UndoStack<string>;

/** Node being drawn by the current pointer drag, if any. */
let draft: Konva.Shape | null = null;
let draftStart = { x: 0, y: 0 };
let marquee: Konva.Rect | null = null;

// ---------------------------------------------------------------------------
// Stage setup

function viewportSize(): Size {
  const scroll = el<HTMLDivElement>("canvas-scroll");
  return { width: scroll.clientWidth - 32, height: scroll.clientHeight - 32 };
}

function buildStage() {
  stage = new Konva.Stage({
    container: "stage-holder",
    width: 10,
    height: 10,
  });
  bgLayer = new Konva.Layer({ listening: false });
  annLayer = new Konva.Layer();
  uiLayer = new Konva.Layer();
  stage.add(bgLayer, annLayer, uiLayer);

  transformer = new Konva.Transformer({
    rotateEnabled: true,
    ignoreStroke: true,
    flipEnabled: false,
  });
  uiLayer.add(transformer);

  stage.on("mousedown touchstart", onPointerDown);
  stage.on("mousemove touchmove", onPointerMove);
  stage.on("mouseup touchend", onPointerUp);
  stage.on("click tap", onStageClick);
  stage.on("dblclick dbltap", onStageDblClick);
  stage.on("dragend transformend", () => commit());
}

function applyView() {
  const view = crop ?? { x: 0, y: 0, ...imageSize };
  stage.size({ width: view.width * scale, height: view.height * scale });
  stage.scale({ x: scale, y: scale });
  stage.position({ x: -view.x * scale, y: -view.y * scale });
  el<HTMLButtonElement>("reset-crop").classList.toggle("hidden", crop === null);
  stage.batchDraw();
}

/** Pointer position in image coordinates. */
function pointerPos() {
  const p = stage.getPointerPosition()!;
  return {
    x: (p.x - stage.x()) / scale,
    y: (p.y - stage.y()) / scale,
  };
}

async function loadCapture(entry: CaptureEntry) {
  captureId = entry.id;
  crop = null;
  cancelCrop();
  // Load from raw bytes as a same-origin blob URL — an asset:// image taints
  // the canvas and breaks toDataURL() export.
  const buffer = await invoke<ArrayBuffer>("read_capture_bytes", { id: entry.id });
  const url = URL.createObjectURL(new Blob([buffer], { type: "image/png" }));
  sourceImage = new window.Image();
  sourceImage.src = url;
  await sourceImage.decode();
  URL.revokeObjectURL(url);
  imageSize = { width: sourceImage.naturalWidth, height: sourceImage.naturalHeight };
  scale = fitScale(imageSize, viewportSize());

  bgLayer.destroyChildren();
  bgLayer.add(
    new Konva.Image({ image: sourceImage, ...imageSize, listening: false }),
  );
  annLayer.destroyChildren();
  transformer.nodes([]);
  undoStack = new UndoStack(serializeLayer(annLayer));
  applyView();
}

// ---------------------------------------------------------------------------
// Undo / redo

function commit() {
  undoStack.commit(serializeLayer(annLayer));
}

function restore(snapshot: string) {
  transformer.nodes([]);
  rebuildLayer(annLayer, snapshot, () => annLayer.batchDraw());
  syncDraggable();
  annLayer.batchDraw();
}

function undo() {
  const snapshot = undoStack.undo();
  if (snapshot !== null) restore(snapshot);
}

function redo() {
  const snapshot = undoStack.redo();
  if (snapshot !== null) restore(snapshot);
}

// ---------------------------------------------------------------------------
// Tools

function setTool(next: Tool) {
  tool = next;
  if (next !== "select") transformer.nodes([]);
  if (next !== "crop") cancelCrop();
  for (const button of document.querySelectorAll<HTMLButtonElement>("#tools button")) {
    button.classList.toggle("active", button.dataset.tool === next);
  }
  syncDraggable();
  stage.container().style.cursor = next === "select" ? "default" : "crosshair";
}

function syncDraggable() {
  for (const node of annLayer.getChildren()) {
    node.draggable(tool === "select" && node.name() !== "pixelate");
    node.listening(tool === "select" && node.name() !== "pixelate");
  }
}

function highlightWidth() {
  return Math.max(16, strokeWidth * 4);
}

function startDraft(pos: { x: number; y: number }): Konva.Shape | null {
  const base = { stroke: color, strokeWidth, name: "" };
  switch (tool) {
    case "arrow":
      return new Konva.Arrow({
        ...base,
        name: "arrow",
        points: [pos.x, pos.y, pos.x, pos.y],
        fill: color,
        pointerLength: 6 + strokeWidth * 2.5,
        pointerWidth: 6 + strokeWidth * 2.5,
      });
    case "rect":
      return new Konva.Rect({
        ...base,
        name: "rect",
        x: pos.x,
        y: pos.y,
        width: 1,
        height: 1,
        cornerRadius: 2,
      });
    case "ellipse":
      return new Konva.Ellipse({
        ...base,
        name: "ellipse",
        x: pos.x,
        y: pos.y,
        radiusX: 1,
        radiusY: 1,
      });
    case "line":
      return new Konva.Line({
        ...base,
        name: "line",
        points: [pos.x, pos.y, pos.x, pos.y],
        lineCap: "round",
      });
    case "pen":
      return new Konva.Line({
        ...base,
        name: "pen",
        points: [pos.x, pos.y],
        lineCap: "round",
        lineJoin: "round",
        tension: 0.4,
      });
    case "highlight":
      return new Konva.Line({
        ...base,
        name: "highlight",
        strokeWidth: highlightWidth(),
        points: [pos.x, pos.y],
        lineCap: "round",
        lineJoin: "round",
        globalCompositeOperation: "multiply",
        opacity: 0.45,
      });
    default:
      return null;
  }
}

function onPointerDown() {
  if (tool === "select" || tool === "text" || tool === "counter") return;
  const pos = pointerPos();
  draftStart = pos;
  if (tool === "crop" || tool === "pixelate") {
    marquee?.destroy();
    marquee = new Konva.Rect({
      x: pos.x,
      y: pos.y,
      width: 1,
      height: 1,
      stroke: tool === "crop" ? "#4f8ef7" : "#ff9500",
      strokeWidth: 2 / scale,
      dash: [6 / scale, 4 / scale],
      listening: false,
    });
    uiLayer.add(marquee);
    return;
  }
  draft = startDraft(pos);
  if (draft) {
    draft.listening(false);
    annLayer.add(draft);
  }
}

function onPointerMove() {
  const pos = pointerPos();
  if (marquee) {
    const rect = normalizeRect(draftStart.x, draftStart.y, pos.x, pos.y);
    marquee.setAttrs(rect);
    uiLayer.batchDraw();
    return;
  }
  if (!draft) return;
  switch (draft.name()) {
    case "arrow":
    case "line":
      (draft as Konva.Line).points([draftStart.x, draftStart.y, pos.x, pos.y]);
      break;
    case "pen":
    case "highlight": {
      const pts = (draft as Konva.Line).points();
      pts.push(pos.x, pos.y);
      (draft as Konva.Line).points(pts);
      break;
    }
    case "rect":
      draft.setAttrs(normalizeRect(draftStart.x, draftStart.y, pos.x, pos.y));
      break;
    case "ellipse":
      draft.setAttrs({
        x: (draftStart.x + pos.x) / 2,
        y: (draftStart.y + pos.y) / 2,
        radiusX: Math.abs(pos.x - draftStart.x) / 2,
        radiusY: Math.abs(pos.y - draftStart.y) / 2,
      });
      break;
  }
  annLayer.batchDraw();
}

function onPointerUp() {
  if (marquee) {
    const clamped = clampRect(
      {
        x: marquee.x(),
        y: marquee.y(),
        width: marquee.width(),
        height: marquee.height(),
      },
      imageSize,
    );
    marquee.destroy();
    marquee = null;
    uiLayer.batchDraw();
    if (clamped.width < 8 || clamped.height < 8) return;
    if (tool === "pixelate") addPixelation(clamped);
    else if (tool === "crop") proposeCrop(clamped);
    return;
  }
  if (!draft) return;
  const node = draft;
  draft = null;
  if (isDegenerate(node)) {
    node.destroy();
    annLayer.batchDraw();
    return;
  }
  commit();
}

function isDegenerate(node: Konva.Shape): boolean {
  const box = node.getClientRect({ relativeTo: annLayer as unknown as Konva.Container });
  return box.width < 3 && box.height < 3;
}

function addPixelation(region: Rect) {
  const canvas = pixelateRegion(sourceImage, region);
  const node = new Konva.Image({
    name: "pixelate",
    image: canvas,
    ...region,
    listening: false,
  });
  node.setAttr("src", canvas.toDataURL("image/png"));
  annLayer.add(node);
  annLayer.batchDraw();
  commit();
}

// ---------------------------------------------------------------------------
// Crop

function proposeCrop(rect: Rect) {
  pendingCrop = rect;
  marquee = new Konva.Rect({
    ...rect,
    stroke: "#4f8ef7",
    strokeWidth: 2 / scale,
    dash: [6 / scale, 4 / scale],
    listening: false,
  });
  uiLayer.add(marquee);
  uiLayer.batchDraw();
  el<HTMLDivElement>("crop-confirm").classList.remove("hidden");
}

function cancelCrop() {
  pendingCrop = null;
  marquee?.destroy();
  marquee = null;
  uiLayer?.batchDraw();
  el<HTMLDivElement>("crop-confirm").classList.add("hidden");
}

function applyCrop() {
  if (pendingCrop) {
    crop = pendingCrop;
    cancelCrop();
    applyView();
    setTool("select");
  }
}

// ---------------------------------------------------------------------------
// Selection & text

function onStageClick(e: Konva.KonvaEventObject<MouseEvent>) {
  if (tool === "text") {
    addText(pointerPos());
    return;
  }
  if (tool === "counter") {
    addCounter(pointerPos());
    return;
  }
  if (tool !== "select") return;
  const target = e.target;
  if (target === stage || target.getLayer() === bgLayer) {
    transformer.nodes([]);
    uiLayer.batchDraw();
    return;
  }
  // Counter badges are groups; clicks land on the inner circle.
  const node = target.findAncestor(".counter") ?? target;
  if (node.getLayer() === annLayer && node.name() !== "pixelate") {
    transformer.nodes([node]);
    uiLayer.batchDraw();
  }
}

function onStageDblClick(e: Konva.KonvaEventObject<MouseEvent>) {
  if (tool === "select" && e.target.name() === "text") {
    editText(e.target as Konva.Text);
  }
}

/** Stamp a numbered badge; the tool stays active for sequential stamping. */
function addCounter(pos: { x: number; y: number }) {
  const existing = annLayer
    .getChildren((node) => node.name() === "counter")
    .map((node) => Number((node as Konva.Node).getAttr("number")))
    .filter((n) => Number.isFinite(n));
  const node = buildCounter({
    x: pos.x,
    y: pos.y,
    radius: Math.max(14, strokeWidth * 3.5),
    fill: color,
    number: nextCounterNumber(existing),
  });
  annLayer.add(node);
  annLayer.batchDraw();
  commit();
}

function addText(pos: { x: number; y: number }) {
  const node = new Konva.Text({
    name: "text",
    x: pos.x,
    y: pos.y,
    text: "",
    fontSize: Math.max(18, strokeWidth * 6),
    fontStyle: "bold",
    fill: color,
  });
  annLayer.add(node);
  setTool("select");
  editText(node);
}

/** Standard Konva recipe: float a textarea over the text node. */
function editText(node: Konva.Text) {
  node.hide();
  transformer.nodes([]);
  annLayer.batchDraw();

  const stageBox = stage.container().getBoundingClientRect();
  // getAbsolutePosition(stage) is in image coordinates (excludes the stage's
  // fit-scale/pan transform) — map it to screen space before styling.
  const abs = node.getAbsolutePosition(stage);
  const screen = imageToScreen(abs, stage.position(), scale);
  const area = document.createElement("textarea");
  document.body.appendChild(area);
  area.className = "text-editor";
  area.value = node.text();
  area.style.left = `${stageBox.left + screen.x}px`;
  area.style.top = `${stageBox.top + screen.y}px`;
  area.style.width = `${Math.max(160, node.width() * scale + 20)}px`;
  area.style.height = `${Math.max(node.height() * scale + 8, node.fontSize() * scale * 1.4)}px`;
  area.style.fontSize = `${node.fontSize() * scale}px`;
  area.style.fontFamily = "-apple-system, system-ui, sans-serif";
  area.style.fontWeight = "bold";
  area.style.color = node.fill() as string;
  area.focus();

  const done = (apply: boolean) => {
    if (apply) node.text(area.value.trim());
    area.remove();
    node.show();
    if (node.text() === "") node.destroy();
    annLayer.batchDraw();
    commit();
  };
  area.addEventListener("blur", () => done(true));
  area.addEventListener("keydown", (event) => {
    event.stopPropagation();
    if (event.key === "Enter" && !event.shiftKey) area.blur();
    if (event.key === "Escape") {
      area.value = node.text();
      area.blur();
    }
  });
}

function deleteSelection() {
  const nodes = transformer.nodes();
  if (nodes.length === 0) return;
  transformer.nodes([]);
  for (const node of nodes) node.destroy();
  annLayer.batchDraw();
  commit();
}

// ---------------------------------------------------------------------------
// Export

const PNG_PREFIX = "data:image/png;base64,";

/**
 * Render the annotated image to base64 PNG at native resolution. WebKit returns
 * an empty string (or throws) when the export canvas is too large — a full-res
 * multi-display/Retina screenshot can exceed its limits — so retry at
 * progressively lower resolution rather than emit empty data (which would
 * otherwise overwrite the capture with nothing).
 */
function renderPng(): string {
  transformer.nodes([]);
  marquee?.hide();
  uiLayer.batchDraw();
  try {
    let pixelRatio = 1 / scale;
    for (let attempt = 0; attempt < 5; attempt++) {
      let dataUrl = "";
      try {
        dataUrl = stage.toDataURL({
          x: 0,
          y: 0,
          width: stage.width(),
          height: stage.height(),
          pixelRatio,
          mimeType: "image/png",
        });
      } catch {
        // oversized/tainted canvas — fall through and try a smaller one
      }
      if (dataUrl.startsWith(PNG_PREFIX) && dataUrl.length > PNG_PREFIX.length + 32) {
        return dataUrl.slice(PNG_PREFIX.length);
      }
      pixelRatio /= 2;
    }
    throw new Error(t("editor.export_failed"));
  } finally {
    marquee?.show();
  }
}

async function exportPng(action: Record<string, unknown>) {
  await invoke("export_png", { data: renderPng(), action });
}

// ---------------------------------------------------------------------------
// Wiring

/** Persist the current tool/color/stroke so the next editor session restores them. */
function savePrefs() {
  void invoke("set_editor_prefs", {
    prefs: { tool, color, stroke_width: strokeWidth },
  }).catch(() => {});
}

/** Set the active color; optionally apply it to the current selection. */
function selectColor(value: string, applyToSelection: boolean) {
  color = value;
  const colors = el<HTMLDivElement>("colors");
  for (const other of colors.children) {
    other.classList.toggle("active", (other as HTMLElement).title === value);
  }
  if (applyToSelection) {
    for (const node of transformer.nodes()) {
      if (node.name() === "text") node.setAttr("fill", value);
      else if (node.name() === "counter") {
        node.setAttr("fill", value);
        (node as Konva.Group).findOne("Circle")?.setAttr("fill", value);
      } else {
        node.setAttr("stroke", value);
        if (node.name() === "arrow") node.setAttr("fill", value);
      }
    }
    if (transformer.nodes().length > 0) {
      annLayer.batchDraw();
      commit();
    }
  }
}

function buildToolbar() {
  for (const button of document.querySelectorAll<HTMLButtonElement>("#tools button")) {
    button.onclick = () => {
      setTool(button.dataset.tool as Tool);
      savePrefs();
    };
  }

  const colors = el<HTMLDivElement>("colors");
  for (const value of COLORS) {
    const swatch = document.createElement("button");
    swatch.className = "swatch";
    swatch.style.background = value;
    swatch.title = value;
    swatch.onclick = () => {
      selectColor(value, true);
      savePrefs();
    };
    colors.appendChild(swatch);
  }

  el<HTMLInputElement>("stroke-width").oninput = (event) => {
    strokeWidth = Number((event.target as HTMLInputElement).value);
    savePrefs();
  };

  el<HTMLButtonElement>("undo").onclick = undo;
  el<HTMLButtonElement>("redo").onclick = redo;
  el<HTMLButtonElement>("crop-apply").onclick = applyCrop;
  el<HTMLButtonElement>("crop-cancel").onclick = () => cancelCrop();
  el<HTMLButtonElement>("reset-crop").onclick = () => {
    crop = null;
    applyView();
  };

  el<HTMLButtonElement>("copy").onclick = () =>
    void guard(() => exportPng({ kind: "copy" }));
  el<HTMLButtonElement>("save").onclick = () =>
    void guard(async () => {
      const dest = await savePngAs(`annotated-${captureId}`);
      if (dest) await exportPng({ kind: "save_to", dest });
    });
  el<HTMLButtonElement>("done").onclick = () =>
    void guard(async () => {
      await exportPng({ kind: "overwrite", id: captureId });
      await getCurrentWindow().hide();
    });
}

/** Run an editor action, surfacing any failure instead of silently dropping it. */
async function guard(action: () => Promise<void>) {
  try {
    await action();
  } catch (err) {
    showError(String(err));
  }
}

function showError(message: string) {
  const bar = el<HTMLDivElement>("error-bar");
  bar.textContent = message;
  bar.classList.remove("hidden");
  window.setTimeout(() => bar.classList.add("hidden"), 4000);
}

function bindKeyboard() {
  window.addEventListener("keydown", (event) => {
    if (document.activeElement?.tagName === "TEXTAREA") return;
    const primary = event.metaKey || event.ctrlKey;
    if (primary && event.key.toLowerCase() === "z") {
      event.preventDefault();
      event.shiftKey ? redo() : undo();
      return;
    }
    if (event.key === "Delete" || event.key === "Backspace") {
      deleteSelection();
      return;
    }
    if (event.key === "Escape") {
      cancelCrop();
      transformer.nodes([]);
      uiLayer.batchDraw();
      return;
    }
    const shortcuts: Record<string, Tool> = {
      v: "select",
      a: "arrow",
      r: "rect",
      e: "ellipse",
      l: "line",
      p: "pen",
      h: "highlight",
      t: "text",
      n: "counter",
      b: "pixelate",
      c: "crop",
    };
    const next = shortcuts[event.key.toLowerCase()];
    if (next && !primary) {
      setTool(next);
      savePrefs();
    }
  });
}

/** Restore the last-used tool/color/stroke from persisted preferences. */
async function applyPrefs() {
  const valid: Tool[] = [
    "select", "arrow", "rect", "ellipse", "line",
    "pen", "highlight", "text", "counter", "pixelate", "crop",
  ];
  try {
    const prefs = await invoke<{ tool: string; color: string; stroke_width: number }>(
      "get_editor_prefs",
    );
    selectColor(prefs.color, false);
    strokeWidth = prefs.stroke_width;
    el<HTMLInputElement>("stroke-width").value = String(prefs.stroke_width);
    setTool((valid.includes(prefs.tool as Tool) ? prefs.tool : "select") as Tool);
  } catch {
    setTool("select");
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  await initI18n();
  buildStage();
  buildToolbar();
  bindKeyboard();
  await applyPrefs();

  // Reload requests while the window is already open (reuse path).
  await listen<CaptureEntry>("editor:load", (event) => void loadCapture(event.payload));

  // On (re)load, always pull the current target from the backend rather than
  // relying on an event that may have fired before this listener existed.
  try {
    await loadCapture(await invoke<CaptureEntry>("editor_target"));
  } catch (err) {
    console.error("failed to load capture for editor", err);
  }
});
