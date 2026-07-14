import type React from "react";

export const SPECTRUM = ["#8794ff", "#9172e7", "#d05aa6", "#ee3138"] as const;
export const VIOLET = "#7c5ce6";
export const VIOLET_LIFT = "#9172e7";
export const FONT = "-apple-system, system-ui, sans-serif";

export const GLASS: React.CSSProperties = {
  backgroundColor: "rgba(24, 24, 26, 0.88)",
  borderRadius: 12,
  boxShadow: "0 8px 32px rgba(0, 0, 0, 0.45)",
  border: "1px solid rgba(255, 255, 255, 0.08)",
};

export const ACTIVE_GLOW = "0 0 10px rgba(145, 114, 231, 0.5)";

export const HEADLINE: React.CSSProperties = {
  fontFamily: FONT,
  fontWeight: 700,
  color: "#ffffff",
  letterSpacing: "-0.02em",
  fontSize: 84,
  textAlign: "center",
};

export const CAPTION: React.CSSProperties = {
  fontFamily: FONT,
  fontWeight: 500,
  color: "rgba(255, 255, 255, 0.6)",
  fontSize: 36,
  textAlign: "center",
};
