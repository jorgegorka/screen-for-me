import { describe, expect, it } from "vitest";
import { clampRect, dragRect, exportParams, fitScale } from "./geometry";

describe("fitScale", () => {
  it("never upscales past 100%", () => {
    expect(fitScale({ width: 400, height: 300 }, { width: 2000, height: 2000 })).toBe(1);
  });

  it("fits the constraining dimension", () => {
    expect(fitScale({ width: 2000, height: 1000 }, { width: 1000, height: 1000 })).toBe(0.5);
    expect(fitScale({ width: 1000, height: 2000 }, { width: 1000, height: 500 })).toBe(0.25);
  });
});

describe("exportParams", () => {
  const image = { width: 2000, height: 1000 };

  it("round-trips to native resolution without a crop", () => {
    const p = exportParams(image, 0.5);
    expect(p).toEqual({ x: 0, y: 0, width: 1000, height: 500, pixelRatio: 2 });
    // effective output = stage-space size * pixelRatio = native pixels
    expect(p.width * p.pixelRatio).toBe(image.width);
    expect(p.height * p.pixelRatio).toBe(image.height);
  });

  it("maps an image-space crop into stage space", () => {
    const p = exportParams(image, 0.5, { x: 100, y: 50, width: 800, height: 400 });
    expect(p).toEqual({ x: 50, y: 25, width: 400, height: 200, pixelRatio: 2 });
    expect(p.width * p.pixelRatio).toBe(800);
  });
});

describe("dragRect", () => {
  it("normalizes any drag direction", () => {
    expect(dragRect(10, 20, 4, 6)).toEqual({ x: 4, y: 6, width: 6, height: 14 });
    expect(dragRect(4, 6, 10, 20)).toEqual({ x: 4, y: 6, width: 6, height: 14 });
  });
});

describe("clampRect", () => {
  it("clips a rect that overflows the image", () => {
    const image = { width: 100, height: 100 };
    expect(clampRect({ x: -10, y: 90, width: 50, height: 50 }, image)).toEqual({
      x: 0,
      y: 90,
      width: 50,
      height: 10,
    });
  });
});
