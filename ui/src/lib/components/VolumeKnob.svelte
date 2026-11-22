<script lang="ts">
  interface Props {
    volumePct: number | null;
    spotifyRunning: boolean;
    onSetVolume: (pct: number) => void;
  }

  let { volumePct, spotifyRunning, onSetVolume }: Props = $props();

  let knobEl: HTMLDivElement | undefined = $state();
  let dragging = $state(false);
  let dragStartY = 0;
  let dragStartPct = 0;
  let lastSentPct = -1;
  /** The value the user just set, shown until Spotify's reported volume catches up. */
  let localPct = $state<number | null>(null);
  let localTimer: ReturnType<typeof setTimeout> | null = null;
  const LOCAL_HOLD_MS = 2000;

  const enabled = $derived(spotifyRunning && volumePct !== null);
  const currentPct = $derived(localPct ?? volumePct ?? 0);

  function setLocal(pct: number): void {
    localPct = pct;
    if (localTimer !== null) clearTimeout(localTimer);
    localTimer = setTimeout(() => {
      localTimer = null;
      if (!dragging) localPct = null;
    }, LOCAL_HOLD_MS);
    onSetVolume(pct);
  }
  const pointerAngleDeg = $derived(-135 + (currentPct / 100) * 270);

  function snapStep(pct: number): number {
    return Math.min(100, Math.max(0, Math.round(pct / 5) * 5));
  }

  function onPointerDown(event: PointerEvent): void {
    if (!enabled || !knobEl) return;
    event.preventDefault();
    knobEl.setPointerCapture(event.pointerId);
    dragging = true;
    dragStartY = event.clientY;
    dragStartPct = currentPct;
    lastSentPct = snapStep(currentPct);
  }

  function onPointerMove(event: PointerEvent): void {
    if (!dragging || !enabled) return;
    const deltaY = dragStartY - event.clientY;
    const nextPct = snapStep(dragStartPct + deltaY * 0.75);
    if (nextPct !== lastSentPct) {
      lastSentPct = nextPct;
      setLocal(nextPct);
    }
  }

  function onPointerUp(event: PointerEvent): void {
    if (!dragging) return;
    dragging = false;
    if (localTimer === null) localPct = null;
    if (knobEl?.hasPointerCapture(event.pointerId)) {
      knobEl.releasePointerCapture(event.pointerId);
    }
  }

  function onWheel(event: WheelEvent): void {
    if (!enabled) return;
    event.preventDefault();
    const delta = event.deltaY < 0 ? 5 : -5;
    const nextPct = snapStep(currentPct + delta);
    if (nextPct !== currentPct) {
      setLocal(nextPct);
    }
  }

  function onKeyDown(event: KeyboardEvent): void {
    if (!enabled) return;
    if (event.key === "ArrowUp" || event.key === "ArrowRight") {
      event.preventDefault();
      event.stopPropagation();
      setLocal(snapStep(currentPct + 5));
    } else if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
      event.preventDefault();
      event.stopPropagation();
      setLocal(snapStep(currentPct - 5));
    }
  }
</script>

<div
  class="knob"
  class:disabled={!enabled}
  bind:this={knobEl}
  role="slider"
  tabindex={enabled ? 0 : -1}
  aria-label="Volume"
  aria-disabled={!enabled}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={currentPct}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  onwheel={onWheel}
  onkeydown={onKeyDown}
>
  <div class="cap">
    <div class="pointer" style="transform: rotate({pointerAngleDeg}deg)"></div>
  </div>
</div>

<style>
  .knob {
    position: absolute;
    left: 294px;
    top: 352px;
    width: 42px;
    height: 42px;
    border-radius: 50%;
    border: 1px solid #0b0a09;
    background: conic-gradient(
      from 0deg,
      #2a2724 0%,
      #4a4540 12.5%,
      #211f1c 25%,
      #46413c 37.5%,
      #2a2724 50%,
      #4a4540 62.5%,
      #211f1c 75%,
      #46413c 87.5%,
      #2a2724 100%
    );
    box-shadow:
      0 3px 2px rgba(0, 0, 0, 0.7),
      inset 0 1px 0 rgba(255, 255, 255, 0.12);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: ns-resize;
    touch-action: none;
  }

  .knob.disabled {
    opacity: 0.45;
    cursor: default;
  }

  .cap {
    position: relative;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: radial-gradient(circle at 50% 50%, #3e3a35 0%, #25221f 100%);
    border: 0.75px solid rgba(0, 0, 0, 0.5);
  }

  .pointer {
    position: absolute;
    left: 50%;
    top: 2px;
    width: 2px;
    height: 8px;
    margin-left: -1px;
    border-radius: 1px;
    background: #d9cfc0;
    transform-origin: 1px 9.25px;
  }
</style>
