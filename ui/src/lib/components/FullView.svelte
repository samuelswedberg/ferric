<script lang="ts">
  import { windowDrag } from "../windowDrag";
  import type { PlayerState } from "../bindings/PlayerState";
  import { artistLink, openSpotifyLink, trackLink, type SpotifyLink } from "../spotifyLinks";
  import { formatTime, playerStore, resolveArtSrc } from "../state.svelte";
  import KeyBank from "./KeyBank.svelte";
  import ProgressSlider from "./ProgressSlider.svelte";
  import QueueCard from "./QueueCard.svelte";
  import RecordDeck from "./RecordDeck.svelte";
  import ScrollText from "./ScrollText.svelte";
  import { grainTextureUrl, plinthTextureUrl } from "./textures";
  import VolumeKnob from "./VolumeKnob.svelte";
  import WindowChrome from "./WindowChrome.svelte";

  interface Props {
    state: PlayerState;
    positionMs: number;
    isHidden: boolean;
    reducedMotion: boolean;
    onModeSwitch: () => void;
  }

  let { state, positionMs, isHidden, reducedMotion, onModeSwitch }: Props = $props();

  const spotifyRunning = $derived(state.spotifyRunning);
  const track = $derived(state.track);
  const isStopped = $derived(!spotifyRunning || state.playback === "stopped");
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
  const timeText = $derived.by(() => {
    if (!spotifyRunning || !track) return "";
    const cur = isStopped ? "0:00" : formatTime(positionMs);
    const total = durationMs !== null ? formatTime(durationMs) : "--:--";
    return `${cur} / ${total}`;
  });

  const sleeveSrc = $derived(resolveArtSrc(state.theme?.sleeve));
  const fallbackArtSrc = $derived(resolveArtSrc(track?.art));
  const recordSrc = $derived(resolveArtSrc(state.theme?.record));

  const songUrl = $derived(spotifyRunning && track ? trackLink(track) : null);
  const artistUrl = $derived(spotifyRunning && track ? artistLink(track) : null);

  function openExternal(link: SpotifyLink | null): void {
    if (!link) return;
    void openSpotifyLink(link);
  }
</script>

<main
  class="plinth"
  use:windowDrag={onModeSwitch}
  style="background-image: url({plinthTextureUrl})"
>
  <WindowChrome {onModeSwitch} />

  <RecordDeck
    playback={state.playback}
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

  <h1 class="title">
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
  </h1>

  {#if artistText}
    <p class="artist">
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
    </p>
  {/if}

  {#if timeText}
    <span class="time">{timeText}</span>
  {/if}

  <ProgressSlider
    {positionMs}
    {durationMs}
    {spotifyRunning}
    {isStopped}
    onSeek={(ms) => void playerStore.runCommand({ type: "seek", positionMs: ms })}
  />

  <KeyBank
    {spotifyRunning}
    liked={state.liked}
    shuffle={state.shuffle}
    repeat={state.repeat}
    {reducedMotion}
    onPrevious={() => void playerStore.runCommand({ type: "previous" })}
    onNext={() => void playerStore.runCommand({ type: "next" })}
    onSetPlaybackMode={(shuffle, repeat) =>
      void playerStore.runCommand({ type: "setPlaybackMode", shuffle, repeat })
    }
    onToggleLike={() =>
      void playerStore.runCommand({ type: "setLiked", liked: !(state.liked ?? false) })
    }
  />

  <VolumeKnob
    volumePct={state.volumePct}
    {spotifyRunning}
    onSetVolume={(pct) => void playerStore.runCommand({ type: "setVolume", pct })}
  />

  <QueueCard queue={state.queue} />

  <div
    class="grain-overlay"
    style="background-image: url({grainTextureUrl}); opacity: {playerStore.grainOpacity}"
    aria-hidden="true"
  ></div>
</main>

<style>
  .plinth {
    position: relative;
    width: 360px;
    height: 520px;
    border-radius: 20px;
    overflow: hidden;
    background-color: var(--plinth);
    background-size: cover;
    background-position: center;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.4);
  }

  .title {
    position: absolute;
    left: 20px;
    top: 268px;
    margin: 0;
    width: 320px;
    font-family: "Bahnschrift", "Segoe UI Variable", "Segoe UI", sans-serif;
    font-weight: 600;
    font-size: 24px;
    line-height: 1.2;
    color: var(--ink);
    overflow: hidden;
    pointer-events: none;
  }

  .artist {
    position: absolute;
    left: 20px;
    top: 300px;
    margin: 0;
    width: 210px;
    font-family: "Bahnschrift", "Segoe UI Variable", "Segoe UI", sans-serif;
    font-weight: 400;
    font-size: 14px;
    line-height: 1.2;
    color: var(--ink-muted);
    overflow: hidden;
    pointer-events: none;
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
    right: 20px;
    top: 305px;
    font-family: "Courier New", monospace;
    font-weight: 700;
    font-size: 12px;
    line-height: 1.2;
    color: var(--ink-muted);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }

  .grain-overlay {
    position: absolute;
    inset: 0;
    z-index: 40;
    background-size: cover;
    background-position: center;
    pointer-events: none;
  }
</style>
