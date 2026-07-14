import React from "react";
import { FONT, GLASS } from "./brand";

/**
 * Fake screenshot content: gray text bars + a media block on dark glass.
 * Deliberately abstract — no real UI, nothing readable.
 */
export const MockContent: React.FC<{
  width: number;
  height: number;
  seed?: number;
  style?: React.CSSProperties;
}> = ({ width, height, seed = 0, style }) => {
  const pad = Math.round(width * 0.07);
  const barH = Math.max(8, Math.round(height * 0.045));
  const rows = Math.floor((height - pad * 2) / (barH * 2));
  const widths = [0.55, 0.82, 0.7, 0.9, 0.4, 0.76, 0.62, 0.85];
  return (
    <div
      style={{
        ...GLASS,
        width,
        height,
        overflow: "hidden",
        padding: pad,
        display: "flex",
        flexDirection: "column",
        gap: barH,
        ...style,
      }}
    >
      <div
        style={{
          width: "45%",
          height: barH * 1.6,
          borderRadius: barH,
          backgroundColor: "rgba(255,255,255,0.22)",
        }}
      />
      {Array.from({ length: Math.max(0, rows - 2) }).map((_, i) => (
        <div
          key={i}
          style={{
            width: `${widths[(i + seed) % widths.length] * 100}%`,
            height: barH,
            borderRadius: barH,
            backgroundColor: "rgba(255,255,255,0.12)",
          }}
        />
      ))}
    </div>
  );
};

/** Native-looking arrow cursor (white with black outline), tip at top-left. */
export const Cursor: React.FC<{
  x: number;
  y: number;
  scale?: number;
  variant?: "arrow" | "crosshair";
  opacity?: number;
}> = ({ x, y, scale = 1, variant = "arrow", opacity = 1 }) => (
  <div
    style={{
      position: "absolute",
      left: x,
      top: y,
      scale: String(scale),
      opacity,
      pointerEvents: "none",
    }}
  >
    {variant === "arrow" ? (
      <svg width="34" height="44" viewBox="0 0 17 22">
        <path
          d="M1 1 L1 16.5 L4.8 13 L7.2 19 L10 17.8 L7.6 12 L12.5 12 Z"
          fill="#ffffff"
          stroke="#000000"
          strokeWidth="1.2"
        />
      </svg>
    ) : (
      <svg
        width="44"
        height="44"
        viewBox="0 0 22 22"
        style={{ translate: "-22px -22px" }}
      >
        <g stroke="#ffffff" strokeWidth="1.6">
          <line x1="11" y1="1" x2="11" y2="8" />
          <line x1="11" y1="14" x2="11" y2="21" />
          <line x1="1" y1="11" x2="8" y2="11" />
          <line x1="14" y1="11" x2="21" y2="11" />
        </g>
      </svg>
    )}
  </div>
);

/** Physical-looking keycap; `press` 0..1 dips it down. */
export const Keycap: React.FC<{ label: string; press?: number }> = ({
  label,
  press = 0,
}) => (
  <div
    style={{
      fontFamily: FONT,
      fontSize: 30,
      fontWeight: 600,
      color: "rgba(255,255,255,0.92)",
      width: 62,
      height: 62,
      borderRadius: 12,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      backgroundColor: "#232326",
      border: "1px solid rgba(255,255,255,0.14)",
      boxShadow: `0 ${4 - press * 3}px 0 rgba(0,0,0,0.6), inset 0 1px 0 rgba(255,255,255,0.1)`,
      translate: `0 ${press * 3}px`,
    }}
  >
    {label}
  </div>
);
