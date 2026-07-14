import React from "react";
import {
  AbsoluteFill,
  Img,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, FONT, VIOLET } from "../brand";
import { SpectrumPath, roundedRectPath } from "../SpectrumLine";
import { useCopy } from "../copy";

const ICON = 300;

// Scene 8 (210 frames): the spectrum line traces the app icon's rounded
// square, the icon blooms in, then the lockup + "100% Free" + URL + QR.
// The full lockup holds for the final 90+ frames.
export const Finale: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const trace = interpolate(frame, [5, 60], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const iconIn = interpolate(frame, [55, 85], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const titleIn = spring({ frame: frame - 72, fps, config: { damping: 200 } });
  const freeIn = spring({
    frame: frame - 84,
    fps,
    config: { damping: 20, stiffness: 200 },
  });
  const urlIn = spring({ frame: frame - 96, fps, config: { damping: 200 } });
  const qrIn = interpolate(frame, [108, 128], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // one-time glow pulse on "100% Free"
  const freePulse =
    Math.sin(
      Math.PI *
        interpolate(frame, [95, 125], [0, 1], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        }),
    ) * 1;

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 140 }}>
        {/* icon + text lockup */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 44,
          }}
        >
          <div style={{ position: "relative", width: ICON, height: ICON }}>
            <Img
              src={staticFile("icon.png")}
              style={{
                width: ICON,
                height: ICON,
                opacity: iconIn,
                filter: `drop-shadow(0 0 ${24 * iconIn}px rgba(145,114,231,0.45))`,
              }}
            />
            {/* traced neon frame, mirroring the mark */}
            <SpectrumPath
              path={roundedRectPath(ICON, ICON, 66, 4)}
              width={ICON}
              height={ICON}
              progress={trace}
              strokeWidth={3}
              glowWidth={12}
              glowOpacity={0.9 - iconIn * 0.45}
              style={{ position: "absolute", inset: 0 }}
            />
          </div>

          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: 18,
            }}
          >
            <div
              style={{
                fontFamily: FONT,
                fontSize: 64,
                fontWeight: 700,
                color: "#ffffff",
                letterSpacing: "-0.02em",
                opacity: titleIn,
                translate: `0 ${(1 - titleIn) * 26}px`,
              }}
            >
              Screen for me
            </div>
            <div
              style={{
                fontFamily: FONT,
                fontSize: 54,
                fontWeight: 700,
                color: VIOLET,
                fontVariantNumeric: "tabular-nums",
                opacity: freeIn,
                scale: String(0.7 + freeIn * 0.3),
                textShadow: `0 0 ${10 + freePulse * 26}px rgba(145,114,231,${0.5 + freePulse * 0.5})`,
              }}
            >
              {copy.finaleFree}
            </div>
            <div
              style={{
                fontFamily: FONT,
                fontSize: 24,
                fontWeight: 500,
                color: "rgba(255,255,255,0.6)",
                opacity: urlIn,
              }}
            >
              screenforme.app
            </div>
          </div>
        </div>

        {/* QR code, framed by a thin spectrum border */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 28,
            opacity: qrIn,
          }}
        >
          <div style={{ position: "relative", padding: 24 }}>
            <Img
              src={staticFile("qr.svg")}
              style={{ width: 220, height: 220, display: "block" }}
            />
            <SpectrumPath
              path={roundedRectPath(268, 268, 14, 1.5)}
              width={268}
              height={268}
              progress={qrIn}
              strokeWidth={2}
              glowWidth={8}
              style={{ position: "absolute", left: 0, top: 0 }}
            />
          </div>
          <div style={{ ...CAPTION, fontSize: 28 }}>{copy.finaleScan}</div>
        </div>
      </div>
    </AbsoluteFill>
  );
};
