import React from "react";
import { SPECTRUM } from "./brand";

let uid = 0;

/**
 * The brand motif: a thin neon spectrum stroke that draws itself along an
 * arbitrary SVG path. A blurred duplicate underneath provides the bloom.
 * `progress` 0..1 controls how much of the path is drawn (uses pathLength=1
 * so no path measuring is needed). Never used as a fill — strokes only.
 */
export const SpectrumPath: React.FC<{
  path: string;
  width: number;
  height: number;
  progress: number;
  strokeWidth?: number;
  glowWidth?: number;
  glowOpacity?: number;
  style?: React.CSSProperties;
}> = ({
  path,
  width,
  height,
  progress,
  strokeWidth = 3,
  glowWidth = 10,
  glowOpacity = 0.8,
  style,
}) => {
  const [id] = React.useState(() => `spectrum-${uid++}`);
  const p = Math.max(0, Math.min(1, progress));

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      style={{ display: "block", overflow: "visible", ...style }}
    >
      <defs>
        {/* userSpaceOnUse: an objectBoundingBox gradient degenerates on
            zero-height (horizontal-line) paths and renders nothing */}
        <linearGradient
          id={id}
          gradientUnits="userSpaceOnUse"
          x1="0"
          y1="0"
          x2={width}
          y2="0"
        >
          {SPECTRUM.map((c, i) => (
            <stop key={c} offset={i / (SPECTRUM.length - 1)} stopColor={c} />
          ))}
        </linearGradient>
        <filter id={`${id}-blur`} x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation={glowWidth / 2} />
        </filter>
      </defs>
      {/* bloom layer */}
      <path
        d={path}
        fill="none"
        stroke={`url(#${id})`}
        strokeWidth={glowWidth}
        strokeLinecap="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1 - p}
        filter={`url(#${id}-blur)`}
        opacity={p <= 0 ? 0 : glowOpacity}
      />
      {/* crisp line */}
      <path
        d={path}
        fill="none"
        stroke={`url(#${id})`}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1 - p}
        opacity={p <= 0 ? 0 : 1}
      />
    </svg>
  );
};

/** Horizontal spectrum underline that draws left → right. */
export const SpectrumLine: React.FC<{
  width: number;
  progress: number;
  strokeWidth?: number;
  glowWidth?: number;
  glowOpacity?: number;
  style?: React.CSSProperties;
}> = ({ width, progress, strokeWidth = 3, glowWidth, glowOpacity, style }) => (
  <SpectrumPath
    path={`M 0 10 L ${width} 10`}
    width={width}
    height={20}
    progress={progress}
    strokeWidth={strokeWidth}
    glowWidth={glowWidth}
    glowOpacity={glowOpacity}
    style={style}
  />
);

/** A rounded-rect outline path (for tracing card borders and the app icon). */
export const roundedRectPath = (
  w: number,
  h: number,
  r: number,
  inset = 0,
): string => {
  const x0 = inset;
  const y0 = inset;
  const x1 = w - inset;
  const y1 = h - inset;
  return [
    `M ${x0 + r} ${y0}`,
    `L ${x1 - r} ${y0}`,
    `A ${r} ${r} 0 0 1 ${x1} ${y0 + r}`,
    `L ${x1} ${y1 - r}`,
    `A ${r} ${r} 0 0 1 ${x1 - r} ${y1}`,
    `L ${x0 + r} ${y1}`,
    `A ${r} ${r} 0 0 1 ${x0} ${y1 - r}`,
    `L ${x0} ${y0 + r}`,
    `A ${r} ${r} 0 0 1 ${x0 + r} ${y0}`,
    "Z",
  ].join(" ");
};
