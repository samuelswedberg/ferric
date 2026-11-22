<script lang="ts">
  import { isTauri } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { PlayerState } from "../../bindings/PlayerState";
  import { authSetClientId, authSignIn, setMode } from "../../ipc";
  import { cardTextureUrl } from "../textures";
  import { onboarding } from "./controller.svelte";
  import { firstStepFor, hasPortConflict, mockOnboardingStep } from "./logic";

  interface Props {
    playerState: PlayerState;
  }

  const DASHBOARD_URL = "https://developer.spotify.com/dashboard";
  const REDIRECT_URI = "http://127.0.0.1:8898/callback";
  const PREMIUM_NOTE_KEY = "ferric-premium-note-seen";

  let { playerState }: Props = $props();
  let clientId = $state("");
  let localError = $state<string | null>(null);
  let saving = $state(false);
  let copied = $state(false);
  let modeSwitchPending = $state(false);
  let initialized = $state(false);
  let premiumNoteVisible = $state(false);

  const mock =
    typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("mock");
  const forcedStep = mockOnboardingStep(mock);
  const authError = $derived(
    playerState.auth.status === "error" ? playerState.auth.message : null,
  );
  const message = $derived(authError ?? localError);
  const portConflict = $derived(message !== null && hasPortConflict(message));
  const signingIn = $derived(playerState.auth.status === "signingIn");

  $effect(() => {
    if (initialized) return;
    initialized = true;
    if (forcedStep !== null) onboarding.show(forcedStep);
    else if (playerState.auth.status === "noClientId") {
      onboarding.show(firstStepFor(playerState.auth));
    }
  });

  $effect(() => {
    if (!onboarding.open) {
      modeSwitchPending = false;
      return;
    }
    if (playerState.mode === "full" || modeSwitchPending) return;
    modeSwitchPending = true;
    void (async () => {
      try {
        await setMode("full");
      } catch (cause: unknown) {
        localError = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  $effect(() => {
    if (playerState.auth.status !== "signedIn") return;
    onboarding.close();
    if (playerState.auth.premium) return;
    const alreadySeen = localStorage.getItem(PREMIUM_NOTE_KEY) === "true";
    premiumNoteVisible = mock === "free" || !alreadySeen;
  });

  function goTo(step: 1 | 2 | 3 | 4): void {
    localError = null;
    onboarding.step = step;
  }

  async function openDashboard(): Promise<void> {
    localError = null;
    try {
      if (isTauri()) await openUrl(DASHBOARD_URL);
      else window.open(DASHBOARD_URL, "_blank", "noopener,noreferrer");
    } catch (cause: unknown) {
      localError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function copyRedirect(): Promise<void> {
    try {
      await navigator.clipboard.writeText(REDIRECT_URI);
      copied = true;
      window.setTimeout(() => (copied = false), 1_500);
    } catch (cause: unknown) {
      localError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function saveClientId(): Promise<void> {
    saving = true;
    localError = null;
    try {
      await authSetClientId(clientId.trim());
      goTo(4);
    } catch (cause: unknown) {
      localError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = false;
    }
  }

  async function signIn(): Promise<void> {
    localError = null;
    try {
      await authSignIn();
    } catch (cause: unknown) {
      localError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function dismissPremiumNote(): void {
    localStorage.setItem(PREMIUM_NOTE_KEY, "true");
    premiumNoteVisible = false;
  }
</script>

{#if onboarding.open}
  <div class="onboarding-shade">
    <div
      class="onboarding-panel"
      style="background-image: url({cardTextureUrl})"
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
    >
      <header>
        <div class="step-track" aria-label={`Step ${onboarding.step} of 4`}>
          {#each [1, 2, 3, 4] as step}
            <span class:current={step === onboarding.step} class:done={step < onboarding.step}></span>
          {/each}
        </div>
        <button class="close-key" type="button" onclick={() => onboarding.close()} aria-label="Close">
          ×
        </button>
      </header>

      {#if onboarding.step === 1}
        <div class="step-copy">
          <p class="eyebrow">01 / WHY CONNECT?</p>
          <h2 id="onboarding-title">Your queue, on the sleeve.</h2>
          <p>
            Ferric works without signing in. To see your queue and like songs, connect your own free
            Spotify developer app. It takes about 2 minutes, and your data never leaves your PC.
          </p>
        </div>
        <footer>
          <button class="text-key" type="button" onclick={() => onboarding.close()}>Not now</button>
          <button class="matte-key primary" type="button" onclick={() => goTo(2)}>Get started →</button>
        </footer>
      {:else if onboarding.step === 2}
        <div class="step-copy compact">
          <p class="eyebrow">02 / CREATE THE APP</p>
          <h2 id="onboarding-title">Set up Spotify.</h2>
          <ol>
            <li>Open the dashboard and choose <strong>Create app</strong>.</li>
            <li>Add any name and description. Under APIs, tick <strong>Web API</strong>.</li>
            <li>Add this exact Redirect URI, accept the terms, then choose <strong>Save</strong>.</li>
          </ol>
          <div class="redirect-slot">
            <code>{REDIRECT_URI}</code>
            <button type="button" onclick={copyRedirect}>{copied ? "Copied" : "Copy"}</button>
          </div>
          <button class="dashboard-key" type="button" onclick={openDashboard}>
            Open Spotify Developer Dashboard
          </button>
        </div>
        <footer>
          <button class="text-key" type="button" onclick={() => goTo(1)}>← Back</button>
          <button class="matte-key primary" type="button" onclick={() => goTo(3)}>App created →</button>
        </footer>
      {:else if onboarding.step === 3}
        <div class="step-copy">
          <p class="eyebrow">03 / CLIENT ID</p>
          <h2 id="onboarding-title">Paste the Client ID.</h2>
          <p>Open your app's settings and copy its 32-character Client ID. Ferric stores it securely on this PC.</p>
          <label for="client-id">Spotify Client ID</label>
          <input
            id="client-id"
            class="client-slot"
            bind:value={clientId}
            autocomplete="off"
            autocapitalize="off"
            spellcheck="false"
            placeholder="32 hexadecimal characters"
            oninput={() => (localError = null)}
          />
          {#if message}
            <p class="error" role="alert">{message}</p>
          {/if}
        </div>
        <footer>
          <button class="text-key" type="button" onclick={() => goTo(2)}>← Back</button>
          <button class="matte-key primary" type="button" disabled={saving} onclick={saveClientId}>
            {saving ? "Saving…" : "Save →"}
          </button>
        </footer>
      {:else}
        <div class="step-copy">
          <p class="eyebrow">04 / SIGN IN</p>
          <h2 id="onboarding-title">Connect Ferric.</h2>
          <p>
            Spotify will open in your browser. Approve Ferric, then return here—the panel closes when
            your queue is ready.
          </p>
          {#if message}
            <div class="error-block" role="alert">
              <p class="error">{message}</p>
              {#if portConflict}
                <p>Another app is using port 8898. Close it, or restart that app, then try again.</p>
              {/if}
            </div>
          {/if}
          {#if signingIn}
            <div class="waiting" role="status"><span class="spinner"></span>Waiting for Spotify…</div>
          {/if}
        </div>
        <footer>
          <button class="text-key" type="button" onclick={() => goTo(3)}>Edit Client ID</button>
          <button class="matte-key primary" type="button" disabled={signingIn} onclick={signIn}>
            {signingIn ? "Signing in…" : "Sign in with Spotify →"}
          </button>
        </footer>
      {/if}
    </div>
  </div>
{/if}

{#if premiumNoteVisible}
  <aside class="premium-note" role="status" style="background-image: url({cardTextureUrl})">
    <strong>ONE NOTE FOR FREE ACCOUNTS</strong>
    <p>Seek, volume and queue jump need Spotify Premium. Play/pause/skip still work.</p>
    <button type="button" onclick={dismissPremiumNote}>Got it</button>
  </aside>
{/if}

<style>
  .onboarding-shade {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 14px 16px;
    border-radius: 20px;
    overflow: hidden;
    background: rgba(13, 11, 10, 0.72);
  }

  .onboarding-panel {
    width: 328px;
    min-height: 452px;
    padding: 17px 20px 18px;
    display: flex;
    flex-direction: column;
    color: var(--pencil);
    background-color: #d6c8ac;
    background-size: cover;
    background-position: center;
    border-radius: 2px;
    box-shadow: 0 10px 28px rgba(0, 0, 0, 0.75), inset 0 0 0 1px rgba(47, 42, 38, 0.15);
    transform: rotate(-0.35deg);
    animation: panel-in 360ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
    font-family: "Courier New", monospace;
  }

  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .step-track {
    display: flex;
    gap: 7px;
  }

  .step-track span {
    width: 24px;
    height: 3px;
    background: rgba(47, 42, 38, 0.2);
    transform: rotate(-1deg);
  }

  .step-track span.done,
  .step-track span.current {
    background: var(--pencil);
  }

  .step-track span.current {
    height: 5px;
    margin-top: -1px;
  }

  button {
    font-family: "Courier New", monospace;
  }

  .close-key,
  .text-key {
    border: 0;
    color: var(--pencil);
    background: transparent;
    cursor: pointer;
  }

  .close-key {
    width: 24px;
    height: 24px;
    padding: 0;
    font-size: 20px;
    line-height: 20px;
  }

  .step-copy {
    flex: 1;
    padding-top: 45px;
  }

  .step-copy.compact {
    padding-top: 20px;
  }

  .eyebrow {
    margin: 0 0 10px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
  }

  h2 {
    margin: 0 0 18px;
    max-width: 260px;
    font-size: 24px;
    line-height: 1.05;
    letter-spacing: -0.05em;
  }

  p,
  li {
    font-size: 12px;
    line-height: 1.48;
  }

  ol {
    margin: 0 0 13px;
    padding-left: 24px;
  }

  li {
    margin-bottom: 7px;
    padding-left: 3px;
  }

  strong {
    font-weight: 700;
  }

  .redirect-slot,
  .client-slot {
    width: 100%;
    border: 0;
    border-radius: 4px;
    background: var(--tray);
    box-shadow: inset 0 2px 5px rgba(0, 0, 0, 0.8), 0 1px rgba(255, 255, 255, 0.18);
  }

  .redirect-slot {
    height: 38px;
    padding: 0 5px 0 10px;
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--printed-icon);
  }

  .redirect-slot code {
    flex: 1;
    font: 700 9px/1 "Courier New", monospace;
    user-select: text;
  }

  .redirect-slot button,
  .dashboard-key {
    border: 0;
    border-radius: 3px;
    color: var(--printed-icon);
    background: linear-gradient(180deg, var(--key-top), var(--key-bottom));
    box-shadow: 0 2px 0 rgba(0, 0, 0, 0.45), inset 0 1px rgba(255, 255, 255, 0.12);
    font-weight: 700;
    cursor: pointer;
  }

  .redirect-slot button {
    min-width: 48px;
    height: 28px;
    font-size: 9px;
  }

  .dashboard-key {
    width: 100%;
    height: 34px;
    margin-top: 10px;
    font-size: 10px;
  }

  label {
    display: block;
    margin: 28px 0 6px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .client-slot {
    height: 42px;
    padding: 0 12px;
    color: var(--printed-icon);
    font: 700 11px/1 "Courier New", monospace;
    outline: none;
    user-select: text;
  }

  .client-slot:focus {
    box-shadow: inset 0 2px 5px rgba(0, 0, 0, 0.8), 0 0 0 1px var(--accent);
  }

  .client-slot::placeholder {
    color: rgba(217, 207, 192, 0.42);
  }

  footer {
    min-height: 42px;
    gap: 10px;
  }

  .matte-key {
    min-height: 36px;
    padding: 0 14px;
    border: 0;
    border-radius: 4px;
    color: var(--printed-icon);
    background: linear-gradient(180deg, var(--key-top), var(--key-bottom));
    box-shadow: 0 3px 0 #151210, inset 0 1px rgba(255, 255, 255, 0.12);
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .matte-key.primary {
    color: var(--on-accent);
    background: var(--accent);
  }

  .matte-key:active,
  .dashboard-key:active,
  .redirect-slot button:active {
    transform: translateY(2px);
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .matte-key:disabled {
    opacity: 0.58;
    cursor: wait;
  }

  .text-key {
    padding: 8px 0;
    font-size: 10px;
    font-weight: 700;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .error,
  .error-block p {
    margin: 9px 0 0;
    color: #702e25;
    font-weight: 700;
  }

  .error-block {
    margin-top: 20px;
    padding: 7px 10px 9px;
    background: rgba(229, 216, 178, 0.72);
    box-shadow: 1px 2px 5px rgba(47, 42, 38, 0.25);
    transform: rotate(0.35deg);
  }

  .error-block p + p {
    color: var(--pencil);
    font-weight: 400;
  }

  .waiting {
    margin-top: 26px;
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 11px;
    font-weight: 700;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(47, 42, 38, 0.25);
    border-top-color: var(--pencil);
    border-radius: 50%;
    animation: spin 700ms linear infinite;
  }

  .premium-note {
    position: fixed;
    left: 22px;
    right: 22px;
    bottom: 18px;
    z-index: 95;
    padding: 13px 14px 12px;
    color: var(--pencil);
    background-color: #d6c8ac;
    background-size: cover;
    box-shadow: 0 5px 14px rgba(0, 0, 0, 0.65);
    transform: rotate(0.5deg);
    font-family: "Courier New", monospace;
  }

  .premium-note strong {
    font-size: 9px;
    letter-spacing: 0.09em;
  }

  .premium-note p {
    margin: 7px 0 10px;
  }

  .premium-note button {
    float: right;
    border: 0;
    background: transparent;
    color: var(--pencil);
    font-size: 10px;
    font-weight: 700;
    text-decoration: underline;
    cursor: pointer;
  }

  @keyframes panel-in {
    from {
      opacity: 0;
      transform: translateY(42px) rotate(-0.35deg);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .onboarding-panel,
    .spinner {
      animation: none;
    }
  }
</style>
