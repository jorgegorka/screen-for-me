import Konva from "konva";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { savePngAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import { normalizeRect } from "../shared/geometry";
import type { CaptureEntry } from "../shared/ipc";
import {
  clampRect,
  clampScale,
  imageToScreen,
  initialScale,
  nextFitScale,
  type Rect,
  type Size,
} from "./geometry";
import { counterTextColor, nextCounterNumber } from "./counter";
import { UndoStack } from "./history";
import { pixelateRegion } from "./pixelate";
import { buildArrow, buildCounter, rebuildLayer, serializeLayer, type ShapeType } from "./shapes";

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
  // Virtual scrolling: #stage-holder is a spacer with the full zoomed size,
  // while the canvas itself (Konva's content div) never exceeds the visible
  // viewport and sticks to it as the user scrolls. A canvas sized to the whole
  // zoomed image would exceed WebKit's canvas limits on large captures (e.g.
  // scrolling captures) and kill the webview.
  stage.content.style.position = "sticky";
  stage.content.style.top = "0px";
  stage.content.style.left = "0px";
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
  const scroll = el<HTMLDivElement>("canvas-scroll");
  const holder = el<HTMLDivElement>("stage-holder");
  const width = view.width * scale;
  const height = view.height * scale;
  holder.style.width = `${width}px`;
  holder.style.height = `${height}px`;
  stage.size({
    width: Math.min(width, scroll.clientWidth),
    height: Math.min(height, scroll.clientHeight),
  });
  stage.scale({ x: scale, y: scale });
  syncStagePosition();
  el<HTMLButtonElement>("reset-crop").classList.toggle("hidden", crop === null);
  el<HTMLButtonElement>("zoom-level").textContent = `${Math.round(scale * 100)}%`;
}

/** Align the viewport-pinned canvas with the scrolled/zoomed view. */
function syncStagePosition() {
  const view = crop ?? { x: 0, y: 0, ...imageSize };
  const scroll = el<HTMLDivElement>("canvas-scroll");
  stage.position({
    x: -view.x * scale - scroll.scrollLeft,
    y: -view.y * scale - scroll.scrollTop,
  });
  stage.batchDraw();
}

// ---------------------------------------------------------------------------
// Zoom

/**
 * Set the view scale, keeping the image point under `focal` (client
 * coordinates; defaults to the viewport centre) stationary on screen.
 */
function setZoom(next: number, focal?: { x: number; y: number }) {
  const target = clampScale(next);
  if (target === scale) return;
  const scroll = el<HTMLDivElement>("canvas-scroll");
  const holder = el<HTMLDivElement>("stage-holder");
  const box = scroll.getBoundingClientRect();
  const point = focal ?? { x: box.left + scroll.clientWidth / 2, y: box.top + scroll.clientHeight / 2 };
  const view = crop ?? { x: 0, y: 0, ...imageSize };
  const before = holder.getBoundingClientRect();
  const image = {
    x: (point.x - before.left) / scale + view.x,
    y: (point.y - before.top) / scale + view.y,
  };
  scale = target;
  applyView();
  const after = holder.getBoundingClientRect();
  scroll.scrollLeft += after.left + (image.x - view.x) * scale - point.x;
  scroll.scrollTop += after.top + (image.y - view.y) * scale - point.y;
  syncStagePosition();
}

function zoomBy(factor: number) {
  setZoom(scale * factor);
}

/** Fit the current view: smart fit first, toggling to the whole image on repeat. */
function zoomFit() {
  const view = crop ?? { x: 0, y: 0, ...imageSize };
  scale = nextFitScale(scale, view, viewportSize());
  applyView();
}

function bindViewportEvents() {
  const scroll = el<HTMLDivElement>("canvas-scroll");
  scroll.addEventListener("scroll", syncStagePosition);
  window.addEventListener("resize", () => {
    if (imageSize.width > 0) applyView();
  });
  // Pinch on Chromium-style engines and ⌘/Ctrl+scroll everywhere.
  scroll.addEventListener(
    "wheel",
    (event) => {
      if (!event.ctrlKey && !event.metaKey) return;
      event.preventDefault();
      setZoom(scale * Math.exp(-event.deltaY * 0.01), { x: event.clientX, y: event.clientY });
    },
    { passive: false },
  );
  // Trackpad pinch on WebKit arrives as proprietary gesture events.
  let gestureBase = 1;
  scroll.addEventListener("gesturestart", (event) => {
    event.preventDefault();
    gestureBase = scale;
  });
  scroll.addEventListener("gesturechange", (event) => {
    event.preventDefault();
    const g = event as unknown as { scale: number; clientX: number; clientY: number };
    setZoom(gestureBase * g.scale, { x: g.clientX, y: g.clientY });
  });
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
  scale = initialScale(imageSize, viewportSize());

  bgLayer.destroyChildren();
  bgLayer.add(
    new Konva.Image({ image: sourceImage, ...imageSize, listening: false }),
  );
  annLayer.destroyChildren();
  transformer.nodes([]);
  undoStack = new UndoStack(serializeLayer(annLayer));
  syncUndoButtons();
  applyView();
  // A width-fitted tall capture starts taller than the viewport: read from the top.
  const scroll = el<HTMLDivElement>("canvas-scroll");
  scroll.scrollTop = 0;
  scroll.scrollLeft = 0;
}

// ---------------------------------------------------------------------------
// Undo / redo

function commit() {
  undoStack.commit(serializeLayer(annLayer));
  syncUndoButtons();
}

function syncUndoButtons() {
  el<HTMLButtonElement>("undo").disabled = !undoStack.canUndo;
  el<HTMLButtonElement>("redo").disabled = !undoStack.canRedo;
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
  syncUndoButtons();
}

function redo() {
  const snapshot = undoStack.redo();
  if (snapshot !== null) restore(snapshot);
  syncUndoButtons();
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
      return buildArrow({
        name: "arrow",
        points: [pos.x, pos.y, pos.x, pos.y],
        fill: color,
        strokeWidth,
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
      stroke: tool === "crop" ? "#9172e7" : "#ff9500",
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
      // buildArrow returns a generic Konva.Shape — no .points() method
      // (Konva only registers it on Line.prototype); update the attr directly.
      draft.setAttr("points", [draftStart.x, draftStart.y, pos.x, pos.y]);
      break;
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
    stroke: "#9172e7",
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

  // The canvas (Konva content div) is viewport-pinned; its rect, not the
  // scrolling holder's, anchors screen-space overlays.
  const stageBox = stage.content.getBoundingClientRect();
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
  // The on-screen stage is viewport-pinned and only renders the visible slice,
  // so temporarily give it the whole view for export — at a scale that keeps
  // the intermediate layer canvases within WebKit's size limits. toDataURL
  // re-renders vectors at `pixelRatio`, so the export itself stays native-res.
  const view = crop ?? { x: 0, y: 0, ...imageSize };
  const dpr = window.devicePixelRatio || 1;
  const MAX_LAYER_DIM = 8192;
  const exportScale = Math.min(
    1,
    MAX_LAYER_DIM / (view.width * dpr),
    MAX_LAYER_DIM / (view.height * dpr),
  );
  stage.size({ width: view.width * exportScale, height: view.height * exportScale });
  stage.scale({ x: exportScale, y: exportScale });
  stage.position({ x: -view.x * exportScale, y: -view.y * exportScale });
  try {
    let pixelRatio = 1 / exportScale;
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
    applyView();
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
        (node as Konva.Group).findOne("Text")?.setAttr("fill", counterTextColor(value));
      } else if (node.name() === "arrow") {
        node.setAttr("fill", value);
      } else {
        node.setAttr("stroke", value);
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
  el<HTMLButtonElement>("zoom-in").onclick = () => zoomBy(1.25);
  el<HTMLButtonElement>("zoom-out").onclick = () => zoomBy(1 / 1.25);
  el<HTMLButtonElement>("zoom-level").onclick = () => setZoom(1);
  el<HTMLButtonElement>("zoom-fit").onclick = zoomFit;
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
    if (primary && (event.key === "=" || event.key === "+")) {
      event.preventDefault();
      zoomBy(1.25);
      return;
    }
    if (primary && event.key === "-") {
      event.preventDefault();
      zoomBy(1 / 1.25);
      return;
    }
    if (primary && event.key === "0") {
      event.preventDefault();
      zoomFit();
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
  bindViewportEvents();
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
