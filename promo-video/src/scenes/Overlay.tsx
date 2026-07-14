import React from "react";
import {
  AbsoluteFill,
  interpolate,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";
import { CAPTION, FONT, GLASS, HEADLINE } from "../brand";
import { Cursor, MockContent } from "../mock";
import { useCopy } from "../copy";

const PANEL = { x: 90, y: 700, w: 460, h: 290 };
const THUMB = { w: 380, h: 150 };
const CHAT = { x: 1120, y: 560, w: 660 };

// Scene 4 (240 frames): the quick-access overlay appears bottom-left, then
// the thumbnail is dragged out and flies into a generic chat input.
export const Overlay: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const copy = useCopy();

  const headlineIn = spring({ frame: frame - 5, fps, config: { damping: 200 } });
  const panelIn = spring({ frame: frame - 25, fps, config: { damping: 200 } });

  // drag: cursor grabs the thumb at ~70, flight 80–140
  const flight = spring({ frame: frame - 80, fps, config: { damping: 200 } });
  const dragging = frame >= 80;

  // spring arc: horizontal follows the spring, vertical adds a lob
  // matches the thumbnail slot: panel origin + 24px padding
  const startX = PANEL.x + 24;
  const startY = PANEL.y + 24;
  const endX = CHAT.x + 40;
  const endY = CHAT.y - 40;
  const tx = startX + (endX - startX) * flight;
  const ty = startY + (endY - startY) * flight - Math.sin(Math.PI * flight) * 180;
  const thumbScale = 1 - flight * 0.25;

  const landed = flight > 0.985;
  // once landed, ease the thumbnail down into the input and shrink it
  const settle = interpolate(frame, [136, 150], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const glowRing = interpolate(frame, [138, 150, 185], [0, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const captionIn = interpolate(frame, [150, 175], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // cursor path: moves onto the thumbnail (50–70), then rides the drag
  const approach = interpolate(frame, [50, 70], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const cursorX = dragging
    ? tx + (THUMB.w * thumbScale) / 2
    : interpolate(approach, [0, 1], [900, startX + THUMB.w / 2]);
  const cursorY = dragging
    ? ty + (THUMB.h * thumbScale) / 2
    : interpolate(approach, [0, 1], [420, startY + THUMB.h / 2]);

  const buttons = copy.overlayButtons;

  return (
    <AbsoluteFill>
      <div
        style={{
          ...HEADLINE,
          fontSize: 72,
          marginTop: 100,
          opacity: headlineIn,
          translate: `0 ${(1 - headlineIn) * 30}px`,
        }}
      >
        {copy.overlayHeadline}
      </div>

      {/* quick-access overlay panel, bottom-left */}
      <div
        style={{
          ...GLASS,
          position: "absolute",
          left: PANEL.x,
          top: PANEL.y,
          width: PANEL.w,
          height: PANEL.h,
          borderRadius: 16,
          padding: 24,
          display: "flex",
          flexDirection: "column",
          gap: 20,
          opacity: panelIn,
          translate: `0 ${(1 - panelIn) * 60}px`,
        }}
      >
        {/* thumbnail slot (ghost stays while the copy flies out) */}
        <div
          style={{
            width: THUMB.w,
            height: THUMB.h,
            borderRadius: 10,
            border: "1px dashed rgba(255,255,255,0.18)",
            overflow: "hidden",
          }}
        >
          {dragging ? (
            <MockContent
              width={THUMB.w}
              height={THUMB.h}
              seed={4}
              style={{ opacity: 0.25, boxShadow: "none", border: "none" }}
            />
          ) : null}
        </div>
        <div style={{ display: "flex", gap: 12 }}>
          {buttons.map((b) => (
            <div
              key={b}
              style={{
                fontFamily: FONT,
                fontSize: copy.overlayButtonFontSize,
                fontWeight: 500,
                color: "rgba(255,255,255,0.85)",
                padding: "10px 0",
                flex: 1,
                textAlign: "center",
                borderRadius: 8,
                backgroundColor: "rgba(255,255,255,0.07)",
                border: "1px solid rgba(255,255,255,0.1)",
              }}
            >
              {b}
            </div>
          ))}
        </div>
      </div>

      {/* generic chat mock, right side */}
      <div
        style={{
          position: "absolute",
          left: CHAT.x - 60,
          top: CHAT.y - 240,
          width: CHAT.w + 120,
          display: "flex",
          flexDirection: "column",
          gap: 18,
          opacity: panelIn,
        }}
      >
        {[380, 300].map((w, i) => (
          <div
            key={i}
            style={{
              ...GLASS,
              width: w,
              height: 64,
              borderRadius: 18,
              alignSelf: i === 0 ? "flex-start" : "flex-end",
            }}
          />
        ))}
        {/* input box — the drop target */}
        <div
          style={{
            ...GLASS,
            marginTop: 40,
            width: CHAT.w,
            height: 190,
            borderRadius: 16,
            padding: 20,
            border: `1px solid rgba(145,114,231,${0.14 + glowRing * 0.8})`,
            boxShadow: `0 8px 32px rgba(0,0,0,0.45), 0 0 ${glowRing * 26}px rgba(145,114,231,${glowRing * 0.6})`,
          }}
        >
          <div
            style={{
              fontFamily: FONT,
              fontSize: 26,
              color: "rgba(255,255,255,0.35)",
            }}
          >
            {landed ? "" : copy.chatPlaceholder}
          </div>
        </div>
      </div>

      {/* the flying thumbnail */}
      {dragging ? (
        <div
          style={{
            position: "absolute",
            left: 0,
            top: 0,
            translate: `${tx}px ${ty + settle * 40}px`,
            scale: String(thumbScale - settle * 0.2),
          }}
        >
          <MockContent width={THUMB.w} height={THUMB.h} seed={4} />
        </div>
      ) : (
        <div
          style={{
            position: "absolute",
            left: startX,
            top: startY,
            opacity: panelIn,
            translate: `0 ${(1 - panelIn) * 60}px`,
          }}
        >
          <MockContent width={THUMB.w} height={THUMB.h} seed={4} />
        </div>
      )}

      <Cursor x={cursorX} y={cursorY} opacity={frame >= 50 && frame < 150 ? 1 : 0} />

      <div
        style={{
          ...CAPTION,
          position: "absolute",
          bottom: 100,
          left: 0,
          right: 0,
          opacity: captionIn,
        }}
      >
        {copy.overlayCaption}
      </div>
    </AbsoluteFill>
  );
};
