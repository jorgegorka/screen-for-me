/**
 * Frontend half of the i18n system. Catalogs are the same flat JSON files the
 * Rust side embeds (locales/*.json), bundled eagerly by Vite. Interpolation is
 * plain `{name}` replacement and plurals are a two-form one/other split — the
 * only plural string in the app ("N frames") works that way in all five
 * supported languages.
 *
 * This module is deliberately free of static Tauri imports so vitest can load
 * it; `initI18n` pulls them in dynamically at runtime.
 */

const FALLBACK = "en-GB";

const modules = import.meta.glob<Record<string, string>>("/locales/*.json", {
  eager: true,
  import: "default",
});

export const catalogs: Record<string, Record<string, string>> = Object.fromEntries(
  Object.entries(modules).map(([path, catalog]) => [
    path.replace(/^\/locales\//, "").replace(/\.json$/, ""),
    catalog,
  ]),
);

let activeTag = FALLBACK;
let active = catalogs[FALLBACK];

/** Switch the active catalog; unknown tags fall back to en-GB. */
export function setLanguage(tag: string) {
  activeTag = tag in catalogs ? tag : FALLBACK;
  active = catalogs[activeTag];
}

/** Translated string for `key` (active → en-GB → the key itself), with
 * `{name}` placeholders replaced from `args`. */
export function t(key: string, args?: Record<string, string | number>): string {
  let text = active[key] ?? catalogs[FALLBACK][key] ?? key;
  for (const [name, value] of Object.entries(args ?? {})) {
    text = text.split(`{${name}}`).join(String(value));
  }
  return text;
}

/** Two-form plural lookup: `key.one` when n === 1, else `key.other`. */
export function tn(key: string, n: number): string {
  return t(`${key}.${n === 1 ? "one" : "other"}`, { n });
}

/**
 * Re-translate every annotated element under `root`:
 *   data-i18n        → textContent
 *   data-i18n-title  → title attribute
 *   data-i18n-alt    → alt attribute
 * Keys live in the attributes (never in the content), so this is idempotent
 * and safe to re-run on a live language switch.
 */
export function applyTranslations(root: ParentNode = document) {
  for (const node of root.querySelectorAll<HTMLElement>("[data-i18n]")) {
    node.textContent = t(node.getAttribute("data-i18n")!);
  }
  for (const node of root.querySelectorAll<HTMLElement>("[data-i18n-title]")) {
    node.setAttribute("title", t(node.getAttribute("data-i18n-title")!));
  }
  for (const node of root.querySelectorAll<HTMLElement>("[data-i18n-alt]")) {
    node.setAttribute("alt", t(node.getAttribute("data-i18n-alt")!));
  }
  if (root === document) {
    document.documentElement.lang = activeTag;
  }
}

/**
 * Resolve the app language from the backend, translate the page, and keep it
 * translated across live language changes. Call first thing on
 * DOMContentLoaded in every window.
 */
export async function initI18n(): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  setLanguage(await invoke<string>("resolved_language"));
  applyTranslations();
  void listen("settings:changed", async () => {
    setLanguage(await invoke<string>("resolved_language"));
    applyTranslations();
  });
}
