import type { Rect } from "./geometry";

/**
 * Pixelate a region of an image at native resolution and return it as a
 * canvas sized to the region. `region` is in natural image pixels.
 */
export function pixelateRegion(
  image: HTMLImageElement,
  region: Rect,
  blockSize = 14,
): HTMLCanvasElement {
  const out = document.createElement("canvas");
  out.width = Math.max(1, Math.round(region.width));
  out.height = Math.max(1, Math.round(region.height));
  const ctx = out.getContext("2d")!;

  // Downscale the region, then upscale without smoothing = mosaic.
  const small = document.createElement("canvas");
  small.width = Math.max(1, Math.ceil(out.width / blockSize));
  small.height = Math.max(1, Math.ceil(out.height / blockSize));
  const smallCtx = small.getContext("2d")!;
  smallCtx.drawImage(
    image,
    region.x,
    region.y,
    region.width,
    region.height,
    0,
    0,
    small.width,
    small.height,
  );

  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(small, 0, 0, out.width, out.height);
  return out;
}
