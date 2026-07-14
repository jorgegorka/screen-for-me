export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Normalize a drag between two points into a positive-size rect. */
export function normalizeRect(x1: number, y1: number, x2: number, y2: number): Rect {
  return {
    x: Math.min(x1, x2),
    y: Math.min(y1, y2),
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
  };
}
