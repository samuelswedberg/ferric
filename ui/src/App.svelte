<script lang="ts">
  import { onMount } from "svelte";
  import type { TransitionConfig } from "svelte/transition";
  import "./lib/styles/tokens.css";
  import type { Rgb } from "./lib/bindings/Rgb";
  import ErrorToast from "./lib/components/ErrorToast.svelte";
  import FullView from "./lib/components/FullView.svelte";
  import CardView from "./lib/components/modes/CardView.svelte";
  import StripView from "./lib/components/modes/StripView.svelte";
  import OnboardingPanel from "./lib/components/onboarding/OnboardingPanel.svelte";
  import UpdateBar from "./lib/components/update/UpdateBar.svelte";
  import {
    nextMode,
    queueShouldBeVisible,
    stepVolume,
    UNKNOWN_VOLUME_RESET_MS,
    VOLUME_RESET_MS,
  } from "./lib/components/modes/behavior";
  import { setMode, setQueueVisible } from "./lib/ipc";
  import { playerStore } from "./lib/state.svelte";

  type VolumeDisplay = number | "unknown" | null;

  let volumeDisplay = $state<VolumeDisplay>(null);
  let volumeTimer: ReturnType<typeof setTimeout> | null = null;
  let lastQueueVisible: boolean | null = null;

  // Hovering wakes a faded window; it stays awake for another idle period after the
  // pointer leaves, so the fade timer effectively restarts on every visit.
  const IDLE_REARM_MS = 30_000;
  let pointerInside = $state(false);
  let awake = $state(false);
  let awakeTimer: ReturnType<typeof setTimeout> | null = null;
  const faded = $derived(playerStore.idleOpacity < 1 && !pointerInside && !awake);

  function onShellEnter(): void {
    pointerInside = true;
    if (awakeTimer !== null) clearTimeout(awakeTimer);
    awakeTimer = null;
  }

  function onShellLeave(): void {
    pointerInside = false;
    awake = true;
    if (awakeTimer !== null) clearTimeout(awakeTimer);
    awakeTimer = setTimeout(() => {
      awake = false;
      awakeTimer = null;
    }, IDLE_REARM_MS);
  }

  function rgbCss(rgb: Rgb): string {
    return `rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`;
  }

  const accentCss = $derived(
    playerStore.playerState?.theme ? rgbCss(playerStore.playerState.theme.accent) : "#B3B3B3",
  );
  const onAccentCss = $derived(
    playerStore.playerState?.theme ? rgbCss(playerStore.playerState.theme.onAccent) : "#1E1B18",
  );

  function reportError(cause: unknown): void {
    playerStore.showToast(cause instanceof Error ? cause.message : String(cause));
  }

  async function cycleMode(): Promise<void> {
    const state = playerStore.playerState;
    if (!state) return;
    volumeDisplay = null;
    try {
      await setMode(nextMode(state.mode));
    } catch (cause: unknown) {
      reportError(cause);
    }
  }

  function resetVolumeReadout(afterMs: number): void {
    if (volumeTimer !== null) clearTimeout(volumeTimer);
    volumeTimer = setTimeout(() => {
      volumeDisplay = null;
      volumeTimer = null;
    }, afterMs);
  }

  function onWheel(event: WheelEvent): void {
    const state = playerStore.playerState;
    if (!state || state.mode === "full" || event.deltaY === 0) return;
    event.preventDefault();

    if (state.volumePct === null) {
      volumeDisplay = "unknown";
      resetVolumeReadout(UNKNOWN_VOLUME_RESET_MS);
      return;
    }

    const current = typeof volumeDisplay === "number" ? volumeDisplay : state.volumePct;
    const next = stepVolume(current, event.deltaY < 0 ? 1 : -1);
    volumeDisplay = next;
    resetVolumeReadout(VOLUME_RESET_MS);
    if (next !== current) void playerStore.runCommand({ type: "setVolume", pct: next });
  }

  function onGlobalKeyDown(event: KeyboardEvent): void {
    const target = event.target;
    if (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement
    ) return;

    const state = playerStore.playerState;
    if (!state) return;

    if (event.code === "Space") {
      if (target instanceof HTMLButtonElement) return;
      event.preventDefault();
      if (!state.spotifyRunning) void playerStore.runOpenSpotify();
      else void playerStore.runCommand({ type: "playPause" });
      return;
    }

    if (!state.spotifyRunning) return;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      void playerStore.runCommand({ type: "previous" });
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      void playerStore.runCommand({ type: "next" });
    } else if (event.key === "ArrowUp" && state.volumePct !== null) {
      event.preventDefault();
      void playerStore.runCommand({ type: "setVolume", pct: stepVolume(state.volumePct, 1) });
    } else if (event.key === "ArrowDown" && state.volumePct !== null) {
      event.preventDefault();
      void playerStore.runCommand({ type: "setVolume", pct: stepVolume(state.volumePct, -1) });
    }
  }

  function modeTransition(_node: Element, params: { duration: number }): TransitionConfig {
    return {
      duration: params.duration,
      css: (t) => `opacity: ${t}; transform: scale(${0.985 + t * 0.015})`,
    };
  }

  $effect(() => {
    const state = playerStore.playerState;
    if (!state) return;
    const visible = queueShouldBeVisible(state.mode, playerStore.isHidden);
    if (visible === lastQueueVisible) return;
    lastQueueVisible = visible;
    void setQueueVisible(visible).catch(reportError);
  });

  onMount(() => {
    const destroyStore = playerStore.init();
    return () => {
      destroyStore();
      if (volumeTimer !== null) clearTimeout(volumeTimer);
      if (lastQueueVisible === true) void setQueueVisible(false).catch(reportError);
    };
  });
</script>

<svelte:head><title>Ferric</title></svelte:head>
<svelte:window onkeydown={onGlobalKeyDown} onwheel={onWheel} />
<svelte:body onpointerenter={onShellEnter} onpointerleave={onShellLeave} />

<div
  class="app-shell"
  class:idle={faded}
  style="--accent: {accentCss}; --on-accent: {onAccentCss}; --idle-opacity: {playerStore.idleOpacity}; transition: {playerStore.reducedMotion
    ? 'none'
    : 'color 300ms ease, opacity 600ms ease'};"
>
  {#if playerStore.playerState}
    {#key playerStore.playerState.mode}
      <div
        class="mode-frame"
        transition:modeTransition={{ duration: playerStore.reducedMotion ? 0 : 200 }}
      >
        {#if playerStore.playerState.mode === "full"}
          <FullView
            state={playerStore.playerState}
            positionMs={playerStore.displayPositionMs}
            isHidden={playerStore.isHidden}
            reducedMotion={playerStore.reducedMotion}
            onModeSwitch={cycleMode}
          />
        {:else if playerStore.playerState.mode === "card"}
          <CardView
            state={playerStore.playerState}
            positionMs={playerStore.displayPositionMs}
            isHidden={playerStore.isHidden}
            reducedMotion={playerStore.reducedMotion}
            {volumeDisplay}
            onModeSwitch={cycleMode}
          />
        {:else}
          <StripView
            state={playerStore.playerState}
            positionMs={playerStore.displayPositionMs}
            isHidden={playerStore.isHidden}
            reducedMotion={playerStore.reducedMotion}
            {volumeDisplay}
            onModeSwitch={cycleMode}
          />
        {/if}
      </div>
    {/key}
    <ErrorToast message={playerStore.toastMessage} />
    <OnboardingPanel playerState={playerStore.playerState} />
  {/if}
  <UpdateBar />
</div>

<style>
  .app-shell.idle {
    opacity: var(--idle-opacity);
  }

  .app-shell {
    position: relative;
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
  }

  .mode-frame {
    position: relative;
    transform-origin: center;
  }
</style>
