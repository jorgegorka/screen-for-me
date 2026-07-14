// One-off continuation of release.mjs for v1.2.2: the build/sign/notarize
// steps already succeeded; this performs only the manifest + publish tail.
import { execSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildLatestJson } from "./latest-json.mjs";

const ROOT = new URL("..", import.meta.url).pathname;
const BUNDLE = join(ROOT, "src-tauri/target/release/bundle");
const REPO = "jorgegorka/screen-for-me";
const version = "1.2.2";
const tag = `v${version}`;

const dmg = join(BUNDLE, "dmg", `Screen_for_me_${version}_aarch64.dmg`);
const tarball = join(BUNDLE, "macos", `Screen_for_me_${version}_aarch64.app.tar.gz`);
const manifestPath = join(BUNDLE, "latest.json");
const assetUrl = `https://github.com/${REPO}/releases/download/${tag}/Screen_for_me_${version}_aarch64.app.tar.gz`;

function run(cmd) {
  console.log(`\n$ ${cmd}`);
  execSync(cmd, { stdio: "inherit", cwd: ROOT });
}

for (const f of [dmg, tarball, `${tarball}.sig`]) {
  if (!existsSync(f)) throw new Error(`missing artifact: ${f}`);
}

run(`xcrun stapler validate "${dmg}"`);

const manifest = buildLatestJson({
  version,
  pubDate: new Date().toISOString(),
  url: assetUrl,
  signature: readFileSync(`${tarball}.sig`, "utf8").trim(),
});
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

run(
  `gh release create ${tag} -R ${REPO} --draft --title "${version}" --generate-notes ` +
    `"${dmg}" "${tarball}" "${manifestPath}"`,
);
run(`gh release edit ${tag} -R ${REPO} --draft=false`);
console.log(`\nreleased ${tag}`);
