import type { Command } from "./bindings/Command";
import type { Mode } from "./bindings/Mode";
import type { PlayerState } from "./bindings/PlayerState";

type StateListener = (state: PlayerState) => void;

const tracks: NonNullable<PlayerState["track"]>[] = [
  {
    spotifyId: "0VjIjW4GlUZAMYd2vXMi3b",
    title: "Blinding Lights",
    artists: ["The Weeknd"],
    album: "After Hours",
    durationMs: 200_040,
    art: { kind: "url", url: "https://i.scdn.co/image/ab67616d0000b2738863bc11d2aa12b54f5aeb36" },
  },
  {
    spotifyId: "1BxfuPKGuaTgP7aM0Bbdwr",
    title: "Cruel Summer",
    artists: ["Taylor Swift"],
    album: "Lover",
    durationMs: 178_426,
    art: { kind: "url", url: "https://i.scdn.co/image/ab67616d0000b273e787cffec20aa2a396a61647" },
  },
  {
    spotifyId: "4cOdK2wGLETKBW3PvgPWqT",
    title: "Never Gonna Give You Up",
    artists: ["Rick Astley"],
    album: "Whenever You Need Somebody",
    durationMs: 213_573,
    art: { kind: "url", url: "https://i.scdn.co/image/ab67616d0000b27315ebbedaacef61af244262a8" },
  },
];

const queue = Array.from({ length: 20 }, (_, index) => tracks[(index + 1) % tracks.length]);
const listeners = new Set<StateListener>();
let trackIndex = 0;

let state: PlayerState = {
  spotifyRunning: true,
  track: tracks[trackIndex],
  playback: "playing",
  position: { ms: 42_000, sampledAtUnixMs: Date.now() },
  volumePct: 72,
  liked: false,
  queue,
  auth: { status: "signedIn", displayName: "Ferric Listener", premium: true },
  theme: {
    albumKey: "mock-after-hours",
    accent: { r: 214, g: 68, b: 86 },
    onAccent: { r: 255, g: 255, b: 255 },
    sleeve: null,
    record: null,
  },
  mode: "full",
};

function cloneState(): PlayerState {
  return structuredClone(state);
}

function publish(): void {
  const snapshot = cloneState();
  for (const listener of listeners) listener(snapshot);
}

setInterval(() => {
  trackIndex = (trackIndex + 1) % tracks.length;
  state = {
    ...state,
    track: tracks[trackIndex],
    position: { ms: 0, sampledAtUnixMs: Date.now() },
    liked: trackIndex % 2 === 0,
    theme: state.theme ? { ...state.theme, albumKey: `mock-${trackIndex}` } : null,
  };
  publish();
}, 5_000);

export const mockBackend = {
  getState(): PlayerState {
    return cloneState();
  },

  sendCommand(command: Command): void {
    switch (command.type) {
      case "playPause":
        state = { ...state, playback: state.playback === "playing" ? "paused" : "playing" };
        break;
      case "next":
        trackIndex = (trackIndex + 1) % tracks.length;
        state = { ...state, track: tracks[trackIndex] };
        break;
      case "previous":
        trackIndex = (trackIndex + tracks.length - 1) % tracks.length;
        state = { ...state, track: tracks[trackIndex] };
        break;
      case "seek":
        state = { ...state, position: { ms: command.positionMs, sampledAtUnixMs: Date.now() } };
        break;
      case "setVolume":
        state = { ...state, volumePct: Math.min(100, Math.max(0, command.pct)) };
        break;
      case "setLiked":
        state = { ...state, liked: command.liked };
        break;
      case "jumpTo":
        state = { ...state, track: state.queue?.[command.queueIndex] ?? state.track };
        break;
    }
    publish();
  },

  setMode(mode: Mode): void {
    state = { ...state, mode };
    publish();
  },

  setQueueVisible(_visible: boolean): void {},

  openSpotify(): void {
    state = { ...state, spotifyRunning: true };
    publish();
  },

  setClientId(_clientId: string): void {
    state = { ...state, auth: { status: "signedOut" } };
    publish();
  },

  signIn(): void {
    state = { ...state, auth: { status: "signedIn", displayName: "Ferric Listener", premium: true } };
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
