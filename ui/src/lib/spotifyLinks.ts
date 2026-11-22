import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Track } from "./bindings/Track";

/**
 * A Spotify destination as a desktop-app URI (preferred, since Ferric runs next to the
 * Spotify app) and an open.spotify.com URL (used in the browser preview and as a fallback).
 */
export interface SpotifyLink {
  app: string;
  web: string;
}

function searchLink(query: string, webSuffix = ""): SpotifyLink {
  const q = encodeURIComponent(query.trim());
  return {
    app: `spotify:search:${q}`,
    web: `https://open.spotify.com/search/${q}${webSuffix}`,
  };
}

export function trackLink(track: Track): SpotifyLink {
  if (track.spotifyId) {
    const id = encodeURIComponent(track.spotifyId);
    return { app: `spotify:track:${id}`, web: `https://open.spotify.com/track/${id}` };
  }
  return searchLink(`${track.title} ${track.artists.join(" ")}`);
}

/** Track has no album id yet, so albums open as a search (album name + first artist). */
export function albumLink(track: Track): SpotifyLink {
  return searchLink(`${track.album ?? track.title} ${track.artists[0] ?? ""}`, "/albums");
}

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
