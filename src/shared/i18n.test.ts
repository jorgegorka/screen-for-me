import { afterEach, describe, expect, it } from "vitest";

import { catalogs, setLanguage, t, tn } from "./i18n";

afterEach(() => setLanguage("en-GB"));

describe("t", () => {
  it("returns the active language's string", () => {
    setLanguage("es");
    expect(t("overlay.toast_copied")).toBe("Copiado");
  });

  it("falls back to en-GB for an unknown language tag", () => {
    setLanguage("pt-BR");
    expect(t("overlay.toast_copied")).toBe("Copied");
  });

  it("returns the key itself when no catalog has it", () => {
    expect(t("no.such.key")).toBe("no.such.key");
  });

  it("replaces {name} placeholders", () => {
    expect(t("updates.available", { version: "2.0.0" })).toContain("Version 2.0.0");
  });
});

describe("tn", () => {
  it("uses the singular form for exactly 1", () => {
    expect(tn("scrollcap.frames", 1)).toBe("1 frame");
  });

  it("uses the plural form otherwise", () => {
    expect(tn("scrollcap.frames", 0)).toBe("0 frames");
    expect(tn("scrollcap.frames", 12)).toBe("12 frames");
  });
});

describe("catalog parity", () => {
  const reference = Object.keys(catalogs["en-GB"]).sort();

  it("bundles all five languages", () => {
    expect(Object.keys(catalogs).sort()).toEqual(["de", "en-GB", "es", "fr", "it"]);
  });

  it.each(Object.keys(catalogs))("%s has exactly the en-GB key set", (tag) => {
    expect(Object.keys(catalogs[tag]).sort()).toEqual(reference);
  });

  it("has no empty translations", () => {
    for (const [tag, catalog] of Object.entries(catalogs)) {
      for (const [key, value] of Object.entries(catalog)) {
        expect(value.trim(), `${tag}:${key}`).not.toBe("");
      }
    }
  });
});
