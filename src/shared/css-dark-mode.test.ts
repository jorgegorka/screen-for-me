import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// Dark-mode @media blocks add no specificity, so any light-mode rule that
// appears LATER in the file silently wins over the dark override (this is how
// the Settings shortcut buttons ended up white-on-dark). Enforce the
// convention: every `prefers-color-scheme: dark` block must come after all
// non-media rules in its file, and must not paper over ordering bugs with
// `!important`.

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

function cssFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return cssFiles(path);
    return name.endsWith(".css") ? [path] : [];
  });
}

interface Block {
  start: number;
  dark: boolean;
}

/** Top-level blocks ({...} at nesting depth 0) with their file offsets. */
function topLevelBlocks(css: string): Block[] {
  const noComments = css.replace(/\/\*[\s\S]*?\*\//g, (m) => " ".repeat(m.length));
  const blocks: Block[] = [];
  let depth = 0;
  let preludeStart = 0;
  for (let i = 0; i < noComments.length; i++) {
    const ch = noComments[i];
    if (ch === "{") {
      if (depth === 0) {
        const prelude = noComments.slice(preludeStart, i);
        blocks.push({
          start: preludeStart,
          dark: /@media[^{]*prefers-color-scheme:\s*dark/.test(prelude),
        });
      }
      depth++;
    } else if (ch === "}") {
      depth--;
      if (depth === 0) preludeStart = i + 1;
    }
  }
  return blocks;
}

describe("dark-mode CSS ordering", () => {
  const themed = cssFiles(SRC)
    .map((file) => [file.slice(SRC.length + 1), readFileSync(file, "utf8")] as const)
    .filter(([, css]) => css.includes("prefers-color-scheme: dark"));

  it("finds the themed stylesheets", () => {
    expect(themed.length).toBeGreaterThan(0);
  });

  for (const [file, css] of themed) {
    describe(file, () => {
      it("declares dark-mode blocks after every light-mode rule", () => {
        const blocks = topLevelBlocks(css);
        const firstDark = blocks.findIndex((b) => b.dark);
        const lastLight = blocks.map((b) => b.dark).lastIndexOf(false);
        expect(firstDark).toBeGreaterThan(lastLight);
      });

      it("does not use !important inside dark-mode blocks", () => {
        const darkChunks = css.match(
          /@media[^{]*prefers-color-scheme:\s*dark[^{]*\{[\s\S]*?\n\}/g,
        );
        for (const chunk of darkChunks ?? []) {
          expect(chunk).not.toContain("!important");
        }
      });
    });
  }
});
