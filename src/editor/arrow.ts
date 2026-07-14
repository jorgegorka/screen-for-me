/**
 * Tapered (Skitch-style) arrow geometry. Konva-free so vitest can run it.
 *
 * The polygon: pointed tail -> shaft widening toward the head -> triangular
 * head. All dimensions derive from strokeWidth but are clamped by a fraction
 * of the arrow's length, so short arrows stay proportionate (the arrow
 * visually "grows" out of the tail while dragging).
 */

/**
 * Filled polygon for an arrow from (x1,y1) to (x2,y2), as a flat [x,y,...]
 * list of 7 vertices: tail+, shaftBase+, headBase+, tip, headBase-,
 * shaftBase-, tail-. Empty for a zero-length arrow.
 */
export function taperedArrowPoints(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  strokeWidth: number,
): number[] {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const len = Math.hypot(dx, dy);
  if (len === 0) return [];
  const ux = dx / len;
  const uy = dy / len;
  const px = -uy;
  const py = ux;

  const headLen = Math.min(6 + strokeWidth * 2.5, len * 0.4);
  const headHalf = Math.min(3 + strokeWidth * 1.25, len * 0.2);
  const shaftHalf = Math.min(strokeWidth * 0.6, headHalf * 0.6);
  const tailHalf = shaftHalf * 0.15;

  const bx = x2 - ux * headLen;
  const by = y2 - uy * headLen;

  return [
    x1 + px * tailHalf, y1 + py * tailHalf,
    bx + px * shaftHalf, by + py * shaftHalf,
    bx + px * headHalf, by + py * headHalf,
    x2, y2,
    bx - px * headHalf, by - py * headHalf,
    bx - px * shaftHalf, by - py * shaftHalf,
    x1 - px * tailHalf, y1 - py * tailHalf,
  ];
}

/** Axis-aligned bounding box of a flat [x,y,...] polygon; zeros if empty. */
export function polygonBounds(points: number[]): {
  x: number;
  y: number;
  width: number;
  height: number;
} {
  if (points.length < 2) return { x: 0, y: 0, width: 0, height: 0 };
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (let i = 0; i < points.length; i += 2) {
    minX = Math.min(minX, points[i]);
    maxX = Math.max(maxX, points[i]);
    minY = Math.min(minY, points[i + 1]);
    maxY = Math.max(maxY, points[i + 1]);
  }
  return { x: minX, y: minY, width: maxX - minX, height: maxY - minY };
}
