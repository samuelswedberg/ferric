<script lang="ts">
  import { onMount } from "svelte";
  import type { Track } from "../../bindings/Track";
  import { onboarding } from "../onboarding/controller.svelte";
  import { openSpotifyLink, trackLink } from "../../spotifyLinks";
  import { formatTime, playerStore } from "../../state.svelte";

  interface Props {
    queue: Track[] | null;
  }

  let { queue }: Props = $props();
  let queueEl: HTMLDivElement | undefined = $state();
  let menuIndex = $state<number | null>(null);
  let menuX = $state(0);
  let menuY = $state(0);

  const visibleTracks = $derived(queue?.slice(0, 3) ?? []);

  function duration(track: Track): string {
    return track.durationMs === null ? "--:--" : formatTime(track.durationMs);
  }

  function jumpTo(index: number): void {
    if (index > 9) return;
    menuIndex = null;
    void playerStore.runCommand({ type: "jumpTo", queueIndex: index });
  }

  function openMenu(event: MouseEvent, index: number): void {
    event.preventDefault();
    if (!queueEl) return;
    const rect = queueEl.getBoundingClientRect();
    menuIndex = index;
    menuX = Math.min(205, Math.max(0, event.clientX - rect.left));
    menuY = Math.min(39, Math.max(10, event.clientY - rect.top));
  }

  async function copyLink(track: Track): Promise<void> {
    menuIndex = null;
    await navigator.clipboard.writeText(trackLink(track).web);
  }

  function openInSpotify(track: Track): void {
    menuIndex = null;
    void openSpotifyLink(trackLink(track));
  }

  function closeMenu(event: PointerEvent): void {
    if (event.target instanceof Node && queueEl?.contains(event.target)) return;
    menuIndex = null;
  }

  onMount(() => {
    window.addEventListener("pointerdown", closeMenu);
    return () => window.removeEventListener("pointerdown", closeMenu);
  });
</script>

<div class="handwritten-queue" bind:this={queueEl}>
  <h2>side one</h2>
  {#if queue === null}
    <div class="signed-out">
      <span>sign in to see what's next</span>
      <!-- T10 wires this button to the sign-in flow. -->
      <button type="button" onclick={() => onboarding.show(4)}>Sign in</button>
    </div>
  {:else if visibleTracks.length === 0}
    <p class="empty">end of side</p>
  {:else}
    <ol>
      {#each visibleTracks as track, index (`${track.spotifyId ?? track.title}-${index}`)}
        <li>
          <button
            type="button"
            class="queue-row"
            onclick={() => jumpTo(index)}
            oncontextmenu={(event) => openMenu(event, index)}
            aria-label={`Play ${track.title} from the queue`}
          >
            <span class="track-number">{index + 2}.</span>
            <span class="track-title">{track.title}</span>
            <span class="duration">{duration(track)}</span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}

  {#if menuIndex !== null && visibleTracks[menuIndex]}
    <div class="context-menu" style="left: {menuX}px; top: {menuY}px" role="menu">
      <button type="button" role="menuitem" onclick={() => copyLink(visibleTracks[menuIndex!])}>
        Copy link
      </button>
      <button
        type="button"
        role="menuitem"
        onclick={() => openInSpotify(visibleTracks[menuIndex!])}
      >
        Open in Spotify
      </button>
    </div>
  {/if}
</div>

<style>
  .handwritten-queue {
    position: relative;
    width: 100%;
    height: 100%;
    color: var(--pencil);
    font-family: "Segoe Print", "Comic Sans MS", cursive;
    font-size: 13px;
    line-height: 1.45;
  }

  h2 {
    margin: -1px 0 0;
    font-size: 11px;
    font-weight: 700;
    line-height: 1.25;
    letter-spacing: 0.02em;
  }

  ol {
    margin: 1px 0 0;
    padding: 0;
    list-style: none;
  }

  li {
    height: 20px;
  }

  .queue-row {
    position: relative;
    width: 100%;
    height: 20px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr) 42px;
    align-items: baseline;
  }

  .track-title {
    position: relative;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .track-title::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 1px;
    background: currentColor;
    opacity: 0;
    transform: rotate(-0.5deg) scaleX(0.96);
    transition: opacity 100ms ease;
  }

  .queue-row:hover .track-title::after,
  .queue-row:focus-visible .track-title::after {
    opacity: 0.75;
  }

  .duration {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .signed-out {
    margin-top: 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .signed-out button {
    flex: 0 0 auto;
    height: 22px;
    padding: 0 10px;
    border: 0;
    border-radius: 3px;
    color: var(--printed-icon);
    background: linear-gradient(180deg, var(--key-top), var(--key-bottom));
    box-shadow: 0 2px 0 rgba(0, 0, 0, 0.45), inset 0 1px rgba(255, 255, 255, 0.12);
    font: 700 10px/1 "Courier New", monospace;
    cursor: default;
  }

  .empty {
    margin: 14px 0 0;
  }

  .context-menu {
    position: absolute;
    z-index: 5;
    width: 118px;
    padding: 3px;
    border: 1px solid rgba(47, 42, 38, 0.35);
    border-radius: 3px;
    background: #d8ccb0;
    box-shadow: 1px 3px 7px rgba(0, 0, 0, 0.35);
  }

  .context-menu button {
    width: 100%;
    padding: 2px 5px;
    border: 0;
    background: transparent;
    color: var(--pencil);
    font: 700 9px/1.35 "Courier New", monospace;
    text-align: left;
    cursor: pointer;
  }

  .context-menu button:hover,
  .context-menu button:focus-visible {
    background: rgba(47, 42, 38, 0.12);
  }
</style>
