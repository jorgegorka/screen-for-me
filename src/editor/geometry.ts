export interface Size {
  width: number;
  height: number;
}

export interface Rect {
  x: number;
  y: number;
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
 * Parameters for Konva `stage.toDataURL` that produce a native-resolution
 * export. The stage renders the image at `scale`; crop is in image pixels.
 */
export function exportParams(image: Size, scale: number, crop?: Rect | null) {
  const region = crop ?? { x: 0, y: 0, ...image };
  return {
    x: region.x * scale,
    y: region.y * scale,
    width: region.width * scale,
    height: region.height * scale,
    pixelRatio: 1 / scale,
  };
}

/** Normalize a drag between two points into a positive-size rect. */
export function dragRect(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): Rect {
  return {
    x: Math.min(x1, x2),
    y: Math.min(y1, y2),
    width: Math.abs(x2 - x1),
    height: Math.abs(y2 - y1),
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
