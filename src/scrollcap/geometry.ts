// Pure selection math, DOM-free so vitest covers it (same idea as the
// editor's geometry.ts).

import type { Rect } from "../shared/geometry";

export { normalizeRect, type Rect } from "../shared/geometry";

/** Selections smaller than this are accidental clicks, not scroll areas. */
const MIN_SELECTION = 40;

export function isSelectable(rect: Rect): boolean {
  return rect.width >= MIN_SELECTION && rect.height >= MIN_SELECTION;
}

/**
 * Place the HUD under the rect's bottom-right corner, flipping above when the
 * screen edge is too close, always clamped inside the viewport.
 */
export function hudPosition(
  rect: Rect,
  hudW: number,
  hudH: number,
  viewportW: number,
  viewportH: number,
): { x: number; y: number } {
  const gap = 12;
  let x = Math.min(rect.x + rect.width - hudW, viewportW - hudW - gap);
  x = Math.max(gap, x);
  let y = rect.y + rect.height + gap;
  if (y + hudH > viewportH - gap) {
    y = Math.max(gap, rect.y - hudH - gap);
  }
  return { x, y };
}
