// Rename bundle artifacts (.dmg, updater .tar.gz/.sig) to use underscores
// instead of spaces: "Screen for me_1.1.0_aarch64.dmg" →
// "Screen_for_me_1.1.0_aarch64.dmg". Tauri derives these names from
// productName and offers no filename override, and the "Screen for me.app"
// bundle itself must keep its spaces (it's the user-visible app name), so
// only files are renamed. Run via `npm run bundle`.
import { readdirSync, renameSync } from "node:fs";
import { join } from "node:path";

const BUNDLE_DIR = new URL("../src-tauri/target/release/bundle", import.meta.url)
  .pathname;

function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory() && !entry.name.endsWith(".app")) {
      walk(path);
    } else if (entry.isFile() && entry.name.includes(" ")) {
      const renamed = join(dir, entry.name.replaceAll(" ", "_"));
      renameSync(path, renamed);
      console.log(`renamed: ${entry.name} → ${entry.name.replaceAll(" ", "_")}`);
    }
  }
}

try {
  walk(BUNDLE_DIR);
} catch (err) {
  if (err.code === "ENOENT") {
    console.error(`no bundle directory at ${BUNDLE_DIR} — run tauri build first`);
    process.exit(1);
  }
  throw err;
}
