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

/** Scale that fills the viewport along one axis (the other overflows and scrolls). */
export function fillScale(image: Size, viewport: Size): number {
  if (image.width <= 0 || image.height <= 0) return 1;
  return Math.min(
    1,
    Math.max(viewport.width / image.width, viewport.height / image.height),
  );
}

/** Zoom bounds for the editor view. */
export const MIN_SCALE = 0.02;
export const MAX_SCALE = 8;

export function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

/**
 * Initial view scale for a freshly loaded capture. Ordinary screenshots are
 * contain-fitted; an extremely elongated image (e.g. a scrolling capture) would
 * contain-fit to an unusably small sliver, so fill one axis instead and let the
 * other scroll.
 */
export function initialScale(image: Size, viewport: Size): number {
  const fit = fitScale(image, viewport);
  const fill = fillScale(image, viewport);
  return fill > fit * 2 ? fill : fit;
}

/**
 * Scale the Fit action should jump to next. Normally the readable smart fit
 * (see `initialScale`); when already there and the image is elongated enough
 * that smart fit crops, a second press toggles to the whole-image contain fit,
 * and a third back.
 */
export function nextFitScale(current: number, image: Size, viewport: Size): number {
  const smart = initialScale(image, viewport);
  const fit = fitScale(image, viewport);
  return Math.abs(current - smart) < 0.001 && smart !== fit ? fit : smart;
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
