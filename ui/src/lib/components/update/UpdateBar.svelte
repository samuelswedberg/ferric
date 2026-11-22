<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface UpdateAvailable {
    version: string;
  }

  const UPDATE_EVENT = "ferric://update-available";
  const isTauri = typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;

  let version = $state<string | null>(null);
  let installing = $state(false);
  let error = $state<string | null>(null);

  async function install(): Promise<void> {
    if (!version || installing || !isTauri) return;
    installing = true;
    error = null;
    try {
      await invoke<void>("install_update");
    } catch (cause: unknown) {
      installing = false;
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  onMount(() => {
    if (!isTauri) return;
    let unlisten: UnlistenFn | undefined;
    void listen<UpdateAvailable>(UPDATE_EVENT, (event) => {
      version = event.payload.version;
      error = null;
    }).then((stop) => {
      unlisten = stop;
    });
    return () => unlisten?.();
  });
</script>

{#if version}
  <div class="update-tape" role="status" aria-live="polite">
    {#if error}
      <span class="message" title={error}>Update failed · Try again</span>
    {:else}
      <span class="message">Update available: v{version}</span>
    {/if}
    <span aria-hidden="true">·</span>
    <button type="button" onclick={install} disabled={installing}>
      {installing ? "Installing…" : error ? "Retry" : "Restart"}
    </button>
  </div>
{/if}

<style>
  .update-tape {
    position: fixed;
    left: 22px;
    right: 22px;
    bottom: 5px;
    z-index: 50;
    min-width: 0;
    padding: 5px 11px 4px;
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 6px;
    overflow: hidden;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.12), transparent 17%, transparent 82%, rgba(79, 65, 42, 0.08)),
      #dfd2aa;
    color: #302a24;
    border-radius: 1px 2px 1px 1px;
    box-shadow: 0 2px 7px rgba(0, 0, 0, 0.58);
    transform: rotate(-0.35deg);
    font-family: "Courier New", monospace;
    font-size: 11px;
    font-weight: 700;
    line-height: 1.25;
  }

  .update-tape::before,
  .update-tape::after {
    content: "";
    position: absolute;
    inset-block: 0;
    width: 7px;
    opacity: 0.22;
    background: repeating-linear-gradient(135deg, transparent 0 2px, #725f3f 2px 3px);
  }

  .update-tape::before { left: 0; }
  .update-tape::after { right: 0; transform: scaleX(-1); }

  .message {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button {
    flex: none;
    padding: 0;
    border: 0;
    border-bottom: 1px solid currentColor;
    background: transparent;
    color: inherit;
    font: inherit;
    line-height: inherit;
    cursor: pointer;
  }

  button:hover:not(:disabled),
  button:focus-visible {
    color: #6b2b1f;
    outline: none;
    border-bottom-width: 2px;
  }

  button:disabled {
    cursor: wait;
    opacity: 0.68;
  }

  @media (prefers-reduced-motion: reduce) {
    .update-tape { transform: none; }
  }
</style>
