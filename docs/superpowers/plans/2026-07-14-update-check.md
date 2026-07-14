# Real Update Checks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make "Check for Updates" real: notarized builds published to GitHub Releases with a signed updater manifest, an in-app Install & Restart flow, and a silent daily auto-check.

**Architecture:** The tauri-plugin-updater endpoint points at `https://github.com/jorgegorka/screen-for-me/releases/latest/download/latest.json`. A local release script (`npm run release`) builds a Developer ID-signed, notarized bundle with minisign updater artifacts, generates `latest.json`, and uploads everything to a GitHub release. In-app, `windows.rs::check_for_updates` gains a `silent` flag and an install-and-restart dialog; `lib.rs` spawns a silent check 10 s after launch and every 24 h.

**Tech Stack:** Tauri v2 (tauri-plugin-updater 2, tauri-plugin-dialog 2), Rust, Node ESM scripts, vitest, `gh` CLI, minisign (via `tauri signer`), Apple notarytool (via `tauri build`).

**Spec:** `docs/superpowers/specs/2026-07-14-update-check-design.md`

## Global Constraints

- All user-facing strings go in ALL FIVE catalogs `locales/{en-GB,es,fr,de,it}.json` — never inline. Parity is enforced by existing tests (`src/shared/i18n.test.ts` and tests in `src-tauri/src/i18n.rs`).
- Before calling any task done: `npm run build`, `npm test`, `cd src-tauri && cargo test` must pass.
- Only `darwin-aarch64` is published; no Intel/Linux/Windows channels.
- Secrets (minisign private key, Apple credentials) never enter the repo. They live in `~/.tauri/screenforme.key` and `~/.screenforme-release.env`.
- Repo: `jorgegorka/screen-for-me`. Apple team: `X665SZW588`. Signing identity: `Developer ID Application: Jorge Alvarez (X665SZW588)`.
- Commit messages end with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.

---

### Task 1: i18n strings for the install flow

**Files:**
- Modify: `locales/en-GB.json`, `locales/es.json`, `locales/fr.json`, `locales/de.json`, `locales/it.json`

**Interfaces:**
- Produces: i18n keys `updates.available` (reworded), `updates.install`, `updates.later`, `updates.install_failed` — consumed by Task 2 via `crate::i18n::t` / `t_with`.

- [ ] **Step 1: Update en-GB catalog**

In `locales/en-GB.json`, the `updates.*` block currently reads (around line 27):

```json
  "updates.title": "Check for Updates",
  "updates.available_title": "Update Available",
  "updates.available": "Version {version} is available. Download and install it from the Screen for me website.",
  "updates.latest": "You're on the latest version.",
  "updates.unreachable": "Couldn't reach the update server. Please try again later.\n\n({err})",
  "updates.check_failed": "Could not check for updates:\n{err}",
```

Change `updates.available` and add three keys directly after it:

```json
  "updates.available": "Version {version} is available. Would you like to install it now?",
  "updates.install": "Install & Restart",
  "updates.later": "Later",
  "updates.install_failed": "The update could not be installed:\n{err}",
```

- [ ] **Step 2: Update the four other catalogs**

Apply the same change to each file, next to its existing `updates.available` key. **Match the register (formal/informal address) the surrounding strings in each catalog already use** — adjust the suggested wording below if the existing catalog differs:

`locales/es.json`:
```json
  "updates.available": "La versión {version} está disponible. ¿Quieres instalarla ahora?",
  "updates.install": "Instalar y reiniciar",
  "updates.later": "Más tarde",
  "updates.install_failed": "No se pudo instalar la actualización:\n{err}",
```

`locales/fr.json`:
```json
  "updates.available": "La version {version} est disponible. Voulez-vous l'installer maintenant ?",
  "updates.install": "Installer et redémarrer",
  "updates.later": "Plus tard",
  "updates.install_failed": "L'installation de la mise à jour a échoué :\n{err}",
```

`locales/de.json`:
```json
  "updates.available": "Version {version} ist verfügbar. Möchten Sie sie jetzt installieren?",
  "updates.install": "Installieren und neu starten",
  "updates.later": "Später",
  "updates.install_failed": "Das Update konnte nicht installiert werden:\n{err}",
```

`locales/it.json`:
```json
  "updates.available": "La versione {version} è disponibile. Vuoi installarla adesso?",
  "updates.install": "Installa e riavvia",
  "updates.later": "Più tardi",
  "updates.install_failed": "Impossibile installare l'aggiornamento:\n{err}",
```

- [ ] **Step 3: Run parity tests**

Run: `npm test` and `cd src-tauri && cargo test`
Expected: PASS (the catalog-parity tests confirm every key exists in all five files; a typo in any file fails here).

- [ ] **Step 4: Commit**

```bash
git add locales/
git commit -m "Add install-flow update strings to all catalogs"
```

---

### Task 2: Install & Restart flow in `check_for_updates`

**Files:**
- Modify: `src-tauri/src/windows.rs:130-182` (the `check_for_updates` function)
- Modify: `src-tauri/src/tray.rs:118` (call site)

**Interfaces:**
- Consumes: i18n keys from Task 1.
- Produces: `pub fn check_for_updates(app: &AppHandle, silent: bool)` — Task 3 calls it with `silent = true`; the tray calls it with `silent = false`.

- [ ] **Step 1: Replace `check_for_updates` in `src-tauri/src/windows.rs`**

Replace the whole function (lines 130–182, doc comment included) with:

```rust
/// Check for updates against the GitHub Releases manifest. In `silent` mode
/// (launch/daily auto-check) "up to date" and network errors produce no UI —
/// only an actual update shows the install prompt. The manual tray item
/// (`silent = false`) reports every outcome in a dialog.
pub fn check_for_updates(app: &AppHandle, silent: bool) {
    use tauri_plugin_updater::UpdaterExt;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let dialog = app.dialog().clone();
        let result = match app.updater() {
            Ok(updater) => updater.check().await,
            Err(err) => {
                if silent {
                    eprintln!("update auto-check failed: {err}");
                    return;
                }
                dialog
                    .message(crate::i18n::t_with(
                        "updates.check_failed",
                        &[("err", &err.to_string())],
                    ))
                    .title(crate::i18n::t("updates.title"))
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
                return;
            }
        };
        match result {
            Ok(Some(update)) => prompt_and_install(app, update),
            Ok(None) => {
                if !silent {
                    dialog
                        .message(crate::i18n::t("updates.latest"))
                        .title(crate::i18n::t("updates.title"))
                        .kind(MessageDialogKind::Info)
                        .show(|_| {});
                }
            }
            Err(err) => {
                if silent {
                    eprintln!("update auto-check failed: {err}");
                    return;
                }
                dialog
                    .message(crate::i18n::t_with(
                        "updates.unreachable",
                        &[("err", &err.to_string())],
                    ))
                    .title(crate::i18n::t("updates.title"))
                    .kind(MessageDialogKind::Warning)
                    .show(|_| {});
            }
        }
    });
}

/// Offer to install a found update; on confirmation download it, verify the
/// minisign signature (done by the plugin), swap the .app and relaunch.
fn prompt_and_install(app: AppHandle, update: tauri_plugin_updater::Update) {
    use tauri_plugin_dialog::MessageDialogButtons;
    app.dialog()
        .message(crate::i18n::t_with(
            "updates.available",
            &[("version", &update.version)],
        ))
        .title(crate::i18n::t("updates.available_title"))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            crate::i18n::t("updates.install"),
            crate::i18n::t("updates.later"),
        ))
        .show(move |install| {
            if !install {
                return;
            }
            tauri::async_runtime::spawn(async move {
                match update.download_and_install(|_, _| {}, || {}).await {
                    Ok(()) => app.restart(),
                    Err(err) => {
                        app.dialog()
                            .message(crate::i18n::t_with(
                                "updates.install_failed",
                                &[("err", &err.to_string())],
                            ))
                            .title(crate::i18n::t("updates.title"))
                            .kind(MessageDialogKind::Warning)
                            .show(|_| {});
                    }
                }
            });
        });
}
```

- [ ] **Step 2: Update the tray call site**

In `src-tauri/src/tray.rs:118` change:

```rust
            "updates" => windows::check_for_updates(app),
```

to:

```rust
            "updates" => windows::check_for_updates(app, false),
```

- [ ] **Step 3: Build and test**

Run: `cd src-tauri && cargo build && cargo test`
Expected: compiles, all tests PASS. If `MessageDialogButtons` fails to resolve, it lives in `tauri_plugin_dialog` (already a dependency, version 2 — check `MessageDialogButtons::OkCancelCustom(String, String)` exists in the installed version with `cargo doc` or the source in `~/.cargo/registry`; do not bump plugin versions without need).

- [ ] **Step 4: Manual smoke check (dev)**

Run `npm run tauri dev`, open the tray → "Check for Updates…". Expected with the still-placeholder endpoint: the "couldn't reach the update server" warning dialog (i.e. behavior unchanged for the manual path). Quit the dev app afterwards.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/windows.rs src-tauri/src/tray.rs
git commit -m "Offer Install & Restart when an update is found"
```

---

### Task 3: Silent auto-check on launch + every 24 h

**Files:**
- Modify: `src-tauri/Cargo.toml` (add tokio `time` feature)
- Modify: `src-tauri/src/lib.rs` (spawn the check in `setup`)

**Interfaces:**
- Consumes: `windows::check_for_updates(&AppHandle, silent: bool)` from Task 2.

- [ ] **Step 1: Add tokio dependency**

In `src-tauri/Cargo.toml` under `[dependencies]` (tauri already depends on tokio; this only makes the `time` API directly usable):

```toml
tokio = { version = "1", features = ["time"] }
```

- [ ] **Step 2: Spawn the auto-check in `setup`**

In `src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, directly before the final `Ok(())` (after the `main` window retitle block at lines 72–76), add:

```rust
            // Auto-check for updates: shortly after launch, then daily (a
            // menu-bar app runs for weeks). Silent — only an actual update
            // shows UI. Release builds only: dev builds run version 1.x too
            // and would nag against the published releases.
            #[cfg(not(debug_assertions))]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    loop {
                        windows::check_for_updates(&handle, true);
                        tokio::time::sleep(std::time::Duration::from_secs(60 * 60 * 24))
                            .await;
                    }
                });
            }
```

- [ ] **Step 3: Build both profiles and test**

Run: `cd src-tauri && cargo build && cargo build --release && cargo test`
Expected: both compile (the release build exercises the `cfg(not(debug_assertions))` block), tests PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs
git commit -m "Silently auto-check for updates on launch and daily"
```

---

### Task 4: `latest.json` manifest builder (TDD)

**Files:**
- Create: `scripts/latest-json.mjs`
- Test: `scripts/latest-json.test.mjs`

**Interfaces:**
- Produces: `buildLatestJson({ version, pubDate, url, signature })` → plain object in Tauri's static-manifest shape. Consumed by Task 5.

- [ ] **Step 1: Write the failing test**

Create `scripts/latest-json.test.mjs`:

```js
import { describe, expect, it } from "vitest";
import { buildLatestJson } from "./latest-json.mjs";

describe("buildLatestJson", () => {
  const input = {
    version: "1.3.0",
    pubDate: "2026-07-14T12:00:00.000Z",
    url: "https://github.com/jorgegorka/screen-for-me/releases/download/v1.3.0/Screen_for_me_1.3.0_aarch64.app.tar.gz",
    signature: "dW50cnVzdGVkIGNvbW1lbnQ...",
  };

  it("produces Tauri's static manifest shape", () => {
    expect(buildLatestJson(input)).toEqual({
      version: "1.3.0",
      pub_date: "2026-07-14T12:00:00.000Z",
      platforms: {
        "darwin-aarch64": {
          signature: "dW50cnVzdGVkIGNvbW1lbnQ...",
          url: "https://github.com/jorgegorka/screen-for-me/releases/download/v1.3.0/Screen_for_me_1.3.0_aarch64.app.tar.gz",
        },
      },
    });
  });

  it("rejects a malformed version", () => {
    expect(() => buildLatestJson({ ...input, version: "v1.3.0" })).toThrow(/version/);
  });

  it("rejects an empty signature", () => {
    expect(() => buildLatestJson({ ...input, signature: "" })).toThrow(/signature/);
  });

  it("rejects a non-https url", () => {
    expect(() => buildLatestJson({ ...input, url: "http://example.com/x.tar.gz" })).toThrow(/url/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run scripts/latest-json.test.mjs`
Expected: FAIL — cannot resolve `./latest-json.mjs`.

- [ ] **Step 3: Write the implementation**

Create `scripts/latest-json.mjs`:

```js
// Build the tauri-plugin-updater static manifest (latest.json) for a release.
// Kept pure (no fs/network) so it stays unit-testable.
export function buildLatestJson({ version, pubDate, url, signature }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`invalid version "${version}" — expected e.g. 1.3.0 (no leading v)`);
  }
  if (!signature) throw new Error("missing signature (.sig file contents)");
  if (!/^https:\/\//.test(url)) throw new Error(`url must be https: ${url}`);
  return {
    version,
    pub_date: pubDate,
    platforms: {
      "darwin-aarch64": { signature, url },
    },
  };
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npx vitest run scripts/latest-json.test.mjs` then the full `npm test`
Expected: PASS (and no other suites broken).

- [ ] **Step 5: Commit**

```bash
git add scripts/latest-json.mjs scripts/latest-json.test.mjs
git commit -m "Add latest.json manifest builder"
```

---

### Task 5: Release script (`npm run release`)

**Files:**
- Create: `scripts/release.mjs`
- Modify: `package.json` (add `"release"` script)

**Interfaces:**
- Consumes: `buildLatestJson` from Task 4; `scripts/rename-bundles.mjs` (run as a child process).
- Produces: `npm run release` (full pipeline) and `npm run release -- --dry-run` (validate env/versions/artifact names only, no build/upload).

- [ ] **Step 1: Write `scripts/release.mjs`**

```js
#!/usr/bin/env node
// Build, sign, notarize and publish a release to GitHub Releases.
//
//   npm run release              full pipeline
//   npm run release -- --dry-run validate env + versions, print the plan, exit
//
// Secrets come from the environment or ~/.screenforme-release.env (untracked):
//   TAURI_SIGNING_PRIVATE_KEY           path to ~/.tauri/screenforme.key
//   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  its password
//   APPLE_SIGNING_IDENTITY              "Developer ID Application: Jorge Alvarez (X665SZW588)"
//   APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID   notarization (app-specific password)
import { execSync } from "node:child_process";
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { buildLatestJson } from "./latest-json.mjs";

const ROOT = new URL("..", import.meta.url).pathname;
const BUNDLE = join(ROOT, "src-tauri/target/release/bundle");
const REPO = "jorgegorka/screen-for-me";
const dryRun = process.argv.includes("--dry-run");

function fail(msg) {
  console.error(`release: ${msg}`);
  process.exit(1);
}

function run(cmd) {
  console.log(`\n$ ${cmd}`);
  execSync(cmd, { stdio: "inherit", cwd: ROOT });
}

// --- secrets: environment wins over the env file -------------------------
const envFile = join(homedir(), ".screenforme-release.env");
if (existsSync(envFile)) {
  for (const line of readFileSync(envFile, "utf8").split("\n")) {
    const m = line.match(/^(?:export\s+)?([A-Z_]+)=(.*)$/);
    if (m && !(m[1] in process.env)) {
      process.env[m[1]] = m[2].replace(/^["']|["']$/g, "");
    }
  }
}
const REQUIRED = [
  "TAURI_SIGNING_PRIVATE_KEY",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
];
const missing = REQUIRED.filter((k) => !process.env[k]);
if (missing.length) {
  fail(`missing env vars: ${missing.join(", ")} (set them in ${envFile})`);
}

// --- version consistency ---------------------------------------------------
const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
const conf = JSON.parse(
  readFileSync(join(ROOT, "src-tauri/tauri.conf.json"), "utf8"),
);
const cargo = readFileSync(join(ROOT, "src-tauri/Cargo.toml"), "utf8");
const cargoVersion = cargo.match(/^version = "([^"]+)"/m)?.[1];
const version = pkg.version;
if (conf.version !== version || cargoVersion !== version) {
  fail(
    `version mismatch: package.json=${version} tauri.conf.json=${conf.version} Cargo.toml=${cargoVersion}`,
  );
}
const tag = `v${version}`;

// --- release must not exist yet ---------------------------------------------
let releaseExists = true;
try {
  execSync(`gh release view ${tag} -R ${REPO}`, { stdio: "ignore" });
} catch {
  releaseExists = false;
}
if (releaseExists) fail(`release ${tag} already exists on ${REPO}`);

// --- artifact paths ----------------------------------------------------------
const dmg = join(BUNDLE, "dmg", `Screen_for_me_${version}_aarch64.dmg`);
const rawTarball = join(BUNDLE, "macos", "Screen_for_me.app.tar.gz");
const tarball = join(
  BUNDLE,
  "macos",
  `Screen_for_me_${version}_aarch64.app.tar.gz`,
);
const appPath = join(BUNDLE, "macos", "Screen for me.app");
const manifestPath = join(BUNDLE, "latest.json");
const assetUrl = `https://github.com/${REPO}/releases/download/${tag}/Screen_for_me_${version}_aarch64.app.tar.gz`;

if (dryRun) {
  console.log(`release ${tag} — dry run OK`);
  console.log(`  would build, notarize, then upload:`);
  console.log(`    ${dmg}`);
  console.log(`    ${tarball}`);
  console.log(`    ${manifestPath} (url: ${assetUrl})`);
  process.exit(0);
}

// --- build (signs + notarizes via APPLE_* env, updater artifacts via
// createUpdaterArtifacts) ------------------------------------------------------
run("npm run tauri build");
run("node scripts/rename-bundles.mjs");

if (existsSync(rawTarball)) renameSync(rawTarball, tarball);
if (existsSync(`${rawTarball}.sig`)) renameSync(`${rawTarball}.sig`, `${tarball}.sig`);
for (const f of [dmg, tarball, `${tarball}.sig`]) {
  if (!existsSync(f)) fail(`expected artifact missing: ${f}`);
}

// --- verify signing/notarization actually took -----------------------------
run(`codesign --verify --deep --strict "${appPath}"`);
run(`spctl -a -vv "${appPath}"`);
run(`xcrun stapler validate "${dmg}"`);

// --- manifest ---------------------------------------------------------------
const manifest = buildLatestJson({
  version,
  pubDate: new Date().toISOString(),
  url: assetUrl,
  signature: readFileSync(`${tarball}.sig`, "utf8").trim(),
});
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

// --- publish -----------------------------------------------------------------
run(
  `gh release create ${tag} -R ${REPO} --title "${version}" --generate-notes ` +
    `"${dmg}" "${tarball}" "${manifestPath}"`,
);
console.log(`\nreleased ${tag} — the updater endpoint now serves this build.`);
```

- [ ] **Step 2: Wire it into package.json**

In `package.json` `scripts`, after `"bundle"`:

```json
    "release": "node scripts/release.mjs",
```

- [ ] **Step 3: Verify the dry run fails without secrets**

Run: `npm run release -- --dry-run` (with no env file present yet)
Expected: exits 1 with `missing env vars: TAURI_SIGNING_PRIVATE_KEY, ...`.

- [ ] **Step 4: Verify version-mismatch detection**

Temporarily set fake env vars and confirm the happy dry-run path:

```bash
TAURI_SIGNING_PRIVATE_KEY=x TAURI_SIGNING_PRIVATE_KEY_PASSWORD=x \
APPLE_SIGNING_IDENTITY=x APPLE_ID=x APPLE_PASSWORD=x APPLE_TEAM_ID=x \
npm run release -- --dry-run
```

Expected: `release v1.2.0 — dry run OK` **or** `release v1.2.0 already exists` (correct — v1.2.0 is published; both prove validation runs). Then `npm test` still passes.

- [ ] **Step 5: Commit**

```bash
git add scripts/release.mjs package.json
git commit -m "Add release script: build, notarize, publish with updater manifest"
```

---

### Task 6: Real keys, endpoint, and secrets file (user-interactive)

**Files:**
- Modify: `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`, `plugins.updater.endpoints`)
- Outside repo: `~/.tauri/screenforme.key(.pub)`, `~/.screenforme-release.env`

**Interfaces:**
- Consumes: env-var names defined in Task 5.
- Produces: a working signed-manifest configuration for the packaged app.

⚠️ **This task needs the user.** Key generation prompts for a new password, and the Apple app-specific password is theirs to create. Pause and ask; do not invent secrets.

- [ ] **Step 1: User generates the minisign keypair**

Ask the user to run in their terminal (`!` prefix in the session works):

```bash
npm run tauri signer generate -- -w ~/.tauri/screenforme.key
```

They choose a password when prompted. Confirm afterwards that `~/.tauri/screenforme.key` and `~/.tauri/screenforme.key.pub` exist.

- [ ] **Step 2: User creates the secrets file**

Ask the user to create `~/.screenforme-release.env` (mode 600) with — filling in their two passwords:

```bash
TAURI_SIGNING_PRIVATE_KEY=/Users/jorge/.tauri/screenforme.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=<minisign key password>
APPLE_SIGNING_IDENTITY=Developer ID Application: Jorge Alvarez (X665SZW588)
APPLE_ID=<Apple ID email>
APPLE_PASSWORD=<app-specific password from appleid.apple.com>
APPLE_TEAM_ID=X665SZW588
```

Then `chmod 600 ~/.screenforme-release.env`.

- [ ] **Step 3: Update tauri.conf.json**

Read the new public key (`cat ~/.tauri/screenforme.key.pub` — it's a two-line minisign file; the pubkey value is the whole file base64'd by tauri signer, which prints the exact `pubkey` string to copy at generation time — use the value the generator printed, or `base64 < ~/.tauri/screenforme.key.pub | tr -d '\n'`). In `src-tauri/tauri.conf.json` replace the `plugins.updater` block:

```json
    "updater": {
      "pubkey": "<value from ~/.tauri/screenforme.key.pub as printed by tauri signer>",
      "endpoints": [
        "https://github.com/jorgegorka/screen-for-me/releases/latest/download/latest.json"
      ]
    }
```

- [ ] **Step 4: Validate**

Run: `npm run build && npm test && cd src-tauri && cargo test`, then `npm run release -- --dry-run`
Expected: builds/tests PASS; dry run reports `release v1.2.0 already exists on jorgegorka/screen-for-me` — which proves the env file loaded and validation ran end-to-end.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "Point updater at GitHub Releases with production pubkey"
```

---

### Task 7: Documentation

**Files:**
- Modify: `CLAUDE.md` (the `## Updates` section)

- [ ] **Step 1: Rewrite the Updates section**

Replace the whole `## Updates` section body in `CLAUDE.md` with:

```markdown
## Updates

Releases are published to GitHub Releases on `jorgegorka/screen-for-me`; the
updater endpoint is `https://github.com/jorgegorka/screen-for-me/releases/latest/download/latest.json`
(`plugins.updater` in tauri.conf.json). `npm run release` does everything:
verifies version consistency (package.json / tauri.conf.json / Cargo.toml) and
that the `v<version>` release doesn't exist, builds a Developer ID-signed and
notarized bundle, emits minisign-signed updater artifacts
(`createUpdaterArtifacts`), generates `latest.json`
(`scripts/latest-json.mjs`, unit-tested), and uploads the .dmg, .app.tar.gz
and manifest with `gh`. Secrets live outside the repo in
`~/.screenforme-release.env` (chmod 600): `TAURI_SIGNING_PRIVATE_KEY` (path to
`~/.tauri/screenforme.key`), its `_PASSWORD`, and `APPLE_SIGNING_IDENTITY` /
`APPLE_ID` / `APPLE_PASSWORD` (app-specific) / `APPLE_TEAM_ID` for
notarization. Never commit the private key; the pubkey in tauri.conf.json must
match it or every update check fails signature verification. In-app:
`windows.rs::check_for_updates(app, silent)` — the tray item is the loud path,
and lib.rs auto-checks silently 10 s after launch then daily (release builds
only). Bumping a version means updating package.json, tauri.conf.json **and**
src-tauri/Cargo.toml together (`npm run release` refuses on mismatch).
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "Document the real release/update pipeline"
```

---

### Task 8: End-to-end verification (draft release round-trip)

**Files:** none (procedure only; temporary edits are reverted)

This proves the whole loop — build → notarize → publish → in-app check → download → signature verify → install → relaunch — before trusting it for a real release. It publishes a real `v1.2.1`; that's fine and becomes the first notarized release.

- [ ] **Step 1: Bump to 1.2.1 and release**

Set `version` to `1.2.1` in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`; commit (`git commit -am "Bump version to 1.2.1"`). Then run `npm run release`. Expected: build succeeds, notarization completes (takes a few minutes; watch for `stapler validate` passing), release `v1.2.1` appears with three assets: `Screen_for_me_1.2.1_aarch64.dmg`, `Screen_for_me_1.2.1_aarch64.app.tar.gz`, `latest.json`.

- [ ] **Step 2: Build an older packaged app and update through it**

Temporarily set the version back to `1.2.0` in all three files (do NOT commit), run `npm run bundle`, and launch the built app from `src-tauri/target/release/bundle/macos/Screen for me.app`. Tray → "Check for Updates…". Expected: "Version 1.2.1 is available" dialog with **Install & Restart** / **Later**. Click Install & Restart. Expected: brief pause (download + minisign verification), the app relaunches, and tray → About shows 1.2.1.

- [ ] **Step 3: Restore the working tree**

`git checkout -- package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock` — repo stays at the committed 1.2.1.

- [ ] **Step 4: Verify Gatekeeper on the DMG**

Download `Screen_for_me_1.2.1_aarch64.dmg` from the GitHub release **in a browser** (so it gets the quarantine flag), install to /Applications, launch. Expected: no "unidentified developer" warning. Also remember the CLAUDE.md gotcha: an installed 1.2.1 and a dev instance both register global shortcuts — quit one.

- [ ] **Step 5: Final full check**

Run: `npm run build && npm test && cd src-tauri && cargo test`
Expected: all PASS. Working tree clean (`git status`).
