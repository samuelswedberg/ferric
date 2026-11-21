<script lang="ts">
  import { onMount } from "svelte";
  import type { PlayerState } from "./lib/bindings/PlayerState";
  import { getState, subscribeState } from "./lib/ipc";

  let state: PlayerState | null = null;
  let error: string | null = null;

  onMount(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void getState()
      .then((initialState) => {
        if (active) state = initialState;
      })
      .catch((cause: unknown) => {
        if (active) error = cause instanceof Error ? cause.message : String(cause);
      });

    void subscribeState((nextState) => {
      if (active) state = nextState;
    })
      .then((stop) => {
        if (active) unlisten = stop;
        else stop();
      })
      .catch((cause: unknown) => {
        if (active) error = cause instanceof Error ? cause.message : String(cause);
      });

    return () => {
      active = false;
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>Ferric</title>
</svelte:head>

<main>
  {#if error}
    <p role="alert">{error}</p>
  {:else if state}
    <pre>{JSON.stringify(state, null, 2)}</pre>
  {:else}
    <p>Loading player state…</p>
  {/if}
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    color-scheme: dark;
    background: #121212;
    font-family: "Segoe UI Variable", "Segoe UI", sans-serif;
  }

  :global(body) {
    margin: 0;
  }

  main {
    min-height: 100vh;
    padding: 1rem;
    color: #f5f5f5;
    background: #121212;
  }

  pre {
    margin: 0;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    font: 12px/1.45 "Cascadia Mono", Consolas, monospace;
  }

  [role="alert"] {
    color: #ffb4ab;
  }
</style>
