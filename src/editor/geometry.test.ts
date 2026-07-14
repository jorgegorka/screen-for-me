import { describe, expect, it } from "vitest";
import {
  MAX_SCALE,
  MIN_SCALE,
  clampRect,
  clampScale,
  fillScale,
  fitScale,
  imageToScreen,
  initialScale,
  nextFitScale,
} from "./geometry";

describe("fitScale", () => {
  it("never upscales past 100%", () => {
    expect(fitScale({ width: 400, height: 300 }, { width: 2000, height: 2000 })).toBe(1);
  });

  it("fits the constraining dimension", () => {
    expect(fitScale({ width: 2000, height: 1000 }, { width: 1000, height: 1000 })).toBe(0.5);
    expect(fitScale({ width: 1000, height: 2000 }, { width: 1000, height: 500 })).toBe(0.25);
  });
});

describe("fillScale", () => {
  it("never upscales past 100%", () => {
    expect(fillScale({ width: 400, height: 300 }, { width: 2000, height: 2000 })).toBe(1);
  });

  it("fits the less-constraining dimension so one axis fills the viewport", () => {
    // Tall scrolling capture: fill the width, overflow (scroll) vertically.
    expect(fillScale({ width: 1600, height: 20000 }, { width: 800, height: 700 })).toBe(0.5);
    // Wide panorama: fill the height, overflow horizontally.
    expect(fillScale({ width: 20000, height: 1000 }, { width: 1000, height: 500 })).toBe(0.5);
  });
});

describe("initialScale", () => {
  const viewport = { width: 1167, height: 721 };

  it("fits ordinary screenshots entirely in the viewport", () => {
    // @2x fullscreen capture: contain-fit, unchanged from fitScale.
    expect(initialScale({ width: 3456, height: 2234 }, viewport)).toBe(
      fitScale({ width: 3456, height: 2234 }, viewport),
    );
  });

  it("width-fits extremely tall images instead of shrinking them to a sliver", () => {
    const image = { width: 1600, height: 20000 };
    expect(initialScale(image, viewport)).toBe(fillScale(image, viewport));
  });

  it("height-fits extremely wide images", () => {
    const image = { width: 20000, height: 1000 };
    expect(initialScale(image, viewport)).toBe(fillScale(image, viewport));
  });

  it("never upscales past 100%", () => {
    expect(initialScale({ width: 300, height: 4000 }, { width: 500, height: 500 })).toBeLessThanOrEqual(1);
  });
});

describe("nextFitScale", () => {
  const viewport = { width: 1167, height: 721 };
  const tall = { width: 1600, height: 20000 };
  const normal = { width: 3456, height: 2234 };

  it("goes to the readable smart fit from an arbitrary zoom", () => {
    expect(nextFitScale(1.14, tall, viewport)).toBe(initialScale(tall, viewport));
  });

  it("toggles from smart fit to whole-image fit and back on a tall image", () => {
    const smart = initialScale(tall, viewport);
    const whole = fitScale(tall, viewport);
    expect(nextFitScale(smart, tall, viewport)).toBe(whole);
    expect(nextFitScale(whole, tall, viewport)).toBe(smart);
  });

  it("is a plain contain fit for ordinary screenshots", () => {
    const fit = fitScale(normal, viewport);
    expect(nextFitScale(0.9, normal, viewport)).toBe(fit);
    expect(nextFitScale(fit, normal, viewport)).toBe(fit);
  });
});

describe("clampScale", () => {
  it("clamps to the zoom bounds", () => {
    expect(clampScale(0)).toBe(MIN_SCALE);
    expect(clampScale(100)).toBe(MAX_SCALE);
    expect(clampScale(1.5)).toBe(1.5);
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
