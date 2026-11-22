import { convertFileSrc } from "@tauri-apps/api/core";
import type { Art } from "./bindings/Art";
import type { Command } from "./bindings/Command";
import type { PlayerState } from "./bindings/PlayerState";
import {
  getConfig,
  getState,
  openSpotify,
  sendCommand,
  subscribeConfig,
  subscribeError,
  subscribeIdleOpacity,
  subscribeState,
  type ConfigPayload,
} from "./ipc";

const TOAST_DURATION_MS = 3000;
/** How long a just-made seek overrides the reported position while Spotify catches up. */
const SEEK_HOLD_MS = 3000;

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
  grainOpacity = $state<number>(0.2);
  idleOpacity = $state<number>(1);

  private rafId: number | null = null;
  /** Volume requests go out one at a time; while one is in flight only the latest
   *  wanted value is kept, so a fast drag doesn't queue a burst of stale requests. */
  private volumeInFlight = false;
  private pendingVolume: number | null = null;
  /** A seek the user just made, shown until the backend reports a newer position. */
  private pendingSeek: { ms: number; at: number } | null = null;
  private toastTimer: ReturnType<typeof setTimeout> | null = null;

  private computePositionMs(state: PlayerState | null): number {
    if (!state || !state.spotifyRunning || state.playback === "stopped" || !state.position) {
      return 0;
    }
    let base = state.position.ms;
    let sampledAt = state.position.sampledAtUnixMs;
    const seek = this.pendingSeek;
    if (seek && sampledAt < seek.at && Date.now() - seek.at < SEEK_HOLD_MS) {
      base = seek.ms;
      sampledAt = seek.at;
    } else if (seek) {
      this.pendingSeek = null;
    }
    const extrapolated =
      state.playback === "playing" ? base + Math.max(0, Date.now() - sampledAt) : base;
    const duration = state.track?.durationMs;
    return duration != null ? Math.min(extrapolated, duration) : extrapolated;
  }

  private stopLoop(): void {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
  }

  /** Progress moves in whole-second steps: the time text, slider cap and played line all
   *  change together, instead of the cap gliding ahead of the line. */
  private steppedPositionMs(): number {
    return Math.floor(this.computePositionMs(this.playerState) / 1000) * 1000;
  }

  private syncPositionLoop(): void {
    this.stopLoop();
    this.displayPositionMs = this.steppedPositionMs();
    if (
      this.isHidden ||
      !this.playerState?.spotifyRunning ||
      this.playerState.playback !== "playing"
    ) {
      return;
    }
    const tick = (): void => {
      const next = this.steppedPositionMs();
      if (next !== this.displayPositionMs) this.displayPositionMs = next;
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

  private applyConfig(config: ConfigPayload): void {
    this.grainOpacity = Number.isFinite(config.grain)
      ? Math.min(0.5, Math.max(0, config.grain))
      : 0.2;
    for (const error of config.errors) this.showToast(error);
  }

  async runCommand(cmd: Command): Promise<void> {
    if (cmd.type === "setVolume") {
      this.pendingVolume = cmd.pct;
      if (this.volumeInFlight) return;
      this.volumeInFlight = true;
      try {
        while (this.pendingVolume !== null) {
          const pct = this.pendingVolume;
          this.pendingVolume = null;
          await this.sendOrToast({ type: "setVolume", pct });
        }
      } finally {
        this.volumeInFlight = false;
      }
      return;
    }
    if (cmd.type === "seek") {
      this.pendingSeek = { ms: cmd.positionMs, at: Date.now() };
      this.syncPositionLoop();
    }
    try {
      await sendCommand(cmd);
    } catch (cause: unknown) {
      if (cmd.type === "seek") {
        this.pendingSeek = null;
        this.syncPositionLoop();
      }
      const msg = cause instanceof Error ? cause.message : String(cause);
      this.showToast(msg);
    }
  }

  private async sendOrToast(cmd: Command): Promise<void> {
    try {
      await sendCommand(cmd);
    } catch (cause: unknown) {
      this.showToast(cause instanceof Error ? cause.message : String(cause));
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
    let unlistenConfig: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;
    let unlistenIdle: (() => void) | undefined;
    let configRevision = 0;

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

    const configRevisionAtLoad = configRevision;
    void getConfig()
      .then((config) => {
        if (active && configRevision === configRevisionAtLoad) this.applyConfig(config);
      })
      .catch((cause: unknown) => {
        if (active) this.showToast(cause instanceof Error ? cause.message : String(cause));
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

    void subscribeConfig((config) => {
      if (active) {
        configRevision += 1;
        this.applyConfig(config);
      }
    })
      .then((stop) => {
        if (active) unlistenConfig = stop;
        else stop();
      })
      .catch((cause: unknown) => {
        if (active) this.showToast(cause instanceof Error ? cause.message : String(cause));
      });

    void subscribeError((message) => {
      if (active) this.showToast(message);
    })
      .then((stop) => {
        if (active) unlistenError = stop;
        else stop();
      })
      .catch((cause: unknown) => {
        if (active) this.showToast(cause instanceof Error ? cause.message : String(cause));
      });

    void subscribeIdleOpacity((opacity) => {
      if (active) this.idleOpacity = Number.isFinite(opacity) ? Math.min(1, Math.max(0, opacity)) : 1;
    })
      .then((stop) => {
        if (active) unlistenIdle = stop;
        else stop();
      })
      .catch((cause: unknown) => {
        if (active) this.showToast(cause instanceof Error ? cause.message : String(cause));
      });

    return () => {
      active = false;
      unlisten?.();
      unlistenIdle?.();
      unlistenConfig?.();
      unlistenError?.();
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
