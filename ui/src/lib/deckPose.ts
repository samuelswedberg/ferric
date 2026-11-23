import type { Playback } from "./bindings/Playback";

export interface TonearmInput {
  spotifyRunning: boolean;
  playback: Playback;
  progress: number;
  hovered: boolean;
}

export interface TonearmPose {
  angleDeg: number;
  lifted: boolean;
  transition: "rest" | "hover" | "play";
}

/**
 * Computes the turntable tonearm angle and lift state:
 * - Playing: arm tracks progress across the grooves (21°..34°); hovering lifts and
 *   nudges it slightly back toward the arm rest to preview pausing.
 * - Paused: arm fully rests on the arm rest (0°, not lifted); hovering lifts and
 *   swings it slightly toward the record to preview resuming.
 * - Stopped: arm rests at 0°; hovering nudges slightly toward the record.
 */
export function computeTonearmPose({
  spotifyRunning,
  playback,
  progress,
  hovered,
}: TonearmInput): TonearmPose {
  if (!spotifyRunning) {
    return { angleDeg: 0, lifted: false, transition: "rest" };
  }

  const clampedProgress = Math.min(1, Math.max(0, progress));
  const playingAngle = 21 + 13 * clampedProgress;

  if (playback === "playing") {
    if (hovered) {
      return { angleDeg: playingAngle - 5, lifted: true, transition: "hover" };
    }
    return { angleDeg: playingAngle, lifted: false, transition: "play" };
  }

  if (playback === "paused") {
    if (hovered) {
      return { angleDeg: 8, lifted: true, transition: "hover" };
    }
    return { angleDeg: 0, lifted: false, transition: "rest" };
  }

  if (hovered) {
    return { angleDeg: 6, lifted: true, transition: "hover" };
  }
  return { angleDeg: 0, lifted: false, transition: "rest" };
}
