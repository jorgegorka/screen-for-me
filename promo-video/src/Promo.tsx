import React from "react";
import { AbsoluteFill, Sequence, interpolate, staticFile } from "remotion";
import { Audio } from "@remotion/media";
import { Hook } from "./scenes/Hook";
import { CaptureLoop } from "./scenes/CaptureLoop";
import { Modes } from "./scenes/Modes";
import { Overlay } from "./scenes/Overlay";
import { Editor } from "./scenes/Editor";
import { ScrollTimer } from "./scenes/ScrollTimer";
import { TrustBar } from "./scenes/TrustBar";
import { Finale } from "./scenes/Finale";
import { COPY, CopyProvider, type Locale } from "./copy";

// 60 s @ 30 fps = 1800 frames. Hard cuts on a 120 BPM grid (15-frame beats).
const SCENES: { name: string; from: number; duration: number; el: React.FC }[] = [
  { name: "1 Hook", from: 0, duration: 180, el: Hook },
  { name: "2 Capture loop", from: 180, duration: 210, el: CaptureLoop },
  { name: "3 Capture modes", from: 390, duration: 210, el: Modes },
  { name: "4 Overlay + drag-out", from: 600, duration: 240, el: Overlay },
  { name: "5 Annotation editor", from: 840, duration: 300, el: Editor },
  { name: "6 Scroll + timer", from: 1140, duration: 210, el: ScrollTimer },
  { name: "7 Trust bar", from: 1350, duration: 240, el: TrustBar },
  { name: "8 Finale", from: 1590, duration: 210, el: Finale },
];

export const Promo: React.FC<{ locale: Locale }> = ({ locale }) => {
  return (
    <CopyProvider value={COPY[locale]}>
      <AbsoluteFill style={{ backgroundColor: "#000000" }}>
      <Audio
        src={staticFile("music.mp3")}
        volume={(f) =>
          interpolate(f, [0, 20, 1710, 1798], [0, 1, 1, 0], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          })
        }
      />
      {SCENES.map(({ name, from, duration, el: Scene }) => (
        <Sequence key={name} name={name} from={from} durationInFrames={duration}>
          <Scene />
        </Sequence>
      ))}
      {/* constant film-like vignette to keep focus center-frame */}
      <AbsoluteFill
        style={{
          pointerEvents: "none",
          background:
            "radial-gradient(ellipse at center, rgba(0,0,0,0) 55%, rgba(0,0,0,0.2) 100%)",
        }}
      />
      </AbsoluteFill>
    </CopyProvider>
  );
};
