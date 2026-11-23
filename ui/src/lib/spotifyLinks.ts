import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { albumLink, artistLink, trackLink, type SpotifyLink } from "./spotifyLinkUrls";

export { albumLink, artistLink, trackLink, type SpotifyLink };

/**
 * Opens the link in the Spotify desktop app inside Ferric, falling back to the web URL.
 * In a plain browser (the mock preview) it opens the web URL in a new tab.
 * Only spotify: and https://open.spotify.com/ URLs are allowed by the app's capability.
 */
export async function openSpotifyLink(link: SpotifyLink): Promise<void> {
  if (!isTauri()) {
    window.open(link.web, "_blank", "noopener,noreferrer");
    return;
  }
  try {
    await openUrl(link.app);
  } catch (appError) {
    try {
      await openUrl(link.web);
    } catch (webError) {
      console.error("Could not open Spotify link", { appError, webError });
    }
  }
}
