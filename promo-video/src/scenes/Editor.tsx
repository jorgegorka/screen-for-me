import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, FONT, GLASS, HEADLINE, VIOLET } from "../brand";
import { MockContent } from "../mock";
import { useCopy } from "../copy";

const CARD = { w: 980, h: 560 };
const CROP = { x: 150, y: 90, w: 680, h: 400 };

// Tool step timeline (frames within the scene)
const STEPS = {
  arrow: 40,
  rect: 78,
  highlight: 104,
  badges: 132,
  pixelate: 162,
  crop: 212,
};

const TOOLS = ["arrow", "rect", "highlight", "badges", "pixelate", "crop"] as const;

const ToolGlyph: React.FC<{ kind: string; active: boolean }> = ({
  kind,
  active,
}) => {
  const stroke = active ? VIOLET : "rgba(255,255,255,0.6)";
  return (
    <svg width="26" height="26" viewBox="0 0 26 26" fill="none" stroke={stroke} strokeWidth="2.2">
      {kind === "arrow" ? (
        <path d="M4 22 L20 6 M20 6 L12 7 M20 6 L19 14" />
      ) : kind === "rect" ? (
        <rect x="4" y="6" width="18" height="14" rx="2" />
      ) : kind === "highlight" ? (
        <path d="M4 18 L18 4 L22 8 L8 22 L3 23 Z" />
      ) : kind === "badges" ? (
        <>
          <circle cx="13" cy="13" r="9" />
          <text x="13" y="17.5" textAnchor="middle" fontSize="12" fill={stroke} stroke="none" fontFamily="system-ui">1</text>
        </>
      ) : kind === "pixelate" ? (
        <>
          <rect x="5" y="5" width="7" height="7" />
          <rect x="14" y="14" width="7" height="7" />
          <rect x="14" y="5" width="7" height="7" opacity="0.45" />
          <rect x="5" y="14" width="7" height="7" opacity="0.45" />
        </>
      ) : (
        <path d="M8 3 L8 18 L23 18 M3 8 L18 8 L18 23" />
      )}
    </svg>
  );
};

// deterministic pseudo-random gray for the pixelate mosaic
const cellGray = (i: number) => 30 + ((i * 37) % 11) * 9;

// Scene 5 (300 frames): tools fire in rapid succession on a screenshot card —
// arrow, rectangle, highlighter, counter badges, pixelate, crop.
export const Editor: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const headlineIn = spring({ frame: frame - 5, fps, config: { damping: 200 } });
  const cardIn = spring({ frame: frame - 15, fps, config: { damping: 200 } });

  // which tool is currently "active" in the toolbar
  const activeTool =
    frame >= STEPS.crop ? 5
    : frame >= STEPS.pixelate ? 4
    : frame >= STEPS.badges ? 3
    : frame >= STEPS.highlight ? 2
    : frame >= STEPS.rect ? 1
    : frame >= STEPS.arrow ? 0
    : -1;

  // 1 — arrow draws itself, spring overshoot on the head
  const arrowDraw = interpolate(frame, [STEPS.arrow, STEPS.arrow + 22], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const headPop = spring({
    frame: frame - (STEPS.arrow + 18),
    fps,
    config: { damping: 20, stiffness: 200 },
  });

  // 2 — rectangle + highlighter sweep
  const rectDraw = interpolate(frame, [STEPS.rect, STEPS.rect + 18], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const highlight = interpolate(
    frame,
    [STEPS.highlight, STEPS.highlight + 20],
    [0, 1],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" },
  );

  // 4 — pixelate mosaic grows over a region
  const pixel = interpolate(frame, [STEPS.pixelate, STEPS.pixelate + 30], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // 5 — crop: scrim closes in, then the canvas snaps to the cropped size
  const scrim = interpolate(frame, [STEPS.crop, STEPS.crop + 18], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const snap = spring({ frame: frame - (STEPS.crop + 30), fps, config: { damping: 200 } });
  const cropScale = 1 + snap * (CARD.h / CROP.h - 1) * 0.62;
  const cropCX = CROP.x + CROP.w / 2;
  const cropCY = CROP.y + CROP.h / 2;

  const captionIn = interpolate(frame, [60, 85], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const PIX = { x: 560, y: 300, w: 260, h: 150, cell: 26 };
  const cols = Math.ceil(PIX.w / PIX.cell);
  const rows = Math.ceil(PIX.h / PIX.cell);
  const shownCells = Math.round(cols * rows * pixel);

  return (
    <AbsoluteFill style={{ alignItems: "center" }}>
      <div
        style={{
          ...HEADLINE,
          marginTop: 90,
          opacity: headlineIn,
          translate: `0 ${(1 - headlineIn) * 30}px`,
        }}
      >
        {copy.editorHeadline}
      </div>

      {/* editor stage */}
      <div
        style={{
          position: "relative",
          width: CARD.w,
          height: CARD.h,
          marginTop: 56,
          opacity: cardIn,
          scale: String(0.96 + cardIn * 0.04),
        }}
      >
        {/* the canvas: everything inside scales when the crop snaps */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            scale: String(cropScale),
            transformOrigin: `${cropCX}px ${cropCY}px`,
          }}
        >
          <MockContent
            width={CARD.w}
            height={CARD.h}
            seed={6}
            style={{
              boxShadow: "0 24px 64px rgba(0,0,0,0.7)",
              clipPath:
                snap > 0.01
                  ? `inset(${CROP.y * snap}px ${(CARD.w - CROP.x - CROP.w) * snap}px ${(CARD.h - CROP.y - CROP.h) * snap}px ${CROP.x * snap}px round 12px)`
                  : undefined,
            }}
          />

          {/* 1 — violet arrow */}
          <svg
            width={CARD.w}
            height={CARD.h}
            viewBox={`0 0 ${CARD.w} ${CARD.h}`}
            style={{ position: "absolute", left: 0, top: 0, overflow: "visible" }}
          >
            <path
              d="M 180 430 C 280 380, 330 300, 360 210"
              fill="none"
              stroke={VIOLET}
              strokeWidth="7"
              strokeLinecap="round"
              pathLength={1}
              strokeDasharray={1}
              strokeDashoffset={1 - arrowDraw}
              opacity={arrowDraw > 0 ? 1 : 0}
            />
            <g
              opacity={headPop}
              style={{
                transformOrigin: "360px 210px",
                scale: String(0.4 + headPop * 0.6),
              }}
            >
              <path
                d="M 360 210 L 344 244 M 360 210 L 384 236"
                stroke={VIOLET}
                strokeWidth="7"
                strokeLinecap="round"
                fill="none"
              />
            </g>
            {/* 2 — rectangle outline */}
            <rect
              x="480"
              y="120"
              width="330"
              height="130"
              rx="6"
              fill="none"
              stroke={VIOLET}
              strokeWidth="5"
              pathLength={1}
              strokeDasharray={1}
              strokeDashoffset={1 - rectDraw}
              opacity={rectDraw > 0 ? 1 : 0}
            />
            {/* highlighter sweep across a "text line" */}
            <rect
              x="180"
              y="490"
              width={520 * highlight}
              height="34"
              rx="4"
              fill="rgba(124, 92, 230, 0.35)"
            />
          </svg>

          {/* 3 — numbered counter badges */}
          {[
            { n: "1", x: 200, y: 150 },
            { n: "2", x: 470, y: 300 },
            { n: "3", x: 840, y: 180 },
          ].map((b, i) => {
            const pop = spring({
              frame: frame - (STEPS.badges + i * 4),
              fps,
              config: { damping: 20, stiffness: 200 },
            });
            return (
              <div
                key={b.n}
                style={{
                  position: "absolute",
                  left: b.x,
                  top: b.y,
                  width: 54,
                  height: 54,
                  borderRadius: "50%",
                  backgroundColor: VIOLET,
                  color: "#ffffff",
                  fontFamily: FONT,
                  fontSize: 30,
                  fontWeight: 700,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  scale: String(pop),
                  opacity: pop,
                  boxShadow: "0 0 10px rgba(145,114,231,0.5)",
                }}
              >
                {b.n}
              </div>
            );
          })}

          {/* 4 — pixelate mosaic */}
          <div
            style={{
              position: "absolute",
              left: PIX.x,
              top: PIX.y,
              width: PIX.w,
              height: PIX.h,
              display: "flex",
              flexWrap: "wrap",
              overflow: "hidden",
              borderRadius: 4,
            }}
          >
            {Array.from({ length: cols * rows }).map((_, i) => (
              <div
                key={i}
                style={{
                  width: PIX.cell,
                  height: PIX.cell,
                  backgroundColor: `rgb(${cellGray(i)}, ${cellGray(i)}, ${cellGray(i) + 4})`,
                  opacity: i < shownCells ? 1 : 0,
                }}
              />
            ))}
          </div>

          {/* 5 — crop scrim + marquee */}
          {scrim > 0 && snap < 0.5 ? (
            <>
              <div
                style={{
                  position: "absolute",
                  inset: 0,
                  borderRadius: 12,
                  backgroundColor: `rgba(0,0,0,${scrim * 0.62})`,
                  clipPath: `polygon(0 0, 100% 0, 100% 100%, 0 100%, 0 ${CROP.y}px, ${CROP.x}px ${CROP.y}px, ${CROP.x}px ${CROP.y + CROP.h}px, ${CROP.x + CROP.w}px ${CROP.y + CROP.h}px, ${CROP.x + CROP.w}px ${CROP.y}px, 0 ${CROP.y}px)`,
                }}
              />
              <div
                style={{
                  position: "absolute",
                  left: CROP.x,
                  top: CROP.y,
                  width: CROP.w,
                  height: CROP.h,
                  border: "2px solid rgba(255,255,255,0.9)",
                  opacity: scrim,
                }}
              />
            </>
          ) : null}
        </div>
      </div>

      {/* compact glass toolbar */}
      <div
        style={{
          ...GLASS,
          marginTop: 44,
          display: "flex",
          gap: 10,
          padding: "12px 18px",
          borderRadius: 14,
          opacity: cardIn,
        }}
      >
        {TOOLS.map((t, i) => (
          <div
            key={t}
            style={{
              width: 48,
              height: 48,
              borderRadius: 9,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor:
                i === activeTool ? "rgba(124,92,230,0.22)" : "transparent",
              boxShadow:
                i === activeTool ? "0 0 10px rgba(145,114,231,0.5)" : "none",
            }}
          >
            <ToolGlyph kind={t} active={i === activeTool} />
          </div>
        ))}
      </div>

      <div style={{ ...CAPTION, marginTop: 30, opacity: captionIn }}>
        {copy.editorCaption}
      </div>
    </AbsoluteFill>
  );
};
