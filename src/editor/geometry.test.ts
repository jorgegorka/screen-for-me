import { describe, expect, it } from "vitest";
import { clampRect, fitScale, imageToScreen } from "./geometry";

describe("fitScale", () => {
  it("never upscales past 100%", () => {
    expect(fitScale({ width: 400, height: 300 }, { width: 2000, height: 2000 })).toBe(1);
  });

  it("fits the constraining dimension", () => {
    expect(fitScale({ width: 2000, height: 1000 }, { width: 1000, height: 1000 })).toBe(0.5);
    expect(fitScale({ width: 1000, height: 2000 }, { width: 1000, height: 500 })).toBe(0.25);
  });
});

describe("imageToScreen", () => {
  it("scales image coordinates by the fit scale", () => {
    // @2x capture displayed at 0.5: image point (1650, 900) sits at (825, 450) on screen
    expect(imageToScreen({ x: 1650, y: 900 }, { x: 0, y: 0 }, 0.5)).toEqual({ x: 825, y: 450 });
  });

  it("applies the stage pan from a crop view", () => {
    // crop view starting at image x=200 → stage.position = (-200*0.5, 0)
    expect(imageToScreen({ x: 300, y: 40 }, { x: -100, y: 0 }, 0.5)).toEqual({ x: 50, y: 20 });
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
