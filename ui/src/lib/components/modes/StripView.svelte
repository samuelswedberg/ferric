<script lang="ts">
  import { windowDrag } from "../../windowDrag";
  import type { PlayerState } from "../../bindings/PlayerState";
  import { artistLink, openSpotifyLink, trackLink, type SpotifyLink } from "../../spotifyLinks";
  import { formatTime, playerStore, resolveArtSrc } from "../../state.svelte";
  import KeyBank from "../KeyBank.svelte";
  import RecordDeck from "../RecordDeck.svelte";
  import ScrollText from "../ScrollText.svelte";
  import WindowChrome from "../WindowChrome.svelte";
  import { grainTextureUrl, plinthTextureUrl } from "../textures";

  interface Props {
    state: PlayerState;
    positionMs: number;
    isHidden: boolean;
    reducedMotion: boolean;
    volumeDisplay: number | "unknown" | null;
    onModeSwitch: () => void;
  }

  let {
    state: playerState,
    positionMs,
    isHidden,
    reducedMotion,
    volumeDisplay,
    onModeSwitch,
  }: Props = $props();

  const spotifyRunning = $derived(playerState.spotifyRunning);
  const track = $derived(playerState.track);
  const isStopped = $derived(!spotifyRunning || playerState.playback === "stopped");
  const durationMs = $derived(track?.durationMs ?? null);
  const progress = $derived(
    !spotifyRunning || isStopped || durationMs === null || durationMs <= 0
      ? 0
      : Math.min(1, Math.max(0, positionMs / durationMs)),
  );
  const titleText = $derived(
    !spotifyRunning ? "Spotify isn't open" : (track?.title ?? "Nothing playing"),
  );
  const artistText = $derived(!spotifyRunning ? "" : (track?.artists.join(", ") ?? ""));
  const volumeActive = $derived(typeof volumeDisplay === "number");
  const lineRatio = $derived(volumeActive ? (volumeDisplay as number) / 100 : progress);
  const timeText = $derived.by(() => {
    if (volumeDisplay === "unknown") return "VOL —";
    if (typeof volumeDisplay === "number") return `VOL ${volumeDisplay}`;
    if (!spotifyRunning || !track) return "";
    const current = isStopped ? "0:00" : formatTime(positionMs);
    return `${current} / ${durationMs === null ? "--:--" : formatTime(durationMs)}`;
  });

  const sleeveSrc = $derived(resolveArtSrc(playerState.theme?.sleeve));
  const fallbackArtSrc = $derived(resolveArtSrc(track?.art));
  const recordSrc = $derived(resolveArtSrc(playerState.theme?.record));
  const songUrl = $derived(spotifyRunning && track ? trackLink(track) : null);
  const artistUrl = $derived(spotifyRunning && track ? artistLink(track) : null);

  function openExternal(link: SpotifyLink | null): void {
    if (!link) return;
    void openSpotifyLink(link);
  }
</script>

<main
  class="plinth strip-plinth"
  class:no-motion={reducedMotion}
  use:windowDrag={onModeSwitch}
  style="background-image: url({plinthTextureUrl})"
>
  <WindowChrome variant="strip" {onModeSwitch} />
  <RecordDeck
    variant="strip"
    playback={playerState.playback}
    {spotifyRunning}
    {sleeveSrc}
    {fallbackArtSrc}
    {recordSrc}
    {progress}
    {isHidden}
    {reducedMotion}
    onPlayPause={() => void playerStore.runCommand({ type: "playPause" })}
    onOpenSpotify={() => void playerStore.runOpenSpotify()}
  />

  <span class="title clip">
    {#if songUrl}
      <button
        type="button"
        class="text-link"
        aria-label={`Open song ${titleText} on Spotify`}
        onclick={() => openExternal(songUrl)}
      >
        <ScrollText text={titleText} {reducedMotion} />
      </button>
    {:else}
      <ScrollText text={titleText} {reducedMotion} />
    {/if}
  </span>
  {#if artistText}
    <span class="artist clip">
      {#if artistUrl}
        <button
          type="button"
          class="text-link"
          aria-label={`Open artist ${track?.artists[0] ?? artistText} on Spotify`}
          onclick={() => openExternal(artistUrl)}
        >
          <ScrollText text={artistText} {reducedMotion} />
        </button>
      {:else}
        <ScrollText text={artistText} {reducedMotion} />
      {/if}
    </span>
  {/if}
  {#if timeText}<span class="time">{timeText}</span>{/if}

  <div class="control-fade" aria-hidden="true"></div>
  <KeyBank
    variant="strip"
    {spotifyRunning}
    liked={playerState.liked}
    shuffle={playerState.shuffle}
    repeat={playerState.repeat}
    {reducedMotion}
    onPrevious={() => void playerStore.runCommand({ type: "previous" })}
    onNext={() => void playerStore.runCommand({ type: "next" })}
    onSetPlaybackMode={(shuffle, repeat) =>
      void playerStore.runCommand({ type: "setPlaybackMode", shuffle, repeat })
    }
    onToggleLike={() =>
      void playerStore.runCommand({ type: "setLiked", liked: !(playerState.liked ?? false) })}
  />

  <div class="progress-line" class:volume={volumeActive}>
    <span style="width: {lineRatio * 100}%"></span>
  </div>
  <div
    class="grain-overlay"
    style="background-image: url({grainTextureUrl}); opacity: {playerStore.grainOpacity}"
    aria-hidden="true"
  ></div>
</main>

<style>
  .strip-plinth {
    position: relative;
    width: 360px;
    height: 36px;
    border-radius: 8px;
    overflow: hidden;
    background-color: var(--plinth);
    background-size: cover;
    background-position: center;
    box-shadow: 0 6px 14px rgba(0, 0, 0, 0.4);
  }

  .clip {
    position: absolute;
    z-index: 2;
    display: block;
    overflow: hidden;
    white-space: nowrap;
    pointer-events: none;
  }

  .title {
    left: 50px;
    top: 8px;
    width: 94px;
    color: var(--ink);
    font: 600 14px/1.2 "Bahnschrift", "Segoe UI", sans-serif;
  }

  .artist {
    left: 150px;
    top: 11px;
    width: 116px;
    color: var(--ink-muted);
    font: 400 11px/1.2 "Bahnschrift", "Segoe UI", sans-serif;
  }

  .text-link {
    appearance: none;
    display: inline-block;
    max-width: 100%;
    vertical-align: top;
    padding: 0;
    margin: 0;
    border: 0;
    background: transparent;
    font: inherit;
    color: inherit;
    text-align: left;
    pointer-events: auto;
    cursor: pointer;
    transition: color 140ms ease;
  }

  .text-link:hover,
  .text-link:focus-visible {
    color: var(--ink);
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--accent) 80%, currentColor);
    text-underline-offset: 2px;
  }

  .time {
    position: absolute;
    right: 12px;
    top: 11px;
    z-index: 4;
    color: var(--ink-muted);
    font: 700 10px/1.2 "Courier New", monospace;
    font-variant-numeric: tabular-nums;
    transition: opacity 150ms ease;
    pointer-events: none;
  }

  .control-fade {
    position: absolute;
    z-index: 3;
    left: 154px;
    right: 0;
    top: 0;
    bottom: 3px;
    background: linear-gradient(90deg, transparent, var(--plinth) 24%);
    opacity: 0;
    transition: opacity 150ms ease;
    pointer-events: none;
  }

  :global(.strip-plinth .key-tray) {
    opacity: 0;
    transition: opacity 150ms ease;
  }

  .strip-plinth:hover .time,
  .strip-plinth:focus-within .time {
    opacity: 0;
  }

  .strip-plinth:hover .control-fade,
  .strip-plinth:focus-within .control-fade,
  .strip-plinth:hover :global(.key-tray),
  .strip-plinth:focus-within :global(.key-tray) {
    opacity: 1;
  }

  .progress-line {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: #0a0908;
    overflow: hidden;
  }

  .progress-line span {
    display: block;
    height: 100%;
    background: var(--accent);
    /* Steps with the progress (no width glide); colour still eases for the volume readout. */
    transition: background-color 200ms ease;
  }

  .progress-line.volume span {
    background: var(--volume);
  }

  .grain-overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    background-size: cover;
    background-position: center;
    pointer-events: none;
  }

  .no-motion .time,
  .no-motion .control-fade,
  .no-motion :global(.key-tray),
  .no-motion .progress-line span {
    transition: none;
  }
</style>
