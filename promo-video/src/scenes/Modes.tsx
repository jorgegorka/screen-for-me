import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, FONT, GLASS, HEADLINE } from "../brand";
import { Keycap } from "../mock";
import { useCopy } from "../copy";

const CARDS = [
  { keys: ["⌘", "⇧", "7"], icon: "area" },
  { keys: ["⌘", "⇧", "8"], icon: "window" },
  { keys: ["⌘", "⇧", "9"], icon: "display" },
] as const;

const ModeIcon: React.FC<{ kind: string }> = ({ kind }) => {
  const stroke = "rgba(255,255,255,0.85)";
  return (
    <svg width="88" height="66" viewBox="0 0 88 66">
      {kind === "area" ? (
        <rect
          x="14"
          y="8"
          width="60"
          height="50"
          rx="6"
          fill="none"
          stroke={stroke}
          strokeWidth="3"
          strokeDasharray="9 7"
        />
      ) : kind === "window" ? (
        <g fill="none" stroke={stroke} strokeWidth="3">
          <rect x="14" y="8" width="60" height="50" rx="6" />
          <line x1="14" y1="22" x2="74" y2="22" />
          <circle cx="23" cy="15" r="2.4" fill={stroke} stroke="none" />
          <circle cx="31" cy="15" r="2.4" fill={stroke} stroke="none" />
        </g>
      ) : (
        <g fill="none" stroke={stroke} strokeWidth="3">
          <rect x="10" y="6" width="68" height="44" rx="6" />
          <line x1="34" y1="60" x2="54" y2="60" />
          <line x1="44" y1="50" x2="44" y2="60" />
        </g>
      )}
    </svg>
  );
};

// Scene 3 (210 frames): three glass HUD cards slide up staggered, keycaps
// press down as each card lands, borders glow violet briefly.
export const Modes: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const headlineIn = spring({ frame: frame - 5, fps, config: { damping: 200 } });
  const captionIn = interpolate(frame, [95, 120], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ alignItems: "center" }}>
      <div
        style={{
          ...HEADLINE,
          marginTop: 110,
          opacity: headlineIn,
          translate: `0 ${(1 - headlineIn) * 30}px`,
        }}
      >
        {copy.modesHeadline}
      </div>

      <div style={{ display: "flex", gap: 56, marginTop: 110 }}>
        {CARDS.map((card, i) => {
          const start = 30 + i * 5; // ~150 ms stagger
          const rise = spring({
            frame: frame - start,
            fps,
            config: { damping: 200 },
          });
          const landed = frame - start - 14;
          // keycaps dip for 2 frames right after landing
          const press = landed >= 0 && landed < 4 ? 1 : 0;
          // brief violet border glow as the card lands
          const glow = interpolate(frame - start, [12, 18, 40], [0, 1, 0], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          });
          return (
            <div
              key={card.icon}
              style={{
                ...GLASS,
                width: 380,
                height: 400,
                borderRadius: 20,
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                gap: 34,
                opacity: rise,
                translate: `0 ${(1 - rise) * 120}px`,
                border: `1px solid rgba(145, 114, 231, ${0.14 + glow * 0.8})`,
                boxShadow: `0 8px 32px rgba(0,0,0,0.45), 0 0 ${10 + glow * 14}px rgba(145,114,231,${glow * 0.55})`,
              }}
            >
              <ModeIcon kind={card.icon} />
              <div
                style={{
                  fontFamily: FONT,
                  fontSize: 40,
                  fontWeight: 600,
                  color: "#ffffff",
                }}
              >
                {copy.modeTitles[i]}
              </div>
              <div style={{ display: "flex", gap: 12 }}>
                {card.keys.map((k) => (
                  <Keycap key={k} label={k} press={press} />
                ))}
              </div>
            </div>
          );
        })}
      </div>

      <div style={{ ...CAPTION, marginTop: 80, opacity: captionIn, maxWidth: 1300 }}>
        {copy.modesCaption}
      </div>
    </AbsoluteFill>
  );
};
