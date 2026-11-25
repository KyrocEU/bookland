import React from "react";
import { AbsoluteFill, Composition, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

export type VideoProps = {
  brand?: string | { name?: string };
  topic?: string;
  script?: string | { hook?: string; body?: string; cta?: string; voiceover?: string; on_screen_text?: string[]; duration_seconds?: number };
  shotPlan?: { start_s?: number; end_s?: number; on_screen_text?: string }[];
  durationSeconds?: number;
};

const defaults: VideoProps = {
  brand: "Bookland",
  topic: "Your work, one hover away",
  script: { hook: "What if your Mac's island worked for you?", body: "See your agent's progress, music, calendar and files in one place.", cta: "Comment BOOKLAND for the link." },
  durationSeconds: 25,
};

const linesOf = (props: VideoProps): string[] => {
  const script = props.script;
  if (typeof script === "string") {
    const parts = script.split(/(?<=[.!?])\s+/).filter(Boolean);
    return [parts[0] || props.topic || "", parts.slice(1, -1).join(" ") || props.topic || "", parts[parts.length - 1] || "Discover more"];
  }
  const s = script || {};
  if (s.on_screen_text?.length) {
    const lines = s.on_screen_text.filter(Boolean);
    return [lines[0] || props.topic || "", lines[Math.floor(lines.length / 2)] || props.topic || "", lines[lines.length - 1] || "Discover more"];
  }
  if (s.voiceover) {
    const parts = s.voiceover.split(/(?<=[.!?])\s+/).filter(Boolean);
    return [parts[0] || props.topic || "", parts.slice(1, -1).join(" ") || props.topic || "", parts[parts.length - 1] || "Discover more"];
  }
  return [s.hook || props.topic || "", s.body || props.topic || "", s.cta || "Discover more"];
};

const Video: React.FC<VideoProps> = (props) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();
  const brand = (typeof props.brand === "string" ? props.brand : props.brand?.name || "Bookland").slice(0, 32);
  const [hook, body, cta] = linesOf(props);
  const progress = frame / durationInFrames;
  const open = spring({ frame: frame - 5, fps, config: { damping: 200, stiffness: 90 } });
  const close = spring({ frame: frame - durationInFrames * 0.68, fps, config: { damping: 200, stiffness: 90 } });
  const expansion = Math.max(0, Math.min(1, open - close));
  const phase = progress < 0.31 ? 0 : progress < 0.7 ? 1 : 2;
  const enter = spring({ frame: frame - [0, 0.31, 0.7][phase] * durationInFrames - 4, fps, config: { damping: 200 } });
  const exit = spring({ frame: frame - [0.31, 0.7, 1][phase] * durationInFrames + 13, fps, config: { damping: 200 } });
  const opacity = Math.max(0, Math.min(1, enter - exit));
  return (
    <AbsoluteFill style={{ background: "#05070D", color: "white", fontFamily: "-apple-system, BlinkMacSystemFont, Helvetica Neue, Arial, sans-serif", overflow: "hidden" }}>
      <div style={{ position: "absolute", inset: -300, background: "radial-gradient(ellipse at 50% 28%, #1F385A 0%, #080D19 35%, #05070D 70%)" }} />
      <div style={{ position: "absolute", top: 0, left: "50%", width: 350, height: 26, transform: "translateX(-50%)", background: "#000", borderRadius: "0 0 18px 18px" }} />
      <div style={{
        position: "absolute", top: 108, left: "50%", transform: "translateX(-50%)",
        width: interpolate(expansion, [0, 1], [310, 850]),
        height: interpolate(expansion, [0, 1], [84, 560]),
        borderRadius: interpolate(expansion, [0, 1], [42, 52]),
        background: "linear-gradient(145deg, rgba(38,47,63,.91), rgba(8,12,23,.97) 44%, rgba(24,40,57,.94))",
        boxShadow: "0 50px 130px rgba(0,0,0,.58), inset 0 1px 1px rgba(255,255,255,.43)",
        border: "1px solid rgba(255,255,255,.25)", overflow: "hidden",
      }}>
        <div style={{ position: "absolute", inset: 0, background: "radial-gradient(ellipse at 25% -20%, rgba(135,205,255,.36), transparent 55%)" }} />
        <div style={{ position: "absolute", top: 30, left: 35, display: "flex", alignItems: "center", gap: 17 }}>
          <div style={{ width: 20, height: 20, borderRadius: 10, background: "#70D8FF", boxShadow: "0 0 24px #70D8FF" }} />
          <span style={{ fontSize: 20, letterSpacing: 2, fontWeight: 700, color: "#E8F8FF" }}>{brand.toUpperCase()}</span>
        </div>
        <div style={{ position: "absolute", top: 32, right: 38, fontSize: 19, color: "#A5EBFF" }}>● &nbsp; Agent active</div>
        <div style={{ opacity: expansion, position: "absolute", top: 145, left: 52, right: 52 }}>
          <div style={{ fontSize: 23, color: "#B8D4E5", marginBottom: 25 }}>YOUR WORKSPACE, AT A GLANCE</div>
          <div style={{ display: "flex", gap: 18 }}>
            {([["◉", "Agent", "Working"], ["♫", "Music", "Playing"], ["▦", "Calendar", "Today"]] as const).map(([icon, title, caption]) => (
              <div key={title} style={{ flex: 1, height: 170, borderRadius: 28, padding: 22, background: "rgba(255,255,255,.09)", border: "1px solid rgba(255,255,255,.15)" }}>
                <div style={{ fontSize: 43, color: "#7DDAFF" }}>{icon}</div>
                <div style={{ marginTop: 16, fontSize: 23, color: "white", fontWeight: 650 }}>{title}</div>
                <div style={{ fontSize: 19, color: "#A9BED0" }}>{caption}</div>
              </div>
            ))}
          </div>
          <div style={{ marginTop: 26, height: 90, borderRadius: 24, padding: "23px 28px", boxSizing: "border-box", fontSize: 23, color: "#D4F2FF", background: "rgba(130,205,255,.12)" }}>Your agent is building the next idea &nbsp; ◌</div>
        </div>
      </div>
      <div style={{ position: "absolute", left: 84, right: 84, top: 780, opacity, transform: `translateY(${(1 - enter) * 36}px)`, textAlign: "center" }}>
        <div style={{ fontSize: 25, letterSpacing: 8, color: "#80D8FF", marginBottom: 36, fontWeight: 700 }}>{brand.toUpperCase()}</div>
        <div style={{ fontSize: phase === 1 ? 69 : 80, lineHeight: 1.12, letterSpacing: -3.5, fontWeight: 750, textShadow: "0 10px 45px rgba(0,0,0,.7)" }}>{[hook, body, cta][phase]}</div>
      </div>
      <div style={{ position: "absolute", bottom: 70, left: 90, right: 90, height: 3, background: "rgba(255,255,255,.18)", borderRadius: 2 }}>
        <div style={{ height: "100%", width: `${progress * 100}%`, background: "#9FE6FF", borderRadius: 2 }} />
      </div>
    </AbsoluteFill>
  );
};

export const MyComposition = () => (
  <Composition
    id="CreatorVertical"
    component={Video}
    fps={30}
    width={1080}
    height={1920}
    durationInFrames={750}
    defaultProps={defaults}
    calculateMetadata={({ props }) => ({ durationInFrames: Math.round(Math.max(20, Math.min(30, Number(props.durationSeconds) || 25)) * 30) })}
  />
);
