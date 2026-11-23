<script lang="ts">
  import { windowDrag } from "../../windowDrag";
  import type { PlayerState } from "../../bindings/PlayerState";
  import { artistLink, openSpotifyLink, trackLink, type SpotifyLink } from "../../spotifyLinks";
  import { formatTime, playerStore, resolveArtSrc } from "../../state.svelte";
  import KeyBank from "../KeyBank.svelte";
  import ProgressSlider from "../ProgressSlider.svelte";
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
  const songUrl = $derived(spotifyRunning && track ? trackLink(track) : null);
  const artistUrl = $derived(spotifyRunning && track ? artistLink(track) : null);

  function openExternal(link: SpotifyLink | null): void {
    if (!link) return;
    void openSpotifyLink(link);
  }
</script>

<main
  class="plinth card-plinth"
  use:windowDrag={onModeSwitch}
  style="background-image: url({plinthTextureUrl})"
>
  <WindowChrome variant="card" {onModeSwitch} />
  <RecordDeck
    variant="card"
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
      void playerStore.runCommand({ type: "setLiked", liked: !(state.liked ?? false) })}
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
    overflow: hidden;
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
