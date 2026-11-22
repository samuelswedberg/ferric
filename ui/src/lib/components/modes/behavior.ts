import type { Mode } from "../../bindings/Mode";

export const VOLUME_STEP = 5;
export const VOLUME_RESET_MS = 1_500;
export const UNKNOWN_VOLUME_RESET_MS = 1_000;

export function nextMode(mode: Mode): Mode {
  if (mode === "full") return "card";
  if (mode === "card") return "strip";
  return "full";
}

export function stepVolume(current: number, direction: -1 | 1): number {
  return Math.min(100, Math.max(0, current + direction * VOLUME_STEP));
}

export function queueShouldBeVisible(mode: Mode, documentHidden: boolean): boolean {
  return mode === "full" && !documentHidden;
}
