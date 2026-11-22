<script lang="ts">
  import { windowDrag } from "../../windowDrag";
  import { onMount } from "svelte";
  import type { PlayerState } from "../../bindings/PlayerState";
  import { albumLink, trackLink } from "../../spotifyLinks";
  import { formatTime, playerStore, resolveArtSrc } from "../../state.svelte";
  import KeyBank from "../KeyBank.svelte";
  import RecordDeck from "../RecordDeck.svelte";
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
  let titleEl: HTMLSpanElement | undefined = $state();
  let artistEl: HTMLSpanElement | undefined = $state();
  let titleOverflow = $state(false);
  let artistOverflow = $state(false);

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
  const albumName = $derived(spotifyRunning ? (track?.album ?? track?.title ?? null) : null);
  const trackTitle = $derived(spotifyRunning ? (track?.title ?? null) : null);
  const songUrl = $derived(spotifyRunning && track ? trackLink(track) : null);
  const albumUrl = $derived(spotifyRunning && track ? albumLink(track) : null);

  function measureOverflow(): void {
    titleOverflow = !!titleEl && titleEl.scrollWidth > titleEl.clientWidth;
    artistOverflow = !!artistEl && artistEl.scrollWidth > artistEl.clientWidth;
  }

  $effect(() => {
    titleText;
    artistText;
    queueMicrotask(measureOverflow);
  });

  onMount(() => {
    const observer = new ResizeObserver(measureOverflow);
    if (titleEl) observer.observe(titleEl);
    if (artistEl) observer.observe(artistEl);
    measureOverflow();
    return () => observer.disconnect();
  });

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
    {albumName}
    {trackTitle}
    {albumUrl}
    {songUrl}
    {progress}
    {isHidden}
    {reducedMotion}
  />

  <span class="title clip" class:marquee={titleOverflow} bind:this={titleEl} title={titleText}>
    <span>{titleText}</span>
  </span>
  {#if artistText}
    <span class="artist clip" class:marquee={artistOverflow} bind:this={artistEl} title={artistText}>
      <span>{artistText}</span>
    </span>
  {/if}
  {#if timeText}<span class="time">{timeText}</span>{/if}

  <div class="control-fade" aria-hidden="true"></div>
  <KeyBank
    variant="strip"
    playback={playerState.playback}
    {spotifyRunning}
    liked={playerState.liked}
    {reducedMotion}
    onPrevious={() => void playerStore.runCommand({ type: "previous" })}
    onPlayPause={() => void playerStore.runCommand({ type: "playPause" })}
    onNext={() => void playerStore.runCommand({ type: "next" })}
    onToggleLike={() =>
      void playerStore.runCommand({ type: "setLiked", liked: !(playerState.liked ?? false) })}
    onOpenSpotify={() => void playerStore.runOpenSpotify()}
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

  .marquee > span {
    display: inline-block;
    min-width: 100%;
    padding-right: 18px;
    animation: marquee 6s linear 1s infinite alternate;
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
    left: 192px;
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
    transition: width 200ms ease, background-color 200ms ease;
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

  .no-motion .marquee > span {
    animation: none;
  }

  @keyframes marquee {
    from { transform: translateX(0); }
    to { transform: translateX(calc(-100% + 94px)); }
  }
</style>
