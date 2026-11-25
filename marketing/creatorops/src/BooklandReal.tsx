import React from "react";
import { Video } from "@remotion/media";
import {
  AbsoluteFill,
  Img,
  Sequence,
  interpolate,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

export type BooklandRealProps = {
  feature: "music";
  script?: {
    hook?: string;
    body?: string;
    cta?: string;
    voiceover?: string;
  };
};

const fade = (frame: number, duration: number) =>
  interpolate(frame, [0, 8, duration - 9, duration - 1], [0, 1, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

const Header: React.FC<{ title: string; subtitle: string; duration: number }> =
  ({ title, subtitle, duration }) => {
    const frame = useCurrentFrame();
    return (
      <>
        <div style={{
          position: "absolute", top: 110, left: 85, right: 85,
          textAlign: "center", opacity: fade(frame, duration),
          translate: `0 ${interpolate(frame, [0, 13], [22, 0], {
            extrapolateRight: "clamp",
          })}px`,
        }}>
          <div style={{ fontSize: 24, color: "#89B8E8", letterSpacing: 5,
            fontWeight: 700, marginBottom: 28 }}>BOOKLAND / MACOS</div>
          <div style={{ fontSize: 72, lineHeight: 1.06, letterSpacing: -3,
            fontWeight: 760, whiteSpace: "pre-line" }}>{title}</div>
        </div>
        <div style={{ position: "absolute", left: 110, right: 110, top: 1190,
          textAlign: "center", fontSize: 39, lineHeight: 1.28,
          color: "#C6D7EC", opacity: fade(frame, duration) }}>{subtitle}</div>
      </>
    );
  };

const HoverCapture: React.FC<{ closing?: boolean }> = ({ closing = false }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const side = closing
    ? interpolate(frame, [0, 0.75 * fps, 0.95 * fps, 1.15 * fps],
        [0, 0, 200, 330], { extrapolateRight: "clamp" })
    : interpolate(frame, [0, 1.4 * fps, 1.8 * fps], [330, 330, 0],
        { extrapolateRight: "clamp" });
  const bottom = closing
    ? interpolate(frame, [0, 0.75 * fps, 0.95 * fps, 1.15 * fps],
        [0, 0, 90, 125], { extrapolateRight: "clamp" })
    : interpolate(frame, [0, 1.4 * fps, 1.8 * fps], [225, 225, 60],
        { extrapolateRight: "clamp" });
  return (
    <div style={{ position: "absolute", top: closing ? 440 : 415, left: 40,
      width: 1000, height: closing ? 180 : 280,
      filter: "drop-shadow(0 24px 40px rgba(0,0,0,.36))" }}>
      <Video
        name={closing ? "Real island collapse" : "Real hover opening"}
        src={staticFile(closing ? "bookland/close-source.mp4" : "bookland/hover-source.mp4")}
        muted
        style={{ width: 1000, height: closing ? 180 : 280,
          clipPath: `inset(0px ${side}px ${bottom}px ${side}px round 25px)` }}
      />
    </div>
  );
};

const ActionCapture: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const push = interpolate(frame, [0, 8 * fps], [1, 1.035],
    { extrapolateRight: "clamp" });
  return (
    <div style={{ position: "absolute", top: 345, left: -48,
      width: 1176, height: 666, scale: push,
      borderRadius: 34, overflow: "hidden",
      boxShadow: "0 38px 100px rgba(0,0,0,.44)" }}>
      <Video
        name="Recorded 15 second YouTube seek"
        src={staticFile("bookland/verified-seek.mp4")}
        muted
        style={{ width: 1176, height: 666 }}
      />
    </div>
  );
};

const EndCard: React.FC<{ cta: string }> = ({ cta }) => {
  const frame = useCurrentFrame();
  return (
    <>
      <Img src={staticFile("bookland/logo.png")} style={{
        position: "absolute", width: 310, height: 310, left: 385, top: 285,
        scale: interpolate(frame, [0, 14], [0.88, 1],
          { extrapolateRight: "clamp" }),
        opacity: interpolate(frame, [0, 10], [0, 1],
          { extrapolateRight: "clamp" }),
      }} />
      <div style={{ position: "absolute", top: 720, left: 80, right: 80,
        textAlign: "center", color: "#EAF3FF", fontSize: 79,
        fontWeight: 770, letterSpacing: -3, lineHeight: 1.08,
        opacity: interpolate(frame, [10, 20], [0, 1],
          { extrapolateRight: "clamp" }) }}>{cta}</div>
      <div style={{ position: "absolute", top: 980, left: 100, right: 100,
        textAlign: "center", color: "#9BB9D8", fontSize: 34,
        opacity: interpolate(frame, [18, 30], [0, 1],
          { extrapolateRight: "clamp" }) }}>Free · Open source · GitHub</div>
    </>
  );
};

export const BooklandMusicReal: React.FC<BooklandRealProps> = ({ script }) => {
  const { fps } = useVideoConfig();
  const hook = (script?.hook || "STOP OPENING YOUTUBE.").slice(0, 80);
  const body = (script?.body || "SKIP 15 SECONDS.\nKEEP WORKING.").slice(0, 90);
  const cta = (script?.cta || "COMMENT\nBOOKLAND").slice(0, 60);
  return (
    <AbsoluteFill style={{ overflow: "hidden", background: "#06101E",
      color: "#F4F8FF",
      fontFamily: "Helvetica Neue, Helvetica, Arial, sans-serif" }}>
      <div style={{ position: "absolute", inset: -200,
        background: "radial-gradient(ellipse at 50% 24%, #152B48 0%, #081525 42%, #06101E 78%)" }} />
      <Sequence name="Problem: one quick skip" durationInFrames={4 * fps}>
        <ActionCapture />
        <Header title={hook} subtitle="Just to skip 15 seconds?" duration={4 * fps} />
      </Sequence>
      <Sequence name="Actual hover opening" from={4 * fps} durationInFrames={4 * fps}>
        <HoverCapture />
        <Header title="HOVER TO OPEN." subtitle="The controls live beside the camera." duration={4 * fps} />
      </Sequence>
      <Sequence name="Recorded YouTube seek" from={8 * fps} durationInFrames={8 * fps}>
        <ActionCapture />
        <Header title={body} subtitle="00:45 → 00:30 in the same take." duration={8 * fps} />
      </Sequence>
      <Sequence name="Actual island collapse" from={16 * fps} durationInFrames={2 * fps}>
        <HoverCapture closing />
        <Header title="BACK TO WORK." subtitle="Move away. It closes." duration={2 * fps} />
      </Sequence>
      <Sequence name="Bookland call to action" from={18 * fps} durationInFrames={4 * fps}>
        <EndCard cta={cta} />
      </Sequence>
    </AbsoluteFill>
  );
};
