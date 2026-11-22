<script lang="ts">
  import type { PlayerState } from "../../bindings/PlayerState";
  import { albumLink, trackLink } from "../../spotifyLinks";
  import { formatTime, playerStore, resolveArtSrc } from "../../state.svelte";
  import KeyBank from "../KeyBank.svelte";
  import ProgressSlider from "../ProgressSlider.svelte";
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

  let { state, positionMs, isHidden, reducedMotion, volumeDisplay, onModeSwitch }: Props = $props();

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
  const volumeActive = $derived(typeof volumeDisplay === "number");
  const timeText = $derived.by(() => {
    if (volumeDisplay === "unknown") return "VOL —";
    if (typeof volumeDisplay === "number") return `VOL ${volumeDisplay}`;
    if (!spotifyRunning || !track) return "";
    const current = isStopped ? "0:00" : formatTime(positionMs);
    return `${current} / ${durationMs === null ? "--:--" : formatTime(durationMs)}`;
  });

  const sleeveSrc = $derived(resolveArtSrc(state.theme?.sleeve));
  const fallbackArtSrc = $derived(resolveArtSrc(track?.art));
  const recordSrc = $derived(resolveArtSrc(state.theme?.record));
  const albumName = $derived(spotifyRunning ? (track?.album ?? track?.title ?? null) : null);
  const trackTitle = $derived(spotifyRunning ? (track?.title ?? null) : null);
  const songUrl = $derived(spotifyRunning && track ? trackLink(track) : null);
  const albumUrl = $derived(spotifyRunning && track ? albumLink(track) : null);

  function onDoubleClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) onModeSwitch();
  }
</script>

<main
  class="plinth card-plinth"
  data-tauri-drag-region
  style="background-image: url({plinthTextureUrl})"
  ondblclick={onDoubleClick}
>
  <WindowChrome variant="card" {onModeSwitch} />
  <RecordDeck
    variant="card"
    playback={state.playback}
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

  <h1 class="title" title={titleText}>{titleText}</h1>
  {#if artistText}<p class="artist" title={artistText}>{artistText}</p>{/if}
  {#if timeText}<span class="time">{timeText}</span>{/if}

  <ProgressSlider
    variant="card"
    {positionMs}
    {durationMs}
    {spotifyRunning}
    {isStopped}
    displayRatio={volumeActive ? (volumeDisplay as number) / 100 : null}
    volumeMode={volumeActive}
    {reducedMotion}
    onSeek={(ms) => void playerStore.runCommand({ type: "seek", positionMs: ms })}
  />
  <KeyBank
    variant="card"
    playback={state.playback}
    {spotifyRunning}
    liked={state.liked}
    {reducedMotion}
    onPrevious={() => void playerStore.runCommand({ type: "previous" })}
    onPlayPause={() => void playerStore.runCommand({ type: "playPause" })}
    onNext={() => void playerStore.runCommand({ type: "next" })}
    onToggleLike={() =>
      void playerStore.runCommand({ type: "setLiked", liked: !(state.liked ?? false) })}
    onOpenSpotify={() => void playerStore.runOpenSpotify()}
  />
  <div
    class="grain-overlay"
    style="background-image: url({grainTextureUrl}); opacity: {playerStore.grainOpacity}"
    aria-hidden="true"
  ></div>
</main>

<style>
  .card-plinth {
    position: relative;
    width: 360px;
    height: 120px;
    border-radius: 14px;
    overflow: hidden;
    background-color: var(--plinth);
    background-size: cover;
    background-position: center;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.42);
  }

  .title {
    position: absolute;
    left: 162px;
    top: 12px;
    width: 184px;
    margin: 0;
    color: var(--ink);
    font: 600 18px/1.15 "Bahnschrift", "Segoe UI", sans-serif;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .artist {
    position: absolute;
    left: 162px;
    top: 35px;
    width: 108px;
    margin: 0;
    color: var(--ink-muted);
    font: 400 12px/1.2 "Bahnschrift", "Segoe UI", sans-serif;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .time {
    position: absolute;
    right: 14px;
    top: 35px;
    color: var(--ink-muted);
    font: 700 10px/1.2 "Courier New", monospace;
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
