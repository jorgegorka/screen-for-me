# Real update checks for Screen for me

**Date:** 2026-07-14
**Status:** Approved

## Goal

Replace the placeholder updater configuration (fake endpoint `releases.screenforme.example`, throwaway dev pubkey) with a working update pipeline: signed releases published to GitHub, an in-app "Install & Restart" flow, and a silent auto-check on launch. The site at https://screenforme.app stays untouched — publishing a GitHub release is the entire rollout.

## Current state

- `tauri-plugin-updater` is configured in `src-tauri/tauri.conf.json` (`plugins.updater`) with a placeholder endpoint and a dev pubkey whose private key is throwaway.
- `windows.rs::check_for_updates` (triggered by the tray "Check for Updates…" item) checks and reports in a dialog but never downloads or installs.
- GitHub release `v1.2.0` exists on `jorgegorka/screen-for-me` with only the `.dmg` asset — no updater artifacts, no manifest.
- `bundle.createUpdaterArtifacts: true` is already set, so a build with signing env vars produces `.app.tar.gz` + `.sig`.
- Builds are produced locally on the developer's Mac (`npm run bundle`), ad-hoc signed (no Apple Developer certificate).

## Design

### 1. Signing keys

- Generate a production minisign keypair: `npm run tauri signer generate -- -w ~/.tauri/screenforme.key`, password-protected.
- Replace `plugins.updater.pubkey` in `tauri.conf.json` with the new public key.
- The private key never enters the repo. The release script reads `TAURI_SIGNING_PRIVATE_KEY_PATH` (or `TAURI_SIGNING_PRIVATE_KEY`) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from the environment.
- No migration concern: no released build has ever successfully updated against the old key.

### 2. Endpoint

`plugins.updater.endpoints` becomes:

```
https://github.com/jorgegorka/screen-for-me/releases/latest/download/latest.json
```

### 3. Release script (`scripts/release.mjs`, wired as `npm run release`)

1. Verify version consistency across `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`; abort if a `v<version>` tag or GitHub release already exists.
2. Verify the signing env vars are set, then run `tauri build` — produces the `.dmg` plus updater artifact `Screen for me.app.tar.gz` + `.sig`; rename artifacts to underscores (reuse/extend the `rename-bundles.mjs` logic).
3. Generate `latest.json`:
   - `version`, `pub_date` (ISO 8601), `platforms."darwin-aarch64"` with `signature` (contents of the `.sig` file) and `url` pointing at the **versioned** release asset URL (`…/releases/download/v<version>/Screen_for_me_<version>_aarch64.app.tar.gz`), not `latest/`, so old manifests stay valid.
   - Only `darwin-aarch64` for now. An Intel Mac finds no matching platform and reports "up to date" — acceptable.
4. Create the GitHub release (`gh release create v<version>`) and upload `.dmg`, `.app.tar.gz`, and `latest.json`.

### 4. In-app behavior (`src-tauri/src/windows.rs`, `src-tauri/src/lib.rs`)

- `check_for_updates` gains a `silent: bool` parameter.
  - Update found (both modes): native dialog with **Install & Restart** / **Later** buttons. On confirm, `update.download_and_install(...)` then `app.restart()`. Install/verification failure shows a warning dialog (both modes — the user explicitly opted in by clicking Install).
  - Up to date / network error: dialogs only in manual mode (tray item keeps today's behavior); silent mode logs and stays quiet.
- Auto-check: spawned from `setup` in `lib.rs`, first check ~10 s after launch, then every 24 h (menu-bar app is long-running).
- New i18n keys in all five catalogs (`locales/{en-GB,es,fr,de,it}.json`): `updates.install`, `updates.later`, `updates.install_failed` (existing parity tests enforce coverage). No inline strings.

### 5. Error handling

- Manual check: existing `updates.check_failed` / `updates.unreachable` / `updates.latest` dialogs unchanged.
- Silent check: all failures logged (`eprintln!`/log), never surfaced.
- Signature verification failure surfaces as an install error (warning dialog), never installs.
- The updater-replaced `.app` carries no browser quarantine flag, so the ad-hoc-signed app relaunches without a Gatekeeper re-prompt.

### 6. Testing & verification

- `npm run build`, `npm test`, `cd src-tauri && cargo test` (i18n parity tests cover the new keys).
- End-to-end: temporarily set the local version below the released one, run the **packaged** build, and confirm it finds, downloads, signature-verifies, installs, and relaunches from a draft release. First real use: next version bump (e.g. v1.2.1).
- Update the CLAUDE.md "Updates" section to describe the real pipeline (key location, `npm run release`, endpoint).

## Out of scope

- Intel (x86_64) and universal macOS builds; Linux/Windows update channels.
- A Settings toggle to disable auto-check (can be added later).
- Apple Developer signing / notarization.
- CI-based release builds (design keeps `latest.json` generation scriptable so a GitHub Actions producer could be added later).
