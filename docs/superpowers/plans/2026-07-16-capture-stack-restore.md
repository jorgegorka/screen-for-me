# Capture Stack Overlay + Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the quick-access overlay into a stack of capture panels (new captures and history "Restore" both push a panel on top) and reduce History cards to two buttons: Copy + Restore.

**Architecture:** The single persistent `overlay` window grows into a vertical column of cards, bottom-anchored so it grows upward. Stack state (ordered list, newest first) lives in the overlay webview; a pure `src/overlay/stack.ts` module holds the ordering logic. The Rust backend stays the single owner of the window's size: a new `set_overlay_panels(count)` command resizes/re-places the window and returns the count clamped to what fits the monitor; a new `restore_capture(id)` command emits a `capture:restore` event and shows the overlay.

**Tech Stack:** Tauri v2 (Rust backend), TypeScript + Vite frontend, vitest for frontend unit tests, `cargo test` for Rust.

**Spec:** `docs/superpowers/specs/2026-07-16-capture-stack-restore-design.md`

## Global Constraints

- All user-visible strings go through i18n: keys must exist in all five catalogs `locales/{en-GB,es,fr,de,it}.json` (parity is unit-tested in `src/shared/i18n.test.ts`).
- Never localise command names, event names, or settings enum values.
- Done gate for the whole feature: `npm run build`, `npm test`, and `cd src-tauri && cargo test` all pass.
- Run all commands from the repo root `/Users/jorge/Sites/native/screenforme` unless a step says otherwise.
- `cargo test` compiles the whole crate — it doubles as the Rust type-check.
- Follow existing code style: 2-space TS, rustfmt Rust, doc comments explain *why*.

---

### Task 1: Pure stack-ordering module (`src/overlay/stack.ts`)

**Files:**
- Create: `src/overlay/stack.ts`
- Test: `src/overlay/stack.test.ts`

**Interfaces:**
- Consumes: `CaptureEntry` type from `src/shared/ipc.ts` (`{ path: string; id: string; created_ms: number }`).
- Produces (used by Task 4):
  - `pushTop(stack: CaptureEntry[], entry: CaptureEntry): CaptureEntry[]` — new array with `entry` at index 0; if an entry with the same `id` exists it is moved (not duplicated).
  - `removeEntry(stack: CaptureEntry[], id: string): CaptureEntry[]`
  - `trimStack(stack: CaptureEntry[], max: number): CaptureEntry[]` — keeps the first (newest) `max` entries.
  - Convention: **index 0 is the top of the visual stack (newest)**.

- [ ] **Step 1: Write the failing test**

Create `src/overlay/stack.test.ts`:

```ts
import { describe, expect, it } from "vitest";

import type { CaptureEntry } from "../shared/ipc";
import { pushTop, removeEntry, trimStack } from "./stack";

const entry = (id: string): CaptureEntry => ({
  id,
  path: `/captures/${id}`,
  created_ms: 0,
});

describe("pushTop", () => {
  it("puts a new entry at the top", () => {
    const stack = pushTop([entry("a.png")], entry("b.png"));
    expect(stack.map((e) => e.id)).toEqual(["b.png", "a.png"]);
  });

  it("moves an existing entry to the top instead of duplicating", () => {
    const stack = [entry("a.png"), entry("b.png"), entry("c.png")];
    const next = pushTop(stack, entry("b.png"));
    expect(next.map((e) => e.id)).toEqual(["b.png", "a.png", "c.png"]);
  });

  it("does not mutate the input", () => {
    const stack = [entry("a.png")];
    pushTop(stack, entry("b.png"));
    expect(stack.map((e) => e.id)).toEqual(["a.png"]);
  });
});

describe("removeEntry", () => {
  it("removes only the matching panel", () => {
    const stack = [entry("a.png"), entry("b.png")];
    expect(removeEntry(stack, "a.png").map((e) => e.id)).toEqual(["b.png"]);
  });

  it("is a no-op for an unknown id", () => {
    const stack = [entry("a.png")];
    expect(removeEntry(stack, "x.png").map((e) => e.id)).toEqual(["a.png"]);
  });
});

describe("trimStack", () => {
  it("keeps the newest entries (top of the stack)", () => {
    const stack = [entry("c.png"), entry("b.png"), entry("a.png")];
    expect(trimStack(stack, 2).map((e) => e.id)).toEqual(["c.png", "b.png"]);
  });

  it("returns the stack unchanged when it already fits", () => {
    const stack = [entry("a.png")];
    expect(trimStack(stack, 5).map((e) => e.id)).toEqual(["a.png"]);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/overlay/stack.test.ts`
Expected: FAIL — cannot resolve `./stack`.

- [ ] **Step 3: Write the implementation**

Create `src/overlay/stack.ts`:

```ts
import type { CaptureEntry } from "../shared/ipc";

// Pure stack-ordering logic for the overlay's panel column, kept free of
// Tauri/DOM imports so vitest covers it. Index 0 is the top of the visual
// stack (newest panel).

/** Push on top; an entry already in the stack moves up instead of duplicating. */
export function pushTop(stack: CaptureEntry[], entry: CaptureEntry): CaptureEntry[] {
  return [entry, ...stack.filter((e) => e.id !== entry.id)];
}

export function removeEntry(stack: CaptureEntry[], id: string): CaptureEntry[] {
  return stack.filter((e) => e.id !== id);
}

/** Keep the newest `max` panels, dropping from the bottom of the stack. */
export function trimStack(stack: CaptureEntry[], max: number): CaptureEntry[] {
  return stack.slice(0, Math.max(max, 0));
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/overlay/stack.test.ts`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src/overlay/stack.ts src/overlay/stack.test.ts
git commit -m "Add pure stack-ordering module for the overlay panel column"
```

---

### Task 2: Backend panel sizing — `set_overlay_panels`, monitor clamp

**Files:**
- Modify: `src-tauri/src/commands.rs` (AppState, `place_overlay`, `show_overlay`, follow loop, new command, tests)
- Modify: `src-tauri/src/lib.rs` (AppState init, command registration)

**Interfaces:**
- Consumes: existing `overlay_origin`, `monitor_logical_bounds`, `OVERLAY_BASE_WIDTH`/`OVERLAY_BASE_HEIGHT`, `AppState`.
- Produces (used by Task 4):
  - Tauri command `set_overlay_panels(count: usize) -> usize` — invoked from JS as `invoke<number>("set_overlay_panels", { count })`. Resizes the overlay window to `clamped × panel height`, re-places it bottom-anchored, stores the count in `AppState`, and returns the clamped count.
  - New AppState field `overlay_panels: std::sync::atomic::AtomicUsize` (init 1).
  - Private pure fn `clamp_panels(requested: usize, panel_height: f64, monitor_height: f64) -> usize`.

- [ ] **Step 1: Write the failing Rust test**

In `src-tauri/src/commands.rs`, inside the existing `mod tests` at the bottom (after `overlay_origin_respects_monitor_offset`), add:

```rust
    #[test]
    fn clamp_panels_fits_monitor_height() {
        // 1080-high monitor, 264-high panels: (1080 - 32) / 264 = 3.96 → 3.
        assert_eq!(clamp_panels(1, 264.0, 1080.0), 1);
        assert_eq!(clamp_panels(3, 264.0, 1080.0), 3);
        assert_eq!(clamp_panels(9, 264.0, 1080.0), 3);
    }

    #[test]
    fn clamp_panels_never_returns_zero() {
        // A stack request of 0 (or a monitor too short for even one panel)
        // still sizes the window for one panel.
        assert_eq!(clamp_panels(0, 264.0, 1080.0), 1);
        assert_eq!(clamp_panels(5, 264.0, 100.0), 1);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test clamp_panels`
Expected: FAIL to compile — `clamp_panels` not found.

- [ ] **Step 3: Implement the sizing changes**

All in `src-tauri/src/commands.rs`:

**3a.** Add the field to `AppState` (after `overlay_drag_active`):

```rust
    /// Number of panels the overlay webview is currently stacking; owned
    /// here so `show_overlay` and the follow loop size the window without a
    /// round-trip to the webview. Updated by `set_overlay_panels`.
    pub overlay_panels: std::sync::atomic::AtomicUsize,
```

**3b.** Add the pure clamp next to `overlay_origin`:

```rust
/// How many stacked panels the window may hold on a monitor: the full stack
/// must fit the monitor's logical height minus the top/bottom margins.
/// Always at least 1 so a lone panel still shows on tiny screens.
fn clamp_panels(requested: usize, panel_height: f64, monitor_height: f64) -> usize {
    const MARGIN: f64 = 16.0;
    let fit = ((monitor_height - 2.0 * MARGIN) / panel_height).floor() as usize;
    requested.clamp(1, fit.max(1))
}
```

**3c.** Change `place_overlay` to take a panel count (total height = count × panel height, clamped to the monitor):

```rust
/// Size the overlay for `panels` stacked cards per settings and place it at
/// the configured corner of `monitor`, bottom-anchored so the stack grows
/// upward.
fn place_overlay(
    overlay: &tauri::WebviewWindow,
    monitor: &tauri::Monitor,
    settings: &Settings,
    panels: usize,
) {
    let width = OVERLAY_BASE_WIDTH * settings.overlay_size;
    let panel_height = OVERLAY_BASE_HEIGHT * settings.overlay_size;
    let (mon_pos, mon_size) = monitor_logical_bounds(monitor);
    let height = panel_height * clamp_panels(panels, panel_height, mon_size.height) as f64;
    let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
    let (x, y) = overlay_origin(
        settings.position,
        (mon_pos.x, mon_pos.y),
        (mon_size.width, mon_size.height),
        (width, height),
    );
    let _ = overlay.set_position(tauri::LogicalPosition::new(x, y));
}
```

**3d.** In `show_overlay`, read the count and pass it through. Replace the `match monitor` block with:

```rust
    let panels = state
        .overlay_panels
        .load(std::sync::atomic::Ordering::SeqCst);
    match monitor {
        Some(monitor) => place_overlay(&overlay, &monitor, &settings, panels),
        None => {
            let width = OVERLAY_BASE_WIDTH * settings.overlay_size;
            let height = OVERLAY_BASE_HEIGHT * settings.overlay_size * panels.max(1) as f64;
            let _ = overlay.set_size(tauri::LogicalSize::new(width, height));
        }
    }
```

**3e.** In `follow_active_monitor`'s loop, the `place_overlay(&overlay, &target, &settings);` call becomes:

```rust
                place_overlay(
                    &overlay,
                    &target,
                    &settings,
                    state.overlay_panels.load(Ordering::SeqCst),
                );
```

**3f.** Add the command (next to `set_overlay_drag_active`):

```rust
/// The overlay webview reports its stack size here. The backend stays the
/// single owner of the window's geometry: it clamps the count to what fits
/// the monitor, resizes/re-places the bottom-anchored window, and returns
/// the clamped count — the webview trims its stack to that value.
#[tauri::command]
pub fn set_overlay_panels(app: AppHandle, state: State<AppState>, count: usize) -> usize {
    use std::sync::atomic::Ordering;
    let settings = state.settings.get();
    let Some(overlay) = app.get_webview_window("overlay") else {
        let count = count.max(1);
        state.overlay_panels.store(count, Ordering::SeqCst);
        return count;
    };
    let monitor = overlay
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| overlay.primary_monitor().ok().flatten());
    let clamped = match &monitor {
        Some(monitor) => {
            let (_, mon_size) = monitor_logical_bounds(monitor);
            let panel_height = OVERLAY_BASE_HEIGHT * settings.overlay_size;
            clamp_panels(count, panel_height, mon_size.height)
        }
        None => count.max(1),
    };
    state.overlay_panels.store(clamped, Ordering::SeqCst);
    if let Some(monitor) = monitor {
        place_overlay(&overlay, &monitor, &settings, clamped);
    }
    clamped
}
```

**3g.** In `src-tauri/src/lib.rs`: add to the `app.manage(AppState { ... })` initializer (after `overlay_drag_active`):

```rust
                overlay_panels: std::sync::atomic::AtomicUsize::new(1),
```

and add to `tauri::generate_handler![...]` (after `commands::set_overlay_drag_active`):

```rust
            commands::set_overlay_panels,
```

- [ ] **Step 4: Run the Rust tests**

Run: `cd src-tauri && cargo test`
Expected: PASS, including `clamp_panels_fits_monitor_height`, `clamp_panels_never_returns_zero`, and the pre-existing `overlay_origin_respects_monitor_offset`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "Size the overlay window for a stack of panels (set_overlay_panels)"
```

---

### Task 3: Backend `restore_capture` command

**Files:**
- Modify: `src-tauri/src/commands.rs` (new command, next to `copy_capture`)
- Modify: `src-tauri/src/lib.rs` (registration)

**Interfaces:**
- Consumes: `resolve(history, id)` helper (returns `Result<CaptureEntry, String>`), `show_overlay(app)`, `app.emit`.
- Produces (used by Tasks 4 & 5):
  - Tauri command `restore_capture(id: String) -> Result<(), String>` — invoked from JS as `invoke("restore_capture", { id })`.
  - Event **`capture:restore`** with a `CaptureEntry` payload, emitted app-wide before the overlay is shown.

- [ ] **Step 1: Add the command**

In `src-tauri/src/commands.rs`, after `copy_capture`:

```rust
/// Re-open the overlay with a capture from history: emits `capture:restore`
/// (the overlay pushes it on top of its stack, or moves it up if already
/// shown) and shows the window. No clipboard side effects.
#[tauri::command]
pub fn restore_capture(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let entry = resolve(&state.history, &id)?;
    let _ = app.emit("capture:restore", &entry);
    show_overlay(&app);
    Ok(())
}
```

In `src-tauri/src/lib.rs`, add to `generate_handler!` (after `commands::copy_capture`):

```rust
            commands::restore_capture,
```

- [ ] **Step 2: Verify it compiles and tests pass**

Run: `cd src-tauri && cargo test`
Expected: PASS (this command has no pure logic of its own — `resolve`'s traversal rejection is already covered by `history.rs` tests; the compile is the check here).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "Add restore_capture command emitting capture:restore"
```

---

### Task 4: Overlay frontend — stacked panels

**Files:**
- Modify: `overlay.html` (repo root)
- Modify: `src/overlay/overlay.css`
- Modify: `src/overlay/main.ts` (rewrite)

**Interfaces:**
- Consumes: `pushTop`/`removeEntry`/`trimStack` from Task 1; `set_overlay_panels` from Task 2; `capture:restore` event from Task 3; existing commands `copy_capture`, `save_capture_to`, `save_capture_to_desktop`, `reveal_capture`, `open_editor`, `list_captures`, `set_overlay_drag_active`; `applyTranslations(root)` from `src/shared/i18n.ts`.
- Produces: the overlay renders one card per stack entry, newest on top; the `#stack-badge` element and its CSS are removed.

- [ ] **Step 1: Replace the overlay markup**

Replace the entire `<body>` of `overlay.html` with (head stays unchanged):

```html
  <body>
    <div id="stack"></div>
    <template id="panel-template">
      <div class="card">
        <button class="dismiss" title="Dismiss" data-i18n-title="overlay.dismiss">✕</button>
        <div class="thumb-wrap">
          <img class="thumb" alt="Capture" data-i18n-alt="overlay.latest_capture" draggable="false" />
        </div>
        <div class="actions">
          <button class="copy" title="Copy to clipboard" data-i18n="overlay.copy" data-i18n-title="overlay.copy_tooltip">Copy</button>
          <button class="save" title="Save as…" data-i18n="overlay.save" data-i18n-title="overlay.save_tooltip">Save</button>
          <button class="annotate" title="Annotate" data-i18n="overlay.annotate" data-i18n-title="overlay.annotate">Annotate</button>
          <button class="reveal" title="Show in Finder" data-i18n="overlay.reveal" data-i18n-title="overlay.reveal_tooltip">Finder</button>
        </div>
        <div class="toast hidden"></div>
      </div>
    </template>
  </body>
```

Note: ids became classes (cards are cloned N times); the `#stack-badge` span is gone. The `data-i18n*` attributes stay so `applyTranslations` re-labels live language switches; cards get an explicit `applyTranslations(card)` at build time because template clones are inserted after `initI18n` ran.

- [ ] **Step 2: Update the CSS**

In `src/overlay/overlay.css`:

**2a.** After the `.hidden` rule, add:

```css
#stack {
  display: flex;
  flex-direction: column;
  height: 100%;
}
```

**2b.** Replace the `.card` rule's first two lines (`position: absolute; inset: 8px;`) so the rule becomes:

```css
.card {
  position: relative;
  flex: 1 1 0;
  min-height: 0;
  margin: 8px;
  display: flex;
  flex-direction: column;
  background: rgba(24, 24, 26, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45);
  padding: 10px;
  gap: 8px;
}
```

(The window is sized to `panels × panel height` by the backend, so `flex: 1 1 0` gives every card exactly one panel slot; `margin: 8px` reproduces today's inset for a single panel.)

**2c.** Change the `#thumb` selector to `.thumb` (same declarations).

**2d.** Delete the whole `.badge { ... }` rule.

- [ ] **Step 3: Rewrite `src/overlay/main.ts`**

Replace the file's entire contents with:

```ts
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { startDrag } from "@crabnebula/tauri-plugin-drag";

import { savePngAs } from "../shared/dialogs";
import { el } from "../shared/dom";
import { applyTranslations, initI18n, t } from "../shared/i18n";
import type { CaptureEntry, Settings } from "../shared/ipc";
import { pushTop, removeEntry, trimStack } from "./stack";

/** The overlay only cares about the auto-close/drag subset of Settings. */
type OverlaySettings = Pick<
  Settings,
  "auto_close_enabled" | "auto_close_action" | "auto_close_seconds" | "close_after_drag"
>;

/** One stacked card: its entry, DOM node, and its own auto-close timer. */
interface Panel {
  entry: CaptureEntry;
  node: HTMLElement;
  hovering: boolean;
  hideTimer?: number;
}

// Ordered newest-first (index 0 = top card). Lives in this webview: the
// window hides instead of closing, so the stack survives hide/show —
// including the temporary hide while a capture is in progress.
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
  applyTranslations(node);
  const panel: Panel = { entry, node, hovering: false };

  node.addEventListener("mouseenter", () => (panel.hovering = true));
  node.addEventListener("mouseleave", () => {
    panel.hovering = false;
    armAutoHide(panel);
  });

  const query = <T extends HTMLElement>(selector: string) =>
    node.querySelector<T>(selector)!;

  // cache-bust: a restore can re-show a file the webview saw before
  const thumb = query<HTMLImageElement>(".thumb");
  thumb.src = `${convertFileSrc(entry.path)}?t=${entry.created_ms}`;

  query<HTMLButtonElement>(".dismiss").onclick = () => removePanel(entry.id);

  // Drag the capture out to other apps: native drag starts once the pointer
  // moves a few pixels with the button held on the thumbnail.
  thumb.addEventListener("mousedown", (down) => {
    const cancel = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", cancel);
    };
    const onMove = (move: MouseEvent) => {
      if (Math.hypot(move.clientX - down.clientX, move.clientY - down.clientY) < 5) return;
      cancel();
      const keepOpen = move.altKey; // ⌥ at drag start keeps the panel
      // Pause the backend's follow-the-cursor loop for the duration so it
      // can't relocate this window mid-drag: the plugin's result callback
      // fires on both drop and cancel, and the settled promise is a fallback.
      const dragEnded = () => void invoke("set_overlay_drag_active", { active: false });
      void invoke("set_overlay_drag_active", { active: true });
      void startDrag({ item: [entry.path], icon: entry.path }, dragEnded)
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
      const dest = await savePngAs(entry.id);
      if (dest) {
        await invoke("save_capture_to", { id: entry.id, dest });
        toast(panel, t("overlay.toast_saved"));
      }
    });

  query<HTMLButtonElement>(".reveal").onclick = () =>
    run(panel, () => invoke("reveal_capture", { id: entry.id }));

  query<HTMLButtonElement>(".annotate").onclick = () =>
    run(panel, () => invoke("open_editor", { id: entry.id }));

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

/**
 * Reconcile the DOM with `stack` and hand the panel count to the backend
 * (the single owner of the window's size). The returned count is clamped to
 * what fits the monitor; drop the bottom-most panels beyond it.
 */
async function syncStack() {
  for (const id of [...panels.keys()]) {
    if (!stack.some((e) => e.id === id)) dropPanel(id);
  }
  if (stack.length === 0) {
    await appWindow.hide();
    // Reset the backend's size for the next single-panel show.
    void invoke("set_overlay_panels", { count: 1 }).catch(() => {});
    return;
  }
  for (const entry of stack) {
    if (!panels.has(entry.id)) {
      const panel = buildPanel(entry);
      panels.set(entry.id, panel);
      armAutoHide(panel);
    }
  }
  // append() moves existing nodes, so this both inserts and re-orders.
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

/** A new capture or a restore lands on top; a restore of an open panel moves it up. */
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

  // If the window was shown before the page finished loading, catch up.
  void invoke<CaptureEntry[]>("list_captures").then((captures) => {
    if (captures.length > 0 && stack.length === 0) showCapture(captures[0]);
  });
});
```

- [ ] **Step 4: Type-check and test**

Run: `npm run build && npm test`
Expected: both PASS (no TS errors; stack + existing suites green).

- [ ] **Step 5: Commit**

```bash
git add overlay.html src/overlay/overlay.css src/overlay/main.ts
git commit -m "Render the overlay as a stack of capture panels"
```

---

### Task 5: History window — Copy + Restore buttons and i18n catalogs

**Files:**
- Modify: `src/history/main.ts`
- Modify: `locales/en-GB.json`, `locales/es.json`, `locales/fr.json`, `locales/de.json`, `locales/it.json`

**Interfaces:**
- Consumes: `restore_capture` command from Task 3; existing `copy_capture`.
- Produces: history cards with exactly two buttons; i18n key `history.restore` in all five catalogs; keys `history.annotate`, `history.save`, `history.reveal` removed from all five.

- [ ] **Step 1: Update the catalogs**

In each of the five files, the `history.*` block sits together (lines ~156–161). Replace the four action keys with the two new ones — **keep `history.title` and `history.empty` untouched**. The action lines become:

`locales/en-GB.json`:
```json
  "history.copy": "Copy",
  "history.restore": "Restore",
```

`locales/es.json`:
```json
  "history.copy": "Copiar",
  "history.restore": "Restaurar",
```

`locales/fr.json`:
```json
  "history.copy": "Copier",
  "history.restore": "Restaurer",
```

`locales/de.json`:
```json
  "history.copy": "Kopieren",
  "history.restore": "Wiederherstellen",
```

`locales/it.json`:
```json
  "history.copy": "Copia",
  "history.restore": "Ripristina",
```

(i.e. delete the `history.annotate`, `history.save`, `history.reveal` lines in every file and add `history.restore` after `history.copy`. They are referenced nowhere else — `grep -rn "history\.annotate\|history\.save\|history\.reveal" src src-tauri/src` must come back empty after Step 2.)

- [ ] **Step 2: Update `src/history/main.ts`**

Replace the `actions.append(...)` block (currently four `actionButton` calls) with:

```ts
  actions.append(
    actionButton("history.copy", () => void invoke("copy_capture", { id: entry.id })),
    actionButton("history.restore", () => void invoke("restore_capture", { id: entry.id })),
  );
```

The History window stays open after Restore (no `getCurrentWindow().hide()`).

Then remove the now-unused imports at the top of the file: delete the `getCurrentWindow` import line and the `savePngAs` import line. The remaining imports are:

```ts
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { el } from "../shared/dom";
import { initI18n, t } from "../shared/i18n";
import type { CaptureEntry } from "../shared/ipc";
```

- [ ] **Step 3: Verify**

Run: `grep -rn "history\.annotate\|history\.save\|history\.reveal" src src-tauri/src locales` — expected: no matches.
Run: `npm run build && npm test`
Expected: PASS — the i18n parity test (`src/shared/i18n.test.ts` "catalog parity") proves all five catalogs still share one key set.

- [ ] **Step 4: Commit**

```bash
git add src/history/main.ts locales/en-GB.json locales/es.json locales/fr.json locales/de.json locales/it.json
git commit -m "History cards: Copy + Restore buttons; restore reopens the overlay"
```

---

### Task 6: Full verification gate

**Files:** none (verification only)

- [ ] **Step 1: Run the full gate**

```bash
npm run build && npm test && (cd src-tauri && cargo test)
```
Expected: all three PASS.

- [ ] **Step 2: Manual smoke test (dev app)**

Run: `npm run tauri dev` (note the CLAUDE.md gotcha: make sure no stale `/Applications/Screen for me.app` instance is running — check `pgrep -fl screenforme`). Verify:

1. Take a capture (⌘⇧9) → one panel appears bottom-corner.
2. Take another → second panel stacks on top of the first.
3. Tray → Capture History → cards show exactly Copy + Restore.
4. Restore an old capture → panel appears on top of the stack; History stays open.
5. Restore a capture already in the stack → its panel moves to the top, no duplicate.
6. Click × on a middle panel → only that panel goes; window shrinks.
7. Dismiss all panels → window hides. Next capture shows a single panel.
8. Drag a thumbnail out to Finder → drops the file; with "close after drag" on, only that panel closes.

- [ ] **Step 3: Done**

Report results; no commit needed unless fixes were required.
