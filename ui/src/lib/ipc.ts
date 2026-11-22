import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { Command } from "./bindings/Command";
import type { Mode } from "./bindings/Mode";
import type { PlayerState } from "./bindings/PlayerState";
import { mockBackend } from "./mock";

const STATE_EVENT = "ferric://state";
const CONFIG_EVENT = "ferric://config";
const ERROR_EVENT = "ferric://error";
const IDLE_OPACITY_EVENT = "ferric://idle-opacity";
const isTauri = typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;

export interface ConfigPayload {
  grain: number;
  errors: string[];
}

export async function getState(): Promise<PlayerState> {
  return isTauri ? invoke<PlayerState>("get_state") : mockBackend.getState();
}

export async function getConfig(): Promise<ConfigPayload> {
  return isTauri ? invoke<ConfigPayload>("get_config") : mockBackend.getConfig();
}

export async function sendCommand(cmd: Command): Promise<void> {
  if (isTauri) return invoke<void>("send_command", { cmd });
  mockBackend.sendCommand(cmd);
}

export async function setMode(mode: Mode): Promise<void> {
  if (isTauri) return invoke<void>("set_mode", { mode });
  mockBackend.setMode(mode);
}

export async function setQueueVisible(visible: boolean): Promise<void> {
  if (isTauri) return invoke<void>("set_queue_visible", { visible });
  mockBackend.setQueueVisible(visible);
}

export async function openSpotify(): Promise<void> {
  if (isTauri) return invoke<void>("open_spotify");
  mockBackend.openSpotify();
}

export async function authSetClientId(clientId: string): Promise<void> {
  if (isTauri) return invoke<void>("auth_set_client_id", { clientId });
  mockBackend.setClientId(clientId);
}

export async function authSignIn(): Promise<void> {
  if (isTauri) return invoke<void>("auth_sign_in");
  await mockBackend.signIn();
}

export async function authSignOut(): Promise<void> {
  if (isTauri) return invoke<void>("auth_sign_out");
  mockBackend.signOut();
}

export async function subscribeState(listener: (state: PlayerState) => void): Promise<UnlistenFn> {
  if (!isTauri) return mockBackend.subscribe(listener);
  return listen<PlayerState>(STATE_EVENT, (event) => listener(event.payload));
}

export async function subscribeConfig(
  listener: (config: ConfigPayload) => void,
): Promise<UnlistenFn> {
  if (!isTauri) return mockBackend.subscribeConfig(listener);
  return listen<ConfigPayload>(CONFIG_EVENT, (event) => listener(event.payload));
}

export async function subscribeError(listener: (message: string) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => undefined;
  return listen<string>(ERROR_EVENT, (event) => listener(event.payload));
}

/** Used by browser tooling and tests to exercise the same state event shape as Tauri. */
export async function emitStateForTesting(state: PlayerState): Promise<void> {
  if (isTauri) await emit(STATE_EVENT, state);
}

/** Opacity the UI fades to after 30 s of nothing playing (1 = fully visible). */
export async function subscribeIdleOpacity(listener: (opacity: number) => void): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return listen<number>(IDLE_OPACITY_EVENT, (event) => listener(event.payload));
}
