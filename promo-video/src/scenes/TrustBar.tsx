import React from "react";
import {
  AbsoluteFill,
  Sequence,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { HEADLINE } from "../brand";
import { SpectrumLine } from "../SpectrumLine";
import { useCopy } from "../copy";

const Punch: React.FC<{ text: string }> = ({ text }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const pop = spring({ frame, fps, config: { damping: 20, stiffness: 200 } });
  const out = interpolate(frame, [70, 77], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const underline = interpolate(frame, [3, 20], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 26,
          opacity: pop * out,
          scale: String(0.8 + pop * 0.2),
        }}
      >
        <div style={{ ...HEADLINE, fontSize: 76, maxWidth: 1600 }}>{text}</div>
        <SpectrumLine width={560} progress={underline} strokeWidth={2.5} glowWidth={8} />
      </div>
    </AbsoluteFill>
  );
};

// Scene 7 (240 frames): three trust lines punch in and out, 78 frames each —
// long enough to actually read.
export const TrustBar: React.FC = () => {
  const { trustLines } = useCopy();
  return (
    <AbsoluteFill>
      {trustLines.map((line, i) => (
        <Sequence key={i} name={`Trust ${i + 1}`} from={5 + i * 78} durationInFrames={78}>
          <Punch text={line} />
        </Sequence>
      ))}
    </AbsoluteFill>
  );
};
