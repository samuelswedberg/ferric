import mockPlinthUrl from "../mock-assets/plinth.png";
import mockGrainUrl from "../mock-assets/grain.png";
import mockCardUrl from "../mock-assets/card.png";

const staticTextures = import.meta.glob<string>("../assets/textures/*.png", {
  eager: true,
  import: "default",
});

function pickTexture(filename: string, fallback: string): string {
  const match = Object.entries(staticTextures).find(([path]) => path.endsWith(`/${filename}`));
  return match ? match[1] : fallback;
}

export const plinthTextureUrl = pickTexture("plinth.png", mockPlinthUrl);
export const grainTextureUrl = pickTexture("grain.png", mockGrainUrl);
export const cardTextureUrl = pickTexture("card.png", mockCardUrl);
