import { convertFileSrc } from "@tauri-apps/api/core";
import type { Art } from "./bindings/Art";
import type { Command } from "./bindings/Command";
import type { PlayerState } from "./bindings/PlayerState";
import { getState, openSpotify, sendCommand, subscribeState } from "./ipc";

const TOAST_DURATION_MS = 3000;

export function resolveArtSrc(art: Art | null | undefined): string | null {
  if (!art) return null;
  if (art.kind === "url") return art.url;
  const isTauri = typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;
  return isTauri ? convertFileSrc(art.path) : art.path;
}

export function formatTime(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

class PlayerStore {
  playerState = $state<PlayerState | null>(null);
  toastMessage = $state<string | null>(null);
  displayPositionMs = $state<number>(0);
  isHidden = $state<boolean>(false);
  reducedMotion = $state<boolean>(false);

  private rafId: number | null = null;
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  private computePositionMs(state: PlayerState | null): number {
    if (!state || !state.spotifyRunning || state.playback === "stopped" || !state.position) {
      return 0;
    }
    const base = state.position.ms;
    const extrapolated =
      state.playback === "playing"
        ? base + Math.max(0, Date.now() - state.position.sampledAtUnixMs)
        : base;
    const duration = state.track?.durationMs;
    return duration != null ? Math.min(extrapolated, duration) : extrapolated;
  }

  private stopLoop(): void {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  private syncPositionLoop(): void {
    this.stopLoop();
    this.displayPositionMs = this.computePositionMs(this.playerState);
    if (
      this.isHidden ||
      !this.playerState?.spotifyRunning ||
      this.playerState.playback !== "playing"
    ) {
      return;
    }
    const tick = (): void => {
      this.displayPositionMs = this.computePositionMs(this.playerState);
      if (
        !this.isHidden &&
        this.playerState?.spotifyRunning &&
        this.playerState.playback === "playing"
      ) {
        this.rafId = requestAnimationFrame(tick);
      } else {
        this.rafId = null;
      }
    };
    this.rafId = requestAnimationFrame(tick);
  }

  showToast(message: string): void {
    this.toastMessage = message;
    if (this.toastTimer !== null) {
      clearTimeout(this.toastTimer);
    }
    this.toastTimer = setTimeout(() => {
      this.toastMessage = null;
      this.toastTimer = null;
    }, TOAST_DURATION_MS);
  }

  async runCommand(cmd: Command): Promise<void> {
    try {
      await sendCommand(cmd);
    } catch (cause: unknown) {
      const msg = cause instanceof Error ? cause.message : String(cause);
      this.showToast(msg);
    }
  }

  async runOpenSpotify(): Promise<void> {
    try {
      await openSpotify();
    } catch (cause: unknown) {
      const msg = cause instanceof Error ? cause.message : String(cause);
      this.showToast(msg);
    }
  }

  init(): () => void {
    let active = true;
    let unlisten: (() => void) | undefined;

    const applyState = (next: PlayerState): void => {
      if (!active) return;
      this.playerState = next;
      this.syncPositionLoop();
    };

    const onVisibilityChange = (): void => {
      this.isHidden = document.visibilityState === "hidden";
      this.syncPositionLoop();
    };
    this.isHidden = typeof document !== "undefined" && document.visibilityState === "hidden";
    document.addEventListener("visibilitychange", onVisibilityChange);

    const motionQuery =
      typeof window !== "undefined" && typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-reduced-motion: reduce)")
        : null;
    this.reducedMotion = motionQuery?.matches ?? false;
    const onMotionChange = (event: MediaQueryListEvent): void => {
      this.reducedMotion = event.matches;
    };
    motionQuery?.addEventListener("change", onMotionChange);

    void getState()
      .then(applyState)
      .catch((cause: unknown) => {
        if (active) {
          this.showToast(cause instanceof Error ? cause.message : String(cause));
        }
      });

    void subscribeState(applyState)
      .then((stop) => {
        if (active) unlisten = stop;
        else stop();
      })
      .catch((cause: unknown) => {
        if (active) {
          this.showToast(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      active = false;
      unlisten?.();
      this.stopLoop();
      if (this.toastTimer !== null) {
        clearTimeout(this.toastTimer);
        this.toastTimer = null;
      }
      document.removeEventListener("visibilitychange", onVisibilityChange);
      motionQuery?.removeEventListener("change", onMotionChange);
    };
  }
}

export const playerStore = new PlayerStore();
