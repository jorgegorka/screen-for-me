import { Composition } from "remotion";
import { Promo } from "./Promo";
import { LOCALES } from "./copy";

// One composition per locale: `ScreenForMePromo` is the original en-GB cut,
// the rest render as `ScreenForMePromo-<locale>`.
export const MyComposition = () => {
  return (
    <>
      {LOCALES.map((locale) => (
        <Composition
          key={locale}
          id={
            locale === "en-GB" ? "ScreenForMePromo" : `ScreenForMePromo-${locale}`
          }
          component={Promo}
          defaultProps={{ locale }}
          durationInFrames={1800}
          fps={30}
          width={1920}
          height={1080}
        />
      ))}
    </>
  );
};
