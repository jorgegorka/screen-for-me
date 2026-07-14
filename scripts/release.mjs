#!/usr/bin/env node
// Build, sign, notarize and publish a release to GitHub Releases.
//
//   npm run release              full pipeline
//   npm run release -- --dry-run validate env + versions, print the plan, exit
//
// Secrets come from the shell environment:
//   TAURI_SIGNING_PRIVATE_KEY           path to ~/.tauri/screenforme.key
//   TAURI_SIGNING_PRIVATE_KEY_PASSWORD  its password
//   APPLE_SIGNING_IDENTITY              "Developer ID Application: Jorge Alvarez (X665SZW588)"
//   APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID   notarization (app-specific password)
import { execSync } from "node:child_process";
import { existsSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
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

// --- secrets: required environment variables -------------------------------
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
  fail(`missing env vars: ${missing.join(", ")} (export them in your shell)`);
}

// --- version consistency ---------------------------------------------------
const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
const conf = JSON.parse(
  readFileSync(join(ROOT, "src-tauri/tauri.conf.json"), "utf8"),
);
const cargo = readFileSync(join(ROOT, "src-tauri/Cargo.toml"), "utf8");
const cargoPackageSection = cargo
  .slice(cargo.indexOf("[package]"))
  .split(/\n\[/)[0];
const cargoVersion = cargoPackageSection.match(/^version = "([^"]+)"/m)?.[1];
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
const assetName = basename(tarball);
const assetUrl = `https://github.com/${REPO}/releases/download/${tag}/${assetName}`;

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

// --- publish -------------------------------------------------------------
// Create as a draft first and only flip it public after every asset has
// uploaded successfully — drafts are invisible to the releases/latest/download
// endpoint, so a failure mid-upload leaves an unpublished draft instead of a
// broken published release.
try {
  run(
    `gh release create ${tag} -R ${REPO} --draft --title "${version}" --generate-notes ` +
      `"${dmg}" "${tarball}" "${manifestPath}"`,
  );
} catch (err) {
  fail(
    `release creation/upload failed — a draft release ${tag} may exist on ${REPO}; ` +
      `delete it with "gh release delete ${tag} -R ${REPO}" before retrying. (${err.message})`,
  );
}
run(`gh release edit ${tag} -R ${REPO} --draft=false`);
console.log(`\nreleased ${tag} — the updater endpoint now serves this build.`);
