<script lang="ts">
  import type { PlayerState } from "../bindings/PlayerState";
  import { formatTime, playerStore, resolveArtSrc } from "../state.svelte";
  import ErrorToast from "./ErrorToast.svelte";
  import KeyBank from "./KeyBank.svelte";
  import ProgressSlider from "./ProgressSlider.svelte";
  import QueueCard from "./QueueCard.svelte";
  import RecordDeck from "./RecordDeck.svelte";
  import { grainTextureUrl, plinthTextureUrl } from "./textures";
  import VolumeKnob from "./VolumeKnob.svelte";
  import WindowChrome from "./WindowChrome.svelte";

  interface Props {
    state: PlayerState;
    positionMs: number;
    toastMessage: string | null;
    isHidden: boolean;
    reducedMotion: boolean;
  }

  let { state, positionMs, toastMessage, isHidden, reducedMotion }: Props = $props();

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
</script>

<main
  class="plinth"
  data-tauri-drag-region
  style="background-image: url({plinthTextureUrl})"
>
  <WindowChrome />

  <RecordDeck
    playback={state.playback}
    {spotifyRunning}
    {sleeveSrc}
    {fallbackArtSrc}
    {recordSrc}
    {progress}
    {isHidden}
    {reducedMotion}
  />

  <h1 class="title" title={titleText}>{titleText}</h1>

  {#if artistText}
    <p class="artist" title={artistText}>{artistText}</p>
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
    playback={state.playback}
    {spotifyRunning}
    liked={state.liked}
    {reducedMotion}
    onPrevious={() => void playerStore.runCommand({ type: "previous" })}
    onPlayPause={() => void playerStore.runCommand({ type: "playPause" })}
    onNext={() => void playerStore.runCommand({ type: "next" })}
    onToggleLike={() =>
      void playerStore.runCommand({ type: "setLiked", liked: !(state.liked ?? false) })
    }
    onOpenSpotify={() => void playerStore.runOpenSpotify()}
  />

  <VolumeKnob
    volumePct={state.volumePct}
    {spotifyRunning}
    onSetVolume={(pct) => void playerStore.runCommand({ type: "setVolume", pct })}
  />

  <QueueCard />

  <ErrorToast message={toastMessage} />

  <div class="grain-overlay" style="background-image: url({grainTextureUrl})" aria-hidden="true"></div>
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
    max-width: 320px;
    font-family: "Bahnschrift", "Segoe UI Variable", "Segoe UI", sans-serif;
    font-weight: 600;
    font-size: 24px;
    line-height: 1.2;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .artist {
    position: absolute;
    left: 20px;
    top: 300px;
    margin: 0;
    max-width: 210px;
    font-family: "Bahnschrift", "Segoe UI Variable", "Segoe UI", sans-serif;
    font-weight: 400;
    font-size: 14px;
    line-height: 1.2;
    color: var(--ink-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
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
    opacity: 0.2;
    pointer-events: none;
  }
</style>
