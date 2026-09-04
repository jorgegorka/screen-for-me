import Konva from "konva";

import { polygonBounds, taperedArrowPoints } from "./arrow";
import { counterTextColor } from "./counter";

export type ShapeType =
  | "arrow"
  | "rect"
  | "ellipse"
  | "line"
  | "pen"
  | "highlight"
  | "text"
  | "counter"
  | "pixelate";

export interface ShapeSpec {
  type: ShapeType;
  attrs: Record<string, unknown>;
}

const COMMON = ["x", "y", "rotation", "scaleX", "scaleY", "opacity"];

const ATTRS: Record<ShapeType, string[]> = {
  arrow: [...COMMON, "points", "fill", "strokeWidth"],
  rect: [...COMMON, "width", "height", "stroke", "strokeWidth", "cornerRadius"],
  ellipse: [...COMMON, "radiusX", "radiusY", "stroke", "strokeWidth"],
  line: [...COMMON, "points", "stroke", "strokeWidth", "lineCap"],
  pen: [...COMMON, "points", "stroke", "strokeWidth", "lineCap", "lineJoin", "tension"],
  highlight: [...COMMON, "points", "stroke", "strokeWidth", "lineCap", "lineJoin", "globalCompositeOperation"],
  text: [...COMMON, "text", "fontSize", "fill", "fontStyle", "width"],
  counter: [...COMMON, "radius", "fill", "number"],
  pixelate: [...COMMON, "width", "height", "src"],
};

function nodeToSpec(node: Konva.Node): ShapeSpec | null {
  const type = node.name() as ShapeType;
  if (!(type in ATTRS)) return null;
  const attrs: Record<string, unknown> = {};
  for (const key of ATTRS[type]) {
    const value = node.getAttr(key);
    if (value !== undefined) attrs[key] = value;
  }
  return { type, attrs };
}

export interface CounterConfig {
  x: number;
  y: number;
  radius: number;
  fill: string;
  number: number;
  rotation?: number;
  scaleX?: number;
  scaleY?: number;
  opacity?: number;
}

export function buildCounter(config: CounterConfig): Konva.Group {
  const { radius, fill, number, ...rest } = config;
  const group = new Konva.Group({ ...rest, name: "counter" });
  group.setAttr("radius", radius);
  group.setAttr("fill", fill);
  group.setAttr("number", number);
  group.add(new Konva.Circle({ radius, fill }));
  group.add(
    new Konva.Text({
      text: String(number),
      fill: counterTextColor(fill),
      fontStyle: "bold",
      fontSize: radius * 1.2,
      fontFamily: "-apple-system, system-ui, sans-serif",
      width: radius * 2,
      height: radius * 2,
      x: -radius,
      y: -radius,
      align: "center",
      verticalAlign: "middle",
      listening: false,
    }),
  );
  return group;
}

export function buildArrow(attrs: Konva.ShapeConfig): Konva.Shape {
  const shape = new Konva.Shape({
    ...attrs,
    name: "arrow",
    sceneFunc(ctx, node) {
      const pts = node.getAttr("points") as number[] | undefined;
      if (!pts || pts.length < 4) return;
      const poly = taperedArrowPoints(pts[0], pts[1], pts[2], pts[3], node.strokeWidth());
      if (poly.length === 0) return;
      ctx.beginPath();
      ctx.moveTo(poly[0], poly[1]);
      for (let i = 2; i < poly.length; i += 2) ctx.lineTo(poly[i], poly[i + 1]);
      ctx.closePath();
      ctx.fillStrokeShape(node);
    },
  });
  shape.getSelfRect = () => {
    const pts = shape.getAttr("points") as number[] | undefined;
    if (!pts || pts.length < 4) return { x: 0, y: 0, width: 0, height: 0 };
    return polygonBounds(
      taperedArrowPoints(pts[0], pts[1], pts[2], pts[3], shape.strokeWidth()),
    );
  };
  return shape;
}

function specToNode(spec: ShapeSpec): Konva.Shape | Konva.Group {
  const attrs = { ...spec.attrs, name: spec.type };
  switch (spec.type) {
    case "arrow":
      return buildArrow(attrs as Konva.ShapeConfig);
    case "rect":
      return new Konva.Rect(attrs);
    case "ellipse":
      return new Konva.Ellipse(attrs as Konva.EllipseConfig);
    case "line":
    case "pen":
    case "highlight":
      return new Konva.Line(attrs as Konva.LineConfig);
    case "text":
      return new Konva.Text(attrs as Konva.TextConfig);
    case "counter":
      return buildCounter(spec.attrs as unknown as CounterConfig);
    case "pixelate":
      return new Konva.Image({
        ...(attrs as Konva.ImageConfig),
        image: pixelateImages.get(spec.attrs.src as string),
        listening: false,
      });
  }
}

const pixelateImages = new Map<string, HTMLCanvasElement>();
let nextPixelateId = 0;

export function registerPixelateImage(canvas: HTMLCanvasElement): string {
  const key = `pixelate-${nextPixelateId++}`;
  pixelateImages.set(key, canvas);
  return key;
}

export function clearPixelateImages(): void {
  pixelateImages.clear();
}

export function serializeLayer(layer: Konva.Layer): string {
  const specs = layer
    .getChildren()
    .map(nodeToSpec)
    .filter((s): s is ShapeSpec => s !== null);
  return JSON.stringify(specs);
}

export function rebuildLayer(layer: Konva.Layer, snapshot: string): void {
  layer.destroyChildren();
  const specs = JSON.parse(snapshot) as ShapeSpec[];
  for (const spec of specs) {
    layer.add(specToNode(spec));
  }
}
