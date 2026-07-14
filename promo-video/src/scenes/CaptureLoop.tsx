import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, HEADLINE, VIOLET } from "../brand";
import { Cursor, MockContent } from "../mock";
import { SpectrumPath, roundedRectPath } from "../SpectrumLine";
import { useCopy } from "../copy";

// Selection region, in stage coordinates (stage is 1400x620).
const SEL = { x: 420, y: 130, w: 560, h: 360 };

// Scene 2 (210 frames): crosshair draws a violet marquee over an abstract
// desktop, shutter flash, the region lifts off as a floating card and a
// spectrum line traces its border.
export const CaptureLoop: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const headlineIn = spring({ frame: frame - 5, fps, config: { damping: 200 } });

  // marquee drag: frames 40–85
  const drag = interpolate(frame, [40, 85], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const selW = SEL.w * drag;
  const selH = SEL.h * drag;

  // shutter: 2-frame white flash on the region
  const flash = frame >= 88 && frame < 90 ? 1 : 0;

  // lift-off after the flash
  const lift = spring({ frame: frame - 92, fps, config: { damping: 200 } });

  // spectrum border trace on the floating card
  const trace = interpolate(frame, [105, 155], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const subIn = interpolate(frame, [140, 165], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const cursorX = SEL.x + selW;
  const cursorY = SEL.y + selH;

  return (
    <AbsoluteFill style={{ alignItems: "center" }}>
      <div
        style={{
          ...HEADLINE,
          fontSize: 66,
          marginTop: 100,
          opacity: headlineIn,
          translate: `0 ${(1 - headlineIn) * 30}px`,
        }}
      >
        {copy.captureHeadline}
      </div>

      {/* abstract desktop stage */}
      <div
        style={{
          position: "relative",
          width: 1400,
          height: 620,
          marginTop: 40,
        }}
      >
        {/* blurred glass "windows" — deliberately abstract */}
        <div
          style={{
            position: "absolute",
            inset: 0,
            filter: "blur(3px)",
            opacity: 0.8,
          }}
        >
          <MockContent
            width={640}
            height={430}
            seed={2}
            style={{ position: "absolute", left: 60, top: 60 }}
          />
          <MockContent
            width={520}
            height={360}
            seed={5}
            style={{ position: "absolute", left: 780, top: 150 }}
          />
          <MockContent
            width={420}
            height={280}
            seed={1}
            style={{ position: "absolute", left: 460, top: 280 }}
          />
        </div>

        {/* selection marquee */}
        {drag > 0 && lift < 0.05 ? (
          <div
            style={{
              position: "absolute",
              left: SEL.x,
              top: SEL.y,
              width: selW,
              height: selH,
              border: `2.5px dashed ${VIOLET}`,
              boxShadow: "0 0 10px rgba(145, 114, 231, 0.5)",
              backgroundColor: "rgba(124, 92, 230, 0.07)",
            }}
          />
        ) : null}

        {/* the captured region lifting off as a card */}
        {frame >= 88 ? (
          <div
            style={{
              position: "absolute",
              left: SEL.x,
              top: SEL.y,
              width: SEL.w,
              height: SEL.h,
              scale: String(1 + lift * 0.07),
              translate: `0 ${lift * -26}px`,
            }}
          >
            <MockContent
              width={SEL.w}
              height={SEL.h}
              seed={3}
              style={{
                boxShadow: `0 ${8 + lift * 24}px ${32 + lift * 32}px rgba(0,0,0,${0.45 + lift * 0.25})`,
              }}
            />
            {/* shutter flash */}
            <div
              style={{
                position: "absolute",
                inset: 0,
                borderRadius: 12,
                backgroundColor: "#ffffff",
                opacity: flash,
              }}
            />
            {/* spectrum border trace */}
            <SpectrumPath
              path={roundedRectPath(SEL.w, SEL.h, 12, 1.5)}
              width={SEL.w}
              height={SEL.h}
              progress={trace}
              style={{ position: "absolute", left: 0, top: 0 }}
            />
          </div>
        ) : null}

        {/* crosshair cursor riding the marquee corner */}
        <Cursor
          x={cursorX}
          y={cursorY}
          variant="crosshair"
          opacity={frame >= 30 && frame < 90 ? 1 : 0}
        />
      </div>

      <div style={{ ...CAPTION, marginTop: 36, opacity: subIn }}>
        {copy.captureCaption}
      </div>
    </AbsoluteFill>
  );
};
