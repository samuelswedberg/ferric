<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  interface Props {
    onModeSwitch?: () => void;
  }

  let { onModeSwitch }: Props = $props();

  async function hideWindow(): Promise<void> {
    const isTauri = typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;
    if (isTauri) {
      await getCurrentWindow().hide();
    }
  }
</script>

<div class="chrome">
  <button type="button" class="chrome-btn" aria-label="Hide window" onclick={hideWindow}>
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <line x1="2" y1="2" x2="8" y2="8" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
      <line x1="8" y1="2" x2="2" y2="8" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
    </svg>
  </button>

  <button
    type="button"
    class="chrome-btn"
    aria-label="Switch mode"
    onclick={() => onModeSwitch?.()}
  >
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <rect x="1.5" y="2" width="7" height="2.4" rx="0.6" fill="currentColor" />
      <rect x="1.5" y="5.6" width="7" height="2.4" rx="0.6" fill="currentColor" opacity="0.6" />
    </svg>
  </button>
</div>

<style>
  .chrome {
    position: absolute;
    left: 10px;
    top: 8px;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 150ms ease;
  }

  :global(.plinth:hover) .chrome,
  .chrome:focus-within {
    opacity: 1;
  }

  .chrome-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--printed);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .chrome-btn:hover {
    color: var(--ink);
    background: rgba(255, 255, 255, 0.08);
  }
</style>
