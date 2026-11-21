<script lang="ts">
  import type { Playback } from "../bindings/Playback";

  interface Props {
    playback: Playback;
    spotifyRunning: boolean;
    liked: boolean | null;
    reducedMotion: boolean;
    onPrevious: () => void;
    onPlayPause: () => void;
    onNext: () => void;
    onToggleLike: () => void;
    onOpenSpotify: () => void;
  }

  let {
    playback,
    spotifyRunning,
    liked,
    reducedMotion,
    onPrevious,
    onPlayPause,
    onNext,
    onToggleLike,
    onOpenSpotify,
  }: Props = $props();

  const isPlaying = $derived(spotifyRunning && playback === "playing");
  const likeDisabled = $derived(!spotifyRunning || liked === null);
</script>

<div class="key-tray" role="group" aria-label="Playback controls">
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
    class:held={isPlaying}
    class:no-motion={reducedMotion}
    aria-label={!spotifyRunning ? "Open Spotify" : isPlaying ? "Pause" : "Play"}
    aria-pressed={spotifyRunning ? isPlaying : undefined}
    onclick={() => (spotifyRunning ? onPlayPause() : onOpenSpotify())}
  >
    {#if isPlaying}
      <svg class="accent-icon" width="12" height="14" viewBox="0 0 12 14" aria-hidden="true">
        <rect x="0" y="0" width="4" height="14" fill="currentColor" />
        <rect x="8" y="0" width="4" height="14" fill="currentColor" />
      </svg>
    {:else}
      <svg width="11" height="13" viewBox="0 0 11 13" aria-hidden="true">
        <polygon points="0,0 11,6.5 0,13" fill="currentColor" />
      </svg>
    {/if}
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

  .key:active:not(:disabled),
  .key.held {
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

  .heart-filled {
    color: var(--accent);
    transition: color 0.3s ease;
  }
</style>
