import type { Rect } from "../shared/geometry";

export type { Rect };

export interface Size {
  width: number;
  height: number;
}

/** Scale that fits an image into a viewport without upscaling past 100%. */
export function fitScale(image: Size, viewport: Size): number {
  if (image.width <= 0 || image.height <= 0) return 1;
  return Math.min(
    1,
    viewport.width / image.width,
    viewport.height / image.height,
  );
}

/**
 * Map an image-space point to stage-container (screen) space, given the stage
 * position (pan, already in screen pixels) and the fit scale.
 */
export function imageToScreen(
  point: { x: number; y: number },
  stagePos: { x: number; y: number },
  scale: number,
): { x: number; y: number } {
  return {
    x: stagePos.x + point.x * scale,
    y: stagePos.y + point.y * scale,
  };
}

/** Clamp a rect to the bounds of an image. */
export function clampRect(rect: Rect, image: Size): Rect {
  const x = Math.max(0, Math.min(rect.x, image.width));
  const y = Math.max(0, Math.min(rect.y, image.height));
  return {
    x,
    y,
    width: Math.min(rect.width, image.width - x),
    height: Math.min(rect.height, image.height - y),
  };
}
