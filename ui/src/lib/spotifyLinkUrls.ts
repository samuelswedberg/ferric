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

/** Opens the primary artist directly when the Web API gave its id, else searches by artist name. */
export function artistLink(track: Track): SpotifyLink | null {
  if (track.artistId) {
    const id = encodeURIComponent(track.artistId);
    return { app: `spotify:artist:${id}`, web: `https://open.spotify.com/artist/${id}` };
  }
  const primary = track.artists[0]?.trim();
  if (!primary) return null;
  return searchLink(primary, "/artists");
}

/** Opens the album directly when the Web API gave its id, else searches album name + first artist. */
export function albumLink(track: Track): SpotifyLink {
  if (track.albumId) {
    const id = encodeURIComponent(track.albumId);
    return { app: `spotify:album:${id}`, web: `https://open.spotify.com/album/${id}` };
  }
  return searchLink(`${track.album ?? track.title} ${track.artists[0] ?? ""}`, "/albums");
}
