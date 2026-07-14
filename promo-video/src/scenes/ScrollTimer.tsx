import React from "react";
import {
  AbsoluteFill,
  Sequence,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, FONT, GLASS, HEADLINE, SPECTRUM } from "../brand";
import { MockContent } from "../mock";
import { SpectrumPath, roundedRectPath } from "../SpectrumLine";
import { useCopy } from "../copy";

const VIEW = { w: 620, h: 520 };
const TALL_H = 1400;

const ScrollBeat: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();
  const pillW = copy.capturingPillWidth;

  const inSpring = spring({ frame: frame - 3, fps, config: { damping: 200 } });
  const scroll = interpolate(frame, [12, 75], [0, TALL_H - VIEW.h], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  // the stitched strip zooms out to fit
  const stitch = spring({ frame: frame - 82, fps, config: { damping: 200 } });
  const pillPulse = 0.5 + 0.5 * Math.sin((frame / fps) * Math.PI * 2.4);

  const stitchScale = 1 - stitch * (1 - VIEW.h / TALL_H) * 0.92;

  return (
    <AbsoluteFill style={{ alignItems: "center" }}>
      <div
        style={{
          ...HEADLINE,
          fontSize: 72,
          marginTop: 90,
          opacity: inSpring,
          translate: `0 ${(1 - inSpring) * 30}px`,
        }}
      >
        {copy.scrollHeadline}
      </div>

      <div
        style={{
          position: "relative",
          marginTop: 48,
          width: VIEW.w,
          height: VIEW.h + 120,
          display: "flex",
          justifyContent: "center",
        }}
      >
        {stitch < 0.02 ? (
          // scrolling viewport
          <div
            style={{
              width: VIEW.w,
              height: VIEW.h,
              borderRadius: 12,
              overflow: "hidden",
              position: "relative",
              opacity: inSpring,
            }}
          >
            <div style={{ translate: `0 ${-scroll}px` }}>
              <MockContent width={VIEW.w} height={TALL_H} seed={7} />
            </div>
            {/* spectrum-bordered recording pill */}
            <div
              style={{
                position: "absolute",
                bottom: 18,
                left: "50%",
                translate: "-50% 0",
              }}
            >
              <div
                style={{
                  ...GLASS,
                  borderRadius: 999,
                  width: pillW,
                  height: 47,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  gap: 12,
                  fontFamily: FONT,
                  fontSize: 22,
                  fontWeight: 600,
                  color: "#ffffff",
                  whiteSpace: "nowrap",
                  position: "relative",
                }}
              >
                <div
                  style={{
                    width: 12,
                    height: 12,
                    borderRadius: "50%",
                    backgroundColor: SPECTRUM[3],
                    opacity: 0.5 + pillPulse * 0.5,
                    boxShadow: `0 0 ${6 + pillPulse * 10}px ${SPECTRUM[3]}`,
                  }}
                />
                {copy.capturingPill}
                <SpectrumPath
                  path={roundedRectPath(pillW, 47, 23, 1)}
                  width={pillW}
                  height={47}
                  progress={1}
                  strokeWidth={2}
                  glowWidth={7}
                  glowOpacity={0.4 + pillPulse * 0.5}
                  style={{ position: "absolute", inset: 0 }}
                />
              </div>
            </div>
          </div>
        ) : (
          // the stitched long image snapping to fit
          <div
            style={{
              scale: String(stitchScale),
              transformOrigin: "top center",
            }}
          >
            <MockContent
              width={VIEW.w}
              height={TALL_H}
              seed={7}
              style={{ boxShadow: "0 24px 64px rgba(0,0,0,0.7)" }}
            />
          </div>
        )}
      </div>

      <div
        style={{
          ...CAPTION,
          position: "absolute",
          bottom: 100,
          left: 0,
          right: 0,
          opacity: interpolate(frame, [20, 45], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          }),
        }}
      >
        {copy.scrollCaption}
      </div>
    </AbsoluteFill>
  );
};

const TimerBeat: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const inSpring = spring({ frame: frame - 2, fps, config: { damping: 200 } });
  // 3 → 2 → 1, 22 frames each, then shutter
  const digit = frame < 22 ? 3 : frame < 44 ? 2 : 1;
  const digitFrame = frame % 22;
  const digitPop = spring({
    frame: digitFrame,
    fps,
    config: { damping: 20, stiffness: 200 },
  });
  const ringLeft = interpolate(frame, [0, 66], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const flash = frame >= 68 && frame < 72 ? 1 : 0;

  const R = 118;
  const SIZE = 300;

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 60,
          opacity: inSpring,
          scale: String(0.9 + inSpring * 0.1),
        }}
      >
        <div
          style={{
            ...GLASS,
            width: SIZE,
            height: SIZE,
            borderRadius: "50%",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            position: "relative",
          }}
        >
          <div
            style={{
              fontFamily: FONT,
              fontSize: 120,
              fontWeight: 700,
              color: "#ffffff",
              fontVariantNumeric: "tabular-nums",
              scale: String(0.7 + digitPop * 0.3),
              opacity: digitPop,
            }}
          >
            {digit}
          </div>
          {/* depleting spectrum ring */}
          <SpectrumPath
            path={`M ${SIZE / 2} ${SIZE / 2 - R} A ${R} ${R} 0 1 1 ${SIZE / 2 - 0.01} ${SIZE / 2 - R}`}
            width={SIZE}
            height={SIZE}
            progress={ringLeft}
            strokeWidth={4}
            glowWidth={12}
            style={{ position: "absolute", inset: 0 }}
          />
        </div>
        <div style={{ ...CAPTION }}>{copy.timerCaption}</div>
      </div>
      <AbsoluteFill style={{ backgroundColor: "#ffffff", opacity: flash }} />
    </AbsoluteFill>
  );
};

// Scene 6 (210 frames): beat 1 — scrolling capture (0–120);
// beat 2 — timed capture countdown (120–210).
export const ScrollTimer: React.FC = () => {
  return (
    <AbsoluteFill>
      <Sequence name="Scrolling capture" durationInFrames={120}>
        <ScrollBeat />
      </Sequence>
      <Sequence name="Timed capture" from={120} durationInFrames={90}>
        <TimerBeat />
      </Sequence>
    </AbsoluteFill>
  );
};
