<script lang="ts">
  import { untrack } from "svelte";
  import type { Playback } from "../bindings/Playback";

  interface Props {
    playback: Playback;
    spotifyRunning: boolean;
    sleeveSrc: string | null;
    fallbackArtSrc: string | null;
    recordSrc: string | null;
    albumName: string | null;
    trackTitle: string | null;
    albumUrl: string | null;
    songUrl: string | null;
    progress: number;
    isHidden: boolean;
    reducedMotion: boolean;
  }

  let {
    playback,
    spotifyRunning,
    sleeveSrc,
    fallbackArtSrc,
    recordSrc,
    albumName,
    trackTitle,
    albumUrl,
    songUrl,
    progress,
    isHidden,
    reducedMotion,
  }: Props = $props();

  function openExternal(url: string | null): void {
    if (!url) return;
    window.open(url, "_blank", "noopener,noreferrer");
  }

  let spinningEl: HTMLDivElement | undefined = $state();
  let spinAnim: Animation | null = null;
  let rateRaf: number | null = null;
  let currentRate = 0;

  let activeSleeveSrc = $state<string | null>(null);
  let prevSleeveSrc = $state<string | null>(null);
  let sleeveFading = $state(false);
  let sleeveCssWear = $state(false);
  let prevSleeveCssWear = $state(false);
  let sleeveFadeTimer: ReturnType<typeof setTimeout> | null = null;

  let activeRecordSrc = $state<string | null>(null);
  let prevRecordSrc = $state<string | null>(null);
  let recordFading = $state(false);
  let recordFadeTimer: ReturnType<typeof setTimeout> | null = null;

  const isOut = $derived(spotifyRunning && playback !== "stopped");
  const isPlaying = $derived(spotifyRunning && playback === "playing");
  const isPaused = $derived(spotifyRunning && playback === "paused");

  const effectiveSleeveSrc = $derived(
    spotifyRunning ? (sleeveSrc ?? fallbackArtSrc) : null,
  );
  const usingCssWear = $derived(
    spotifyRunning && sleeveSrc === null && fallbackArtSrc !== null,
  );

  // Cross-fade sleeve on track/theme change
  $effect(() => {
    const nextSrc = effectiveSleeveSrc;
    const nextWear = usingCssWear;
    const prev = untrack(() => activeSleeveSrc);
    const prevWear = untrack(() => sleeveCssWear);

    if (nextSrc === prev && nextWear === prevWear) return;

    if (sleeveFadeTimer !== null) {
      clearTimeout(sleeveFadeTimer);
      sleeveFadeTimer = null;
    }

    if (reducedMotion || prev === null) {
      prevSleeveSrc = null;
      sleeveFading = false;
      activeSleeveSrc = nextSrc;
      sleeveCssWear = nextWear;
      return;
    }

    prevSleeveSrc = prev;
    prevSleeveCssWear = prevWear;
    activeSleeveSrc = nextSrc;
    sleeveCssWear = nextWear;
    sleeveFading = true;

    sleeveFadeTimer = setTimeout(() => {
      prevSleeveSrc = null;
      sleeveFading = false;
      sleeveFadeTimer = null;
    }, 300);
  });

  // Cross-fade record on track/theme change
  $effect(() => {
    const nextSrc = spotifyRunning ? recordSrc : null;
    const prev = untrack(() => activeRecordSrc);
    if (nextSrc === prev) return;

    if (recordFadeTimer !== null) {
      clearTimeout(recordFadeTimer);
      recordFadeTimer = null;
    }

    if (reducedMotion || prev === null) {
      prevRecordSrc = null;
      recordFading = false;
      activeRecordSrc = nextSrc;
      return;
    }

    prevRecordSrc = prev;
    activeRecordSrc = nextSrc;
    recordFading = true;

    recordFadeTimer = setTimeout(() => {
      prevRecordSrc = null;
      recordFading = false;
      recordFadeTimer = null;
    }, 300);
  });

  // Spin animation via Web Animations API (33 1/3 RPM = 1.8s per turn)
  $effect(() => {
    const el = spinningEl;
    if (!el) return;

    const anim = el.animate(
      [{ transform: "rotate(0deg)" }, { transform: "rotate(360deg)" }],
      { duration: 1800, iterations: Infinity, easing: "linear" },
    );
    anim.playbackRate = 0;
    anim.pause();
    spinAnim = anim;

    return () => {
      if (rateRaf !== null) {
        cancelAnimationFrame(rateRaf);
        rateRaf = null;
      }
      anim.cancel();
      spinAnim = null;
    };
  });

  $effect(() => {
    const anim = spinAnim;
    if (!anim) return;

    if (rateRaf !== null) {
      cancelAnimationFrame(rateRaf);
      rateRaf = null;
    }

    if (isHidden || reducedMotion || !spotifyRunning || playback === "stopped") {
      currentRate = 0;
      anim.playbackRate = 0;
      anim.pause();
      return;
    }

    const targetRate = isPlaying ? 1 : 0;
    if (currentRate === targetRate) {
      if (targetRate === 0) anim.pause();
      return;
    }

    const startRate = currentRate;
    const durationMs = targetRate === 1 ? 400 : 600;
    const startTime = performance.now();
    anim.play();

    const step = (now: number): void => {
      const t = Math.min(1, (now - startTime) / durationMs);
      // Ease out for spin-down, ease in-out for spin-up
      const eased = targetRate === 1 ? t * (2 - t) : 1 - (1 - t) * (1 - t);
      currentRate = startRate + (targetRate - startRate) * eased;
      anim.playbackRate = currentRate;

      if (t < 1) {
        rateRaf = requestAnimationFrame(step);
      } else {
        rateRaf = null;
        currentRate = targetRate;
        anim.playbackRate = targetRate;
        if (targetRate === 0) {
          anim.pause();
        }
      }
    };

    rateRaf = requestAnimationFrame(step);
  });

  // Tonearm angle: 0deg = rest, playing = lerp(21, 34, progress), paused = playing - 4.5
  const clampedProgress = $derived(Math.min(1, Math.max(0, progress)));
  const playingAngle = $derived(21 + 13 * clampedProgress);
  const armAngle = $derived(
    !isOut ? 0 : isPaused ? playingAngle - 4.5 : playingAngle,
  );
  const armTransition = $derived(
    reducedMotion
      ? "none"
      : !isOut
        ? "transform 0.7s ease-in-out, filter 0.3s ease-out"
        : isPaused
          ? "transform 0.2s ease-out, filter 0.2s ease-out"
          : "transform 0.25s ease-out, filter 0.2s ease-out",
  );
</script>

<div class="deck">
  <div
    class="record-slide"
    class:slid-in={!isOut}
    class:no-motion={reducedMotion}
  >
    <button
      type="button"
      class="record-hit"
      class:interactive={spotifyRunning && songUrl !== null}
      disabled={!spotifyRunning || songUrl === null}
      tabindex={spotifyRunning && songUrl !== null ? 0 : -1}
      aria-label={trackTitle ? `Open song ${trackTitle} on Spotify` : "Open song on Spotify"}
      onclick={() => openExternal(songUrl)}
    >
      <div class="record-spin" bind:this={spinningEl}>
        {#if prevRecordSrc}
          <img class="record-img prev" src={prevRecordSrc} alt="" draggable="false" />
        {/if}
        {#if activeRecordSrc}
          <img
            class="record-img"
            class:fade-in={recordFading && !reducedMotion}
            src={activeRecordSrc}
            alt=""
            draggable="false"
          />
        {:else}
          <div class="record-fallback">
            <div class="record-label-fallback"></div>
            <div class="record-spindle"></div>
          </div>
        {/if}
      </div>
      <div class="record-rim-light"></div>
      {#if spotifyRunning && songUrl !== null}
        <span class="hover-scrim record-scrim" class:no-motion={reducedMotion} aria-hidden="true">
          <span class="scrim-glyph record-glyph">
            <svg class="scrim-icon" width="22" height="22" viewBox="0 0 24 24" fill="none">
              <path
                d="M9 18V6l11-2v12"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx="6.5" cy="18" r="2.5" stroke="currentColor" stroke-width="1.8" />
              <circle cx="17.5" cy="16" r="2.5" stroke="currentColor" stroke-width="1.8" />
            </svg>
            <svg class="scrim-icon scrim-arrow" width="15" height="15" viewBox="0 0 16 16" fill="none">
              <path
                d="M5 11L11.5 4.5M6.5 4.5H11.5V9.5"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </span>
        </span>
      {/if}
    </button>
  </div>

  <button
    type="button"
    class="sleeve"
    class:interactive={spotifyRunning && albumUrl !== null}
    disabled={!spotifyRunning || albumUrl === null}
    tabindex={spotifyRunning && albumUrl !== null ? 0 : -1}
    aria-label={albumName ? `Open album ${albumName} on Spotify` : "Open album on Spotify"}
    onclick={() => openExternal(albumUrl)}
  >
    {#if prevSleeveSrc}
      <img
        class="sleeve-img prev"
        class:css-wear={prevSleeveCssWear}
        src={prevSleeveSrc}
        alt=""
        draggable="false"
      />
    {/if}
    {#if activeSleeveSrc}
      <img
        class="sleeve-img"
        class:css-wear={sleeveCssWear}
        class:fade-in={sleeveFading && !reducedMotion}
        src={activeSleeveSrc}
        alt=""
        draggable="false"
      />
    {:else}
      <div class="sleeve-blank"></div>
    {/if}
    {#if spotifyRunning && albumUrl !== null}
      <span class="hover-scrim sleeve-scrim" class:no-motion={reducedMotion} aria-hidden="true">
        <span class="scrim-glyph">
          <svg class="scrim-icon" width="26" height="26" viewBox="0 0 28 28" fill="none">
            <rect
              x="4"
              y="5"
              width="16"
              height="18"
              rx="1.5"
              stroke="currentColor"
              stroke-width="1.8"
            />
            <path
              d="M20 8.5a5.5 5.5 0 0 1 0 11"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
            <circle cx="12" cy="14" r="2.5" stroke="currentColor" stroke-width="1.6" />
          </svg>
          <svg class="scrim-icon scrim-arrow" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M5 11L11.5 4.5M6.5 4.5H11.5V9.5"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </span>
      </span>
    {/if}
  </button>

  <svg
    class="tonearm-layer"
    width="360"
    height="260"
    viewBox="0 0 360 260"
    fill="none"
    aria-hidden="true"
  >
    <defs>
      <linearGradient id="arm-rest-grad" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="#4A4540" />
        <stop offset="100%" stop-color="#2A2622" />
      </linearGradient>
      <radialGradient id="pivot-grad" cx="50%" cy="50%" r="50%">
        <stop offset="0%" stop-color="#D8D2CA" />
        <stop offset="100%" stop-color="#6E6862" />
      </radialGradient>
    </defs>

    <!-- Arm rest -->
    <rect
      x="332"
      y="156"
      width="18"
      height="7"
      rx="2"
      fill="url(#arm-rest-grad)"
      style="filter: drop-shadow(0 2px 2px rgba(0, 0, 0, 0.7))"
    />

    <!-- Rotating arm + headshell around pivot (338, 26) -->
    <g
      class="tonearm-arm"
      class:lifted={isPaused}
      style="
        transform-origin: 338px 26px;
        transform: {isPaused
          ? `translate(3px, -3px) rotate(${armAngle}deg)`
          : `rotate(${armAngle}deg)`};
        transition: {armTransition};
      "
    >
      <path
        d="M 338 26 L 344 70 L 341 150"
        stroke="#A9A39C"
        stroke-width="4"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <rect
        x="334.5"
        y="146"
        width="13"
        height="8"
        rx="1.5"
        fill="#77706A"
      />
    </g>

    <!-- Pivot cap -->
    <circle
      cx="338"
      cy="26"
      r="12"
      fill="url(#pivot-grad)"
      stroke="rgba(0, 0, 0, 0.5)"
      stroke-width="0.75"
    />
  </svg>
</div>

<style>
  .deck {
    position: absolute;
    inset: 0 0 auto 0;
    height: 260px;
    pointer-events: none;
  }

  .record-slide {
    position: absolute;
    left: 130px;
    top: 34px;
    width: 212px;
    height: 212px;
    transform: translateX(0);
    transition: transform 0.7s ease-in-out;
  }

  .record-slide.slid-in {
    transform: translateX(-84px);
  }

  .record-slide.no-motion {
    transition: none;
  }

  .record-hit {
    position: relative;
    width: 212px;
    height: 212px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    pointer-events: none;
    display: block;
  }

  .record-hit.interactive {
    pointer-events: auto;
    cursor: pointer;
  }

  .record-spin {
    position: relative;
    width: 212px;
    height: 212px;
    border-radius: 50%;
    filter: drop-shadow(0 6px 12px rgba(0, 0, 0, 0.6));
    will-change: transform;
  }

  .record-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    display: block;
  }

  .record-img.fade-in {
    animation: crossfade-in 0.3s ease-out;
  }

  .record-fallback {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    background: radial-gradient(
      circle,
      #141210 0%,
      #1c1917 34%,
      #11100f 36%,
      #191715 58%,
      #100f0e 76%,
      #161412 100%
    );
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .record-label-fallback {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: #2b2621;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .record-spindle {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #1e1b18;
  }

  .record-rim-light {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    border: 1.5px solid rgba(255, 255, 255, 0.2);
    background: conic-gradient(
      from 0deg,
      transparent 0%,
      rgba(255, 255, 255, 0.1) 25%,
      transparent 50%,
      rgba(255, 255, 255, 0.08) 75%,
      transparent 100%
    );
    pointer-events: none;
  }

  .sleeve {
    position: absolute;
    left: 16px;
    top: 22px;
    width: 232px;
    height: 232px;
    padding: 0;
    border: 0;
    transform: rotate(-2deg);
    box-shadow: 6px 8px 14px rgba(0, 0, 0, 0.7);
    overflow: hidden;
    background: #2b2621;
    pointer-events: none;
  }

  .sleeve.interactive {
    pointer-events: auto;
    cursor: pointer;
  }

  .hover-scrim {
    position: absolute;
    inset: 0;
    z-index: 5;
    background: rgba(54, 51, 48, 0.56);
    color: rgba(237, 228, 212, 0.92);
    opacity: 0;
    pointer-events: none;
    transition: opacity 150ms ease;
  }

  .hover-scrim.no-motion {
    transition: none;
  }

  .sleeve-scrim {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .record-scrim {
    border-radius: 50%;
  }

  .scrim-glyph {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .scrim-icon {
    filter: drop-shadow(0 2px 5px rgba(0, 0, 0, 0.65));
  }

  .scrim-arrow {
    opacity: 0.9;
  }

  .record-glyph {
    position: absolute;
    right: 24px;
    top: 50%;
    transform: translateY(-50%);
  }

  .sleeve.interactive:hover .sleeve-scrim,
  .sleeve.interactive:focus-visible .sleeve-scrim,
  .record-hit.interactive:hover .record-scrim,
  .record-hit.interactive:focus-visible .record-scrim {
    opacity: 1;
  }

  .sleeve-img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .sleeve-img.css-wear {
    filter: sepia(0.1) contrast(0.96);
  }

  .sleeve-img.fade-in {
    animation: crossfade-in 0.3s ease-out;
  }

  .sleeve-blank {
    width: 100%;
    height: 100%;
    background: radial-gradient(circle at 48% 52%, #312b25 0%, #26211d 70%, #201c18 100%);
    box-shadow: inset 0 0 24px rgba(0, 0, 0, 0.35);
  }

  .tonearm-layer {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
  }

  .tonearm-arm {
    filter: drop-shadow(3px 4px 3px rgba(0, 0, 0, 0.6));
  }

  .tonearm-arm.lifted {
    filter: drop-shadow(9px 13px 6px rgba(0, 0, 0, 0.45));
  }

  @keyframes crossfade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
