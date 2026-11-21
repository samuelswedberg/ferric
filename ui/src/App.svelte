<script lang="ts">
  import { onMount } from "svelte";
  import "./lib/styles/tokens.css";
  import type { Rgb } from "./lib/bindings/Rgb";
  import FullView from "./lib/components/FullView.svelte";
  import { playerStore } from "./lib/state.svelte";

  function rgbCss(rgb: Rgb): string {
    return `rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`;
  }

  const accentCss = $derived(
    playerStore.playerState?.theme ? rgbCss(playerStore.playerState.theme.accent) : "#B3B3B3",
  );
  const onAccentCss = $derived(
    playerStore.playerState?.theme ? rgbCss(playerStore.playerState.theme.onAccent) : "#1E1B18",
  );

  function onGlobalKeyDown(event: KeyboardEvent): void {
    const target = event.target;
    if (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement
    ) {
      return;
    }

    const state = playerStore.playerState;
    if (!state) return;

    if (event.code === "Space") {
      // Avoid double-triggering when a button already has focus
      if (target instanceof HTMLButtonElement) return;
      event.preventDefault();
      if (!state.spotifyRunning) {
        void playerStore.runOpenSpotify();
      } else {
        void playerStore.runCommand({ type: "playPause" });
      }
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
      const next = Math.min(100, Math.round((state.volumePct + 5) / 5) * 5);
      void playerStore.runCommand({ type: "setVolume", pct: next });
    } else if (event.key === "ArrowDown" && state.volumePct !== null) {
      event.preventDefault();
      const next = Math.max(0, Math.round((state.volumePct - 5) / 5) * 5);
      void playerStore.runCommand({ type: "setVolume", pct: next });
    }
  }

  onMount(() => {
    return playerStore.init();
  });
</script>

<svelte:head>
  <title>Ferric</title>
</svelte:head>

<svelte:window onkeydown={onGlobalKeyDown} />

<div
  class="app-shell"
  style="
    --accent: {accentCss};
    --on-accent: {onAccentCss};
    transition: {playerStore.reducedMotion ? 'none' : 'color 300ms ease'};
  "
>
  {#if playerStore.playerState}
    <FullView
      state={playerStore.playerState}
      positionMs={playerStore.displayPositionMs}
      toastMessage={playerStore.toastMessage}
      isHidden={playerStore.isHidden}
      reducedMotion={playerStore.reducedMotion}
    />
  {/if}
</div>

<style>
  .app-shell {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
  }
</style>
