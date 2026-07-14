# "Launch on start" Setting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Launch on start" checkbox to the Settings window that registers the app to launch at login via `tauri-plugin-autostart`.

**Architecture:** The OS login-item registration is the single source of truth — no new field in `Settings`/settings.json. The settings webview reads the state with the plugin's `isEnabled()` on load and toggles it with `enable()`/`disable()`. Rust side only registers the plugin and grants capability permissions.

**Tech Stack:** Tauri v2, `tauri-plugin-autostart` (Rust) + `@tauri-apps/plugin-autostart` (JS), vanilla TypeScript settings page (`src/settings/`).

**Spec:** `docs/superpowers/specs/2026-07-14-launch-on-start-design.md`

## Global Constraints

- macOS launcher mode: `MacosLauncher::LaunchAgent`, no extra launch args.
- Default state: unregistered (off).
- Checkbox label copy: exactly `Launch on start`; row key copy: exactly `General:`.
- The checkbox must NOT go through `readForm`/`fillForm`/`set_settings` — it is not part of `Settings`.
- Before calling the change done: `npm run build`, `npm test`, and `cd src-tauri && cargo test` must pass.

---

### Task 1: Register the autostart plugin (Rust + capabilities)

**Files:**
- Modify: `src-tauri/Cargo.toml` (desktop-only dependencies section, lines 31-33)
- Modify: `src-tauri/src/lib.rs` (plugin registration chain, lines 18-23)
- Modify: `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: the `autostart` plugin commands (`enable`, `disable`, `is_enabled`) callable from any window's JS via `@tauri-apps/plugin-autostart` (used by Task 2).

- [ ] **Step 1: Add the Rust dependency**

In `src-tauri/Cargo.toml`, add to the existing desktop-only section (autostart is a desktop-only plugin, same as global-shortcut/updater):

```toml
[target.'cfg(not(any(target_os = "android", target_os = "ios")))'.dependencies]
tauri-plugin-global-shortcut = "2"
tauri-plugin-updater = "2"
tauri-plugin-autostart = "2"
```

- [ ] **Step 2: Register the plugin in lib.rs**

In `src-tauri/src/lib.rs`, add to the builder chain next to the other plugins (order among plugins doesn't matter):

```rust
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
```

- [ ] **Step 3: Grant capability permissions**

In `src-tauri/capabilities/default.json`, extend `permissions`:

```json
  "permissions": [
    "core:default",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-close",
    "dialog:default",
    "drag:default",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled"
  ]
```

- [ ] **Step 4: Verify the Rust side compiles and tests pass**

Run: `cd src-tauri && cargo test`
Expected: compiles (cargo fetches the new crate) and all existing tests PASS. A wrong permission identifier fails right here at build time — the capability schema is checked by `tauri-build`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "Register tauri-plugin-autostart with LaunchAgent mode"
```

---

### Task 2: Settings UI checkbox wired to the plugin

**Files:**
- Modify: `package.json` / `package-lock.json` (npm install)
- Modify: `index.html` (settings window markup)
- Modify: `src/settings/main.ts`

**Interfaces:**
- Consumes: the `autostart` plugin commands registered in Task 1, via `isEnabled()`/`enable()`/`disable()` from `@tauri-apps/plugin-autostart`.
- Produces: nothing consumed by later tasks (final task).

- [ ] **Step 1: Install the JS guest bindings**

Run: `npm install @tauri-apps/plugin-autostart`
Expected: adds `"@tauri-apps/plugin-autostart": "^2..."` to `package.json` dependencies.

- [ ] **Step 2: Add the General row to the settings markup**

In `index.html`, insert a new section at the top of `<main>`, before the existing "Position on screen:" section, separated by the existing `<hr />` divider style:

```html
    <main>
      <section>
        <div class="row">
          <span class="key">General:</span>
          <div class="value">
            <label><input type="checkbox" id="launch-on-start" /> Launch on start</label>
          </div>
        </div>
      </section>
      <hr />
      <section>
        <div class="row">
          <label class="key" for="position">Position on screen:</label>
          ...existing content unchanged...
```

- [ ] **Step 3: Wire the checkbox in `src/settings/main.ts`**

The generic change-listener loop currently binds `set_settings` saves to every `select, input`; the autostart checkbox must be excluded from it (it is not part of `Settings`). Add the import, a dedicated wiring function, and narrow the generic selector:

```ts
import { invoke } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";

import { el } from "../shared/dom";
import type { Settings } from "../shared/ipc";
```

Add below `syncAutoCloseState`:

```ts
/** Launch-on-start reflects the real OS login-item state, not settings.json. */
async function initAutostart() {
  const box = el<HTMLInputElement>("launch-on-start");
  try {
    box.checked = await isEnabled();
  } catch (err) {
    console.error("autostart state unavailable", err);
    box.disabled = true;
    return;
  }
  box.addEventListener("change", async () => {
    try {
      if (box.checked) await enable();
      else await disable();
    } catch (err) {
      console.error("failed to toggle autostart", err);
      box.checked = !box.checked;
    }
  });
}
```

Replace the `DOMContentLoaded` handler's selector so the generic settings plumbing skips the new checkbox, and kick off the autostart init:

```ts
window.addEventListener("DOMContentLoaded", async () => {
  void initAutostart();
  fillForm(await invoke<Settings>("get_settings"));
  document
    .querySelectorAll<HTMLElement>("select, input:not(#launch-on-start)")
    .forEach((control) => {
      control.addEventListener("change", async () => {
        syncAutoCloseState();
        fillForm(await invoke<Settings>("set_settings", { settings: readForm() }));
      });
    });
});
```

- [ ] **Step 4: Verify frontend type-checks and tests pass**

Run: `npm run build`
Expected: tsc + vite build succeed (a typo in the plugin import or element id that `el()` can't find at compile time fails here).

Run: `npm test`
Expected: all existing vitest suites PASS (no editor modules touched).

- [ ] **Step 5: Manual verification in the running app**

Run: `npm run tauri dev`, open Settings from the tray.
Expected:
- A "General: ☐ Launch on start" row appears at the top, unchecked by default.
- Checking it creates `~/Library/LaunchAgents/com.screenforme.app.plist` (verify: `ls ~/Library/LaunchAgents/ | grep screenforme`).
- Unchecking removes the file.
- Check it, delete the plist by hand, close and reopen Settings: the checkbox shows unchecked.
- Toggling it does NOT rewrite `settings.json` (its mtime stays put).

Note: in dev the LaunchAgent points at the dev binary; that's expected and fine.

- [ ] **Step 6: Commit**

```bash
git add package.json package-lock.json index.html src/settings/main.ts
git commit -m "Add Launch on start checkbox backed by OS login-item state"
```
