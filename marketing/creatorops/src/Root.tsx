import { MyComposition } from "./Composition";
import { Composition } from "remotion";
import { BooklandMusicReal } from "./BooklandReal";

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <MyComposition />
      <Composition
        id="BooklandMusicReal"
        component={BooklandMusicReal}
        fps={30}
        width={1080}
        height={1920}
        durationInFrames={660}
        defaultProps={{
          feature: "music",
          script: {
            hook: "STOP OPENING YOUTUBE.",
            body: "SKIP 15 SECONDS.\nKEEP WORKING.",
            cta: "COMMENT\nBOOKLAND",
          },
        }}
      />
    </>
  );
};
