<script lang="ts">
  import type { RepeatMode } from "../bindings/RepeatMode";
  import { derivePlaybackMode, nextPlaybackMode, playbackModeLabel } from "../playbackMode";

  interface Props {
    variant?: "full" | "card" | "strip";
    spotifyRunning: boolean;
    liked: boolean | null;
    shuffle: boolean | null;
    repeat: RepeatMode | null;
    reducedMotion: boolean;
    onPrevious: () => void;
    onNext: () => void;
    onSetPlaybackMode: (shuffle: boolean, repeat: RepeatMode) => void;
    onToggleLike: () => void;
  }

  let {
    variant = "full",
    spotifyRunning,
    liked,
    shuffle,
    repeat,
    reducedMotion,
    onPrevious,
    onNext,
    onSetPlaybackMode,
    onToggleLike,
  }: Props = $props();

  const playbackMode = $derived(derivePlaybackMode(shuffle, repeat));
  const modeDisabled = $derived(!spotifyRunning || playbackMode === null);
  const modeActive = $derived(playbackMode !== null && playbackMode !== "off");
  const modeLabel = $derived(playbackModeLabel(playbackMode));
  const likeDisabled = $derived(!spotifyRunning || liked === null);

  function cyclePlaybackMode(): void {
    const next = nextPlaybackMode(playbackMode);
    onSetPlaybackMode(next.shuffle, next.repeat);
  }
</script>

<div
  class="key-tray"
  class:card={variant === "card"}
  class:strip={variant === "strip"}
  role="group"
  aria-label="Playback controls"
>
  <button
    type="button"
    class="key"
    class:no-motion={reducedMotion}
    disabled={!spotifyRunning}
    aria-label="Previous track"
    onclick={onPrevious}
  >
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <rect x="0" y="0" width="1.875" height="12" fill="currentColor" />
      <polygon points="12,0 12,12 3,6" fill="currentColor" />
    </svg>
  </button>

  <button
    type="button"
    class="key"
    class:no-motion={reducedMotion}
    disabled={!spotifyRunning}
    aria-label="Next track"
    onclick={onNext}
  >
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
      <polygon points="0,0 0,12 9,6" fill="currentColor" />
      <rect x="10.125" y="0" width="1.875" height="12" fill="currentColor" />
    </svg>
  </button>

  <button
    type="button"
    class="key"
    class:no-motion={reducedMotion}
    disabled={modeDisabled}
    aria-label={modeLabel}
    aria-pressed={playbackMode === null ? undefined : modeActive}
    title={modeLabel}
    onclick={cyclePlaybackMode}
  >
    {#if playbackMode === "repeatContext"}
      <svg class="accent-icon" width="15" height="14" viewBox="0 0 16 14" fill="none" aria-hidden="true">
        <path
          d="M2 6.2V4.6A2.1 2.1 0 0 1 4.1 2.5H13.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M11.4 0.8L13.8 2.5L11.4 4.2"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M14 7.8V9.4A2.1 2.1 0 0 1 11.9 11.5H2.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M4.6 9.8L2.2 11.5L4.6 13.2"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {:else if playbackMode === "repeatTrack"}
      <svg class="accent-icon" width="15" height="14" viewBox="0 0 16 14" fill="none" aria-hidden="true">
        <path
          d="M2 6.2V4.6A2.1 2.1 0 0 1 4.1 2.5H13.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M11.4 0.8L13.8 2.5L11.4 4.2"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M14 7.8V9.4A2.1 2.1 0 0 1 11.9 11.5H2.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M4.6 9.8L2.2 11.5L4.6 13.2"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M7.1 5.6L8.2 4.7V9.3"
          stroke="currentColor"
          stroke-width="1.45"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {:else}
      <svg
        class:accent-icon={playbackMode === "shuffle"}
        class:mode-off={playbackMode === "off"}
        width="15"
        height="14"
        viewBox="0 0 16 14"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M1.5 3.5H4.2C6 3.5 7 5.2 8 7C9 8.8 10 10.5 11.8 10.5H14"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M1.5 10.5H4.2C6 10.5 7 8.8 8 7C9 5.2 10 3.5 11.8 3.5H14"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M11.8 1.5L14.2 3.5L11.8 5.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
        <path
          d="M11.8 8.5L14.2 10.5L11.8 12.5"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </button>

  <button
    type="button"
    class="key"
    class:no-motion={reducedMotion}
    disabled={likeDisabled}
    aria-label={liked ? "Unlike track" : "Like track"}
    aria-pressed={liked ?? undefined}
    onclick={onToggleLike}
  >
    {#if liked === true}
      <svg class="heart-filled" width="15" height="14" viewBox="0 0 15 14" aria-hidden="true">
        <path
          d="M7.5 14L6.45 13.01C2.55 9.43 0 7.08 0 4.18C0 1.83 1.8 0 4.125 0C5.4 0 6.675 0.61 7.5 1.6C8.325 0.61 9.6 0 10.875 0C13.2 0 15 1.83 15 4.18C15 7.08 12.45 9.43 8.55 13.01L7.5 14Z"
          fill="currentColor"
        />
      </svg>
    {:else}
      <svg width="15" height="14" viewBox="0 0 15 14" aria-hidden="true">
        <path
          d="M7.5 13.2L6.6 12.35C2.95 9.02 0.75 6.95 0.75 4.28C0.75 2.3 2.25 0.8 4.2 0.8C5.35 0.8 6.48 1.38 7.5 2.35C8.52 1.38 9.65 0.8 10.8 0.8C12.75 0.8 14.25 2.3 14.25 4.28C14.25 6.95 12.05 9.02 8.4 12.35L7.5 13.2Z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.35"
          stroke-linejoin="round"
        />
      </svg>
    {/if}
  </button>
</div>

<style>
  .key-tray {
    position: absolute;
    left: 20px;
    top: 354px;
    width: 266px;
    height: 42px;
    padding: 4px;
    border-radius: 6px;
    background: var(--tray);
    box-shadow:
      inset 0 2px 3px rgba(0, 0, 0, 0.95),
      0 1px 0 rgba(255, 255, 255, 0.08);
    display: flex;
    align-items: flex-start;
    gap: 2px;
  }

  .key {
    position: relative;
    width: 63px;
    height: 32px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    color: var(--printed-icon);
    background: linear-gradient(180deg, var(--key-top) 0%, var(--key-bottom) 100%);
    box-shadow:
      0 2px 0 rgba(0, 0, 0, 0.8),
      inset 0 1px 0 rgba(255, 255, 255, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: transform 60ms ease-out;
  }

  .key svg {
    filter: drop-shadow(0 -1px 0 rgba(0, 0, 0, 0.7));
  }

  .key:active:not(:disabled) {
    height: 30px;
    transform: translateY(2px);
    background: linear-gradient(180deg, var(--key-down-top) 0%, var(--key-down-bottom) 100%);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.6);
  }

  .key.no-motion {
    transition: none;
  }

  .key:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .accent-icon {
    color: var(--accent);
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--accent) 55%, transparent));
    transition: color 0.3s ease;
  }

  .mode-off {
    opacity: 0.65;
  }

  .heart-filled {
    color: var(--accent);
    transition: color 0.3s ease;
  }

  .key-tray.card {
    left: 162px;
    top: 74px;
    width: 188px;
    height: 30px;
    padding: 3px;
    border-radius: 5px;
  }

  .key-tray.card .key {
    width: 44px;
    height: 23px;
  }

  .key-tray.card .key:active:not(:disabled) {
    height: 21px;
  }

  .key-tray.strip {
    z-index: 5;
    /* Leaves 40 px on the right for the window buttons. */
    left: 186px;
    top: 6px;
    width: 128px;
    height: 24px;
    padding: 3px;
    border-radius: 5px;
  }

  .key-tray.strip .key {
    width: 29px;
    height: 18.5px;
  }

  .key-tray.strip .key:active:not(:disabled) {
    height: 16.5px;
  }

  .key-tray.strip .key svg {
    transform: scale(0.76);
  }
</style>
