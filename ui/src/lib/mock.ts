import type { Command } from "./bindings/Command";
import type { Mode } from "./bindings/Mode";
import type { PlayerState } from "./bindings/PlayerState";
import type { Theme } from "./bindings/Theme";
import type { Track } from "./bindings/Track";
import recordEmberUrl from "./mock-assets/record-ember.png";
import sleeveEmberUrl from "./mock-assets/sleeve-ember.png";

type StateListener = (state: PlayerState) => void;

const tracks: Track[] = [
  {
    spotifyId: "mock-ember-season",
    title: "Ember Season",
    artists: ["Low Hollow"],
    album: "Ember Season",
    durationMs: 221_000,
    art: { kind: "url", url: sleeveEmberUrl },
  },
  {
    spotifyId: "mock-glass-harbor",
    title: "Glass Harbor",
    artists: ["Maren Sol"],
    album: "Low Tide",
    durationMs: 201_000,
    art: { kind: "url", url: sleeveEmberUrl },
  },
  {
    spotifyId: "mock-northbound",
    title: "Northbound",
    artists: ["The Cedar Line"],
    album: "Milepost 9",
    durationMs: 187_000,
    art: { kind: "url", url: sleeveEmberUrl },
  },
];

const themes: Theme[] = [
  {
    albumKey: "mock-ember",
    accent: { r: 200, g: 85, b: 61 },
    onAccent: { r: 237, g: 228, b: 212 },
    sleeve: { kind: "url", url: sleeveEmberUrl },
    record: { kind: "url", url: recordEmberUrl },
  },
  {
    albumKey: "mock-glass-harbor",
    accent: { r: 92, g: 158, b: 173 },
    onAccent: { r: 237, g: 228, b: 212 },
    sleeve: { kind: "url", url: sleeveEmberUrl },
    record: { kind: "url", url: recordEmberUrl },
  },
  {
    albumKey: "mock-northbound",
    accent: { r: 138, g: 168, b: 102 },
    onAccent: { r: 237, g: 228, b: 212 },
    sleeve: { kind: "url", url: sleeveEmberUrl },
    record: { kind: "url", url: recordEmberUrl },
  },
];

const mockMode =
  typeof window !== "undefined" ? new URLSearchParams(window.location.search).get("mock") : null;

const copperLight: Track = {
  spotifyId: "mock-copper-light",
  title: "Copper Light",
  artists: ["Juniper Vale"],
  album: "Afterglow",
  durationMs: 227_000,
  art: { kind: "url", url: sleeveEmberUrl },
};
const queueSeed = [tracks[1], tracks[2], copperLight, tracks[0]];
const queue = Array.from({ length: 12 }, (_, index) => queueSeed[index % queueSeed.length]);
const listeners = new Set<StateListener>();
let trackIndex = 0;

function makeInitialState(): PlayerState {
  const now = Date.now();
  const base: PlayerState = {
    spotifyRunning: true,
    track: tracks[0],
    playback: "playing",
    position: { ms: 84_000, sampledAtUnixMs: now },
    volumePct: 72,
    liked: true,
    queue,
    auth: { status: "signedIn", displayName: "Ferric Listener", premium: true },
    theme: themes[0],
    mode: "full",
  };

  switch (mockMode) {
    case "card":
      return { ...base, mode: "card" };
    case "strip":
      return { ...base, mode: "strip" };
    case "volumeZero":
      return { ...base, mode: "card", volumePct: 0 };
    case "signedOut":
      return {
        ...base,
        queue: null,
        volumePct: null,
        liked: null,
        auth: { status: "signedOut" },
      };
    case "unknownVolume":
      return { ...base, mode: "strip", volumePct: null };
    case "paused":
      return { ...base, playback: "paused" };
    case "stopped":
      return { ...base, playback: "stopped", position: { ms: 0, sampledAtUnixMs: now } };
    case "notRunning":
      return {
        ...base,
        spotifyRunning: false,
        playback: "stopped",
        track: null,
        position: null,
        volumePct: null,
        liked: null,
        theme: null,
      };
    default:
      return base;
  }
}

let state: PlayerState = makeInitialState();

function cloneState(): PlayerState {
  return structuredClone(state);
}

function publish(): void {
  const snapshot = cloneState();
  for (const listener of listeners) listener(snapshot);
}

function currentPositionMs(): number {
  if (!state.position) return 0;
  if (state.playback !== "playing") return state.position.ms;
  const elapsed = Math.max(0, Date.now() - state.position.sampledAtUnixMs);
  const raw = state.position.ms + elapsed;
  return state.track?.durationMs != null ? Math.min(raw, state.track.durationMs) : raw;
}

if (mockMode === "cycle") {
  setInterval(() => {
    trackIndex = (trackIndex + 1) % tracks.length;
    state = {
      ...state,
      track: tracks[trackIndex],
      position: { ms: 0, sampledAtUnixMs: Date.now() },
      liked: trackIndex % 2 === 0,
      theme: themes[trackIndex],
    };
    publish();
  }, 5_000);
}

export const mockBackend = {
  getState(): PlayerState {
    return cloneState();
  },

  sendCommand(command: Command): void {
    if (mockMode === "error") {
      throw new Error("Spotify Premium is required for this");
    }

    const now = Date.now();
    switch (command.type) {
      case "playPause": {
        if (state.playback === "playing") {
          state = {
            ...state,
            playback: "paused",
            position: { ms: currentPositionMs(), sampledAtUnixMs: now },
          };
        } else {
          const resumeMs = state.playback === "stopped" ? 0 : (state.position?.ms ?? 0);
          state = {
            ...state,
            playback: "playing",
            position: { ms: resumeMs, sampledAtUnixMs: now },
          };
        }
        break;
      }
      case "next":
        trackIndex = (trackIndex + 1) % tracks.length;
        state = {
          ...state,
          track: tracks[trackIndex],
          theme: themes[trackIndex],
          position: { ms: 0, sampledAtUnixMs: now },
          liked: trackIndex % 2 === 0,
        };
        break;
      case "previous":
        trackIndex = (trackIndex + tracks.length - 1) % tracks.length;
        state = {
          ...state,
          track: tracks[trackIndex],
          theme: themes[trackIndex],
          position: { ms: 0, sampledAtUnixMs: now },
          liked: trackIndex % 2 === 0,
        };
        break;
      case "seek":
        state = {
          ...state,
          position: { ms: command.positionMs, sampledAtUnixMs: now },
        };
        break;
      case "setVolume":
        state = { ...state, volumePct: Math.min(100, Math.max(0, command.pct)) };
        break;
      case "setLiked":
        state = { ...state, liked: command.liked };
        break;
      case "jumpTo":
        state = {
          ...state,
          track: state.queue?.[command.queueIndex] ?? state.track,
          position: { ms: 0, sampledAtUnixMs: now },
        };
        break;
    }
    publish();
  },

  setMode(mode: Mode): void {
    state = { ...state, mode };
    publish();
  },

  setQueueVisible(visible: boolean): void {
    console.info(`[mock] set_queue_visible(${visible})`);
  },

  openSpotify(): void {
    trackIndex = 0;
    state = {
      ...state,
      spotifyRunning: true,
      playback: "stopped",
      track: tracks[0],
      position: { ms: 0, sampledAtUnixMs: Date.now() },
      volumePct: 72,
      liked: true,
      theme: themes[0],
    };
    publish();
  },

  setClientId(_clientId: string): void {
    state = { ...state, auth: { status: "signedOut" } };
    publish();
  },

  signIn(): void {
    state = {
      ...state,
      auth: { status: "signedIn", displayName: "Ferric Listener", premium: true },
    };
    publish();
  },

  signOut(): void {
    state = { ...state, auth: { status: "signedOut" }, queue: null, liked: null, volumePct: null };
    publish();
  },

  subscribe(listener: StateListener): () => void {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};
