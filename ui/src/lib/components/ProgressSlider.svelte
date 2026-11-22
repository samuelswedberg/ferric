<script lang="ts">
  interface Props {
    variant?: "full" | "card";
    positionMs: number;
    durationMs: number | null;
    spotifyRunning: boolean;
    isStopped: boolean;
    displayRatio?: number | null;
    volumeMode?: boolean;
    reducedMotion?: boolean;
    onSeek: (positionMs: number) => void;
  }

  let {
    variant = "full",
    positionMs,
    durationMs,
    spotifyRunning,
    isStopped,
    displayRatio = null,
    volumeMode = false,
    reducedMotion = false,
    onSeek,
  }: Props = $props();

  let trackEl: HTMLDivElement | undefined = $state();
  let dragging = $state(false);
  let dragRatio = $state(0);

  const enabled = $derived(spotifyRunning && durationMs !== null && durationMs > 0);
  const baseRatio = $derived(
    !enabled || isStopped || durationMs === null
      ? 0
      : Math.min(1, Math.max(0, positionMs / durationMs)),
  );
  const ratio = $derived(
    dragging ? dragRatio : displayRatio === null ? baseRatio : Math.min(1, Math.max(0, displayRatio)),
  );
  const capLeftPx = $derived(Math.round(ratio * (variant === "card" ? 174 : 306)));
  const playedWidthPx = $derived(
    Math.max(0, Math.round(ratio * (variant === "card" ? 182 : 318))),
  );

  function ratioFromClientX(clientX: number): number {
    if (!trackEl) return 0;
    const rect = trackEl.getBoundingClientRect();
    const rel = (clientX - rect.left) / rect.width;
    return Math.min(1, Math.max(0, rel));
  }

  function onPointerDown(event: PointerEvent): void {
    if (!enabled || !trackEl) return;
    event.preventDefault();
    trackEl.setPointerCapture(event.pointerId);
    dragging = true;
    dragRatio = ratioFromClientX(event.clientX);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!dragging) return;
    dragRatio = ratioFromClientX(event.clientX);
  }

  function onPointerUp(event: PointerEvent): void {
    if (!dragging) return;
    dragging = false;
    if (trackEl?.hasPointerCapture(event.pointerId)) {
      trackEl.releasePointerCapture(event.pointerId);
    }
    if (enabled && durationMs !== null) {
      onSeek(Math.round(dragRatio * durationMs));
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (!enabled || durationMs === null) return;
    const stepMs = 5000;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      event.stopPropagation();
      onSeek(Math.max(0, positionMs - stepMs));
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      event.stopPropagation();
      onSeek(Math.min(durationMs, positionMs + stepMs));
    }
  }
</script>

<div
  class="slider"
  class:card={variant === "card"}
  class:volume={volumeMode}
  class:no-motion={reducedMotion}
  class:disabled={!enabled}
  bind:this={trackEl}
  role="slider"
  tabindex={enabled ? 0 : -1}
  aria-label="Seek"
  aria-disabled={!enabled}
  aria-valuemin={0}
  aria-valuemax={durationMs ?? 0}
  aria-valuenow={Math.round(ratio * (durationMs ?? 0))}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  onkeydown={onKeyDown}
>
  <div class="slot">
    {#if enabled && (!isStopped || dragging) && playedWidthPx > 0}
      <div class="played" style="width: {playedWidthPx}px"></div>
    {/if}
  </div>

  <svg class="ticks" width="320" height="4" viewBox="0 0 320 4" aria-hidden="true">
    <line x1="0.5" y1="0" x2="0.5" y2="4" />
    <line x1="40" y1="1" x2="40" y2="3" />
    <line x1="80" y1="1" x2="80" y2="3" />
    <line x1="120" y1="1" x2="120" y2="3" />
    <line x1="160" y1="0" x2="160" y2="4" />
    <line x1="200" y1="1" x2="200" y2="3" />
    <line x1="240" y1="1" x2="240" y2="3" />
    <line x1="280" y1="1" x2="280" y2="3" />
    <line x1="319.5" y1="0" x2="319.5" y2="4" />
  </svg>

  <div class="cap" style="transform: translateX({capLeftPx}px)">
    <span class="ridge"></span>
    <span class="ridge"></span>
    <span class="ridge"></span>
    <span class="ridge"></span>
  </div>
</div>

<style>
  .slider {
    position: absolute;
    left: 20px;
    top: 326px;
    width: 320px;
    height: 24px;
    cursor: pointer;
    touch-action: none;
  }

  .slider.disabled {
    cursor: default;
  }

  .slider.disabled .cap {
    opacity: 0.55;
  }

  .slot {
    position: absolute;
    left: 0;
    top: 8px;
    width: 320px;
    height: 6px;
    border-radius: 3px;
    background: #0a0908;
    box-shadow:
      inset 0 1px 2px rgba(0, 0, 0, 0.9),
      0 1px 0 rgba(255, 255, 255, 0.08);
    overflow: hidden;
  }

  .played {
    position: absolute;
    left: 1px;
    top: 2px;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
    transition: background-color 0.3s ease;
  }

  .ticks {
    position: absolute;
    left: 0;
    top: 19px;
    pointer-events: none;
  }

  .ticks line {
    stroke: rgba(217, 207, 192, 0.4);
    stroke-width: 1;
  }

  .cap {
    position: absolute;
    left: 0;
    top: 0;
    width: 14px;
    height: 22px;
    border-radius: 2px;
    background: linear-gradient(90deg, #d8d2c8 0%, #8f8880 100%);
    border: 0.75px solid #000000;
    box-shadow: 1px 2px 2px rgba(0, 0, 0, 0.7);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    pointer-events: none;
  }

  .ridge {
    width: 8px;
    height: 1px;
    background: rgba(0, 0, 0, 0.35);
  }

  .slider.card {
    left: 162px;
    top: 51px;
    width: 184px;
    height: 18px;
  }

  .slider.card .slot {
    top: 6px;
    width: 184px;
    height: 4px;
    border-radius: 2px;
  }

  .slider.card .played {
    top: 1px;
    height: 2px;
  }

  .slider.card .ticks {
    display: none;
  }

  .slider.card .cap {
    width: 10px;
    height: 16px;
  }

  .slider.card .ridge {
    width: 6px;
    gap: 1px;
  }

  .slider.volume .played {
    background: var(--volume);
  }

  .slider .played {
    transition: background-color 200ms ease;
  }

  .slider.no-motion .played {
    transition: none;
  }

  .slider.no-motion .cap {
    transition: none;
  }
</style>
