// Pure selection math, DOM-free so vitest covers it (same idea as the
// editor's geometry.ts).

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Selections smaller than this are accidental clicks, not scroll areas. */
export const MIN_SELECTION = 40;

export function normalizeRect(x1: number, y1: number, x2: number, y2: number): Rect {
  return {
    x: Math.min(x1, x2),
    y: Math.min(y1, y2),
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
  };
}

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
