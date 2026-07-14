import Konva from "konva";

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

/** Whitelist of attrs that survive undo snapshots, per shape type. */
const ATTRS: Record<ShapeType, string[]> = {
  arrow: [...COMMON, "points", "stroke", "fill", "strokeWidth", "pointerLength", "pointerWidth"],
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

/**
 * Numbered badge: a group of circle + white number, with its semantic state
 * (radius/fill/number) baked into attrs on the group so the flat serializer
 * can round-trip it (same trick as pixelate's `src`).
 */
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
      fill: "#ffffff",
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

function specToNode(
  spec: ShapeSpec,
  onImageReady: () => void,
): Konva.Shape | Konva.Group {
  const attrs = { ...spec.attrs, name: spec.type };
  switch (spec.type) {
    case "arrow":
      return new Konva.Arrow(attrs as Konva.ArrowConfig);
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
    case "pixelate": {
      const node = new Konva.Image({
        ...(attrs as Konva.ImageConfig),
        image: undefined,
        listening: false,
      });
      const img = new window.Image();
      img.onload = () => {
        node.image(img);
        onImageReady();
      };
      img.src = spec.attrs.src as string;
      return node;
    }
  }
}

export function serializeLayer(layer: Konva.Layer): string {
  const specs = layer
    .getChildren()
    .map(nodeToSpec)
    .filter((s): s is ShapeSpec => s !== null);
  return JSON.stringify(specs);
}

export function rebuildLayer(
  layer: Konva.Layer,
  snapshot: string,
  onImageReady: () => void,
): void {
  layer.destroyChildren();
  const specs = JSON.parse(snapshot) as ShapeSpec[];
  for (const spec of specs) {
    layer.add(specToNode(spec, onImageReady));
  }
}
