import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { HEADLINE, SPECTRUM } from "../brand";
import { SpectrumLine } from "../SpectrumLine";
import { useCopy } from "../copy";

const LINE_W = 1600;

// Scene 1 (180 frames): a point of light draws the spectrum line, the
// headline snaps in word by word, one pulse, then the lockup scales down 8%.
export const Hook: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { hookWords } = useCopy();

  const draw = interpolate(frame, [10, 70], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // one glow pulse after the lockup lands
  const pulse =
    Math.sin(
      Math.PI *
        interpolate(frame, [115, 140], [0, 1], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        }),
    ) * 0.9;

  const settle = interpolate(frame, [162, 178], [1, 0.92], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const headX = -LINE_W / 2 + LINE_W * draw;

  return (
    <AbsoluteFill
      style={{ alignItems: "center", justifyContent: "center" }}
    >
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 48,
          scale: String(settle),
        }}
      >
        <div style={{ ...HEADLINE, fontSize: 96, display: "flex", gap: 28 }}>
          {hookWords.map((word, i) => {
            const pop = spring({
              frame: frame - 68 - i * 3,
              fps,
              config: { damping: 20, stiffness: 200 },
            });
            return (
              <span
                key={i}
                style={{
                  display: "inline-block",
                  opacity: pop,
                  scale: String(0.6 + 0.4 * pop),
                }}
              >
                {word}
              </span>
            );
          })}
        </div>
        <div style={{ position: "relative" }}>
          <SpectrumLine
            width={LINE_W}
            progress={draw}
            glowOpacity={0.8 + pulse * 0.6}
            glowWidth={10 + pulse * 8}
          />
          {/* the travelling point of light at the head of the line */}
          <div
            style={{
              position: "absolute",
              top: 10,
              left: LINE_W / 2 + headX,
              width: 14,
              height: 14,
              borderRadius: "50%",
              translate: "-50% -50%",
              backgroundColor: "#ffffff",
              boxShadow: `0 0 24px 8px ${SPECTRUM[0]}`,
              opacity: draw > 0 && draw < 1 ? 1 : 0,
            }}
          />
        </div>
      </div>
    </AbsoluteFill>
  );
};
