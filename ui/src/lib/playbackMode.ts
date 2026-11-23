import type { RepeatMode } from "./bindings/RepeatMode";

export type PlaybackModeKey = "off" | "shuffle" | "repeatContext" | "repeatTrack";

export interface PlaybackModeTarget {
  shuffle: boolean;
  repeat: RepeatMode;
}

/**
 * Collapses Spotify's separate shuffle and repeat flags into the single button's
 * visual state. Repeat-track takes precedence over shuffle because repeating one
 * track makes shuffle irrelevant; shuffle otherwise takes precedence over
 * repeat-context so clicking advances cleanly into Repeat Context (shuffle off).
 */
export function derivePlaybackMode(
  shuffle: boolean | null,
  repeat: RepeatMode | null,
): PlaybackModeKey | null {
  if (shuffle === null || repeat === null) return null;
  if (repeat === "track") return "repeatTrack";
  if (shuffle) return "shuffle";
  if (repeat === "context") return "repeatContext";
  return "off";
}

/**
 * Advances through the 3-state cycle:
 * Shuffle -> Repeat album/playlist (context) -> Repeat song (track) -> Shuffle.
 * If Spotify starts with both off (or unknown), the first click enters Shuffle.
 */
export function nextPlaybackMode(current: PlaybackModeKey | null): PlaybackModeTarget {
  switch (current) {
    case "shuffle":
      return { shuffle: false, repeat: "context" };
    case "repeatContext":
      return { shuffle: false, repeat: "track" };
    case "repeatTrack":
    case "off":
    case null:
      return { shuffle: true, repeat: "off" };
  }
}

export function playbackModeLabel(mode: PlaybackModeKey | null): string {
  switch (mode) {
    case "shuffle":
      return "Shuffle on — switch to repeat album";
    case "repeatContext":
      return "Repeat album on — switch to repeat song";
    case "repeatTrack":
      return "Repeat song on — switch to shuffle";
    case "off":
      return "Normal order — turn on shuffle";
    case null:
      return "Shuffle and repeat";
  }
}
