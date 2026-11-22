<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    text: string;
    reducedMotion?: boolean;
  }

  let { text, reducedMotion = false }: Props = $props();

  /** Scroll speed for text wider than its box, in px per second. */
  const SPEED_PX_PER_S = 28;

  let box: HTMLSpanElement | undefined = $state();
  let inner: HTMLSpanElement | undefined = $state();
  let overflowPx = $state(0);

  function measure(): void {
    if (!box || !inner) return;
    overflowPx = Math.max(0, Math.ceil(inner.scrollWidth - box.clientWidth));
  }

  $effect(() => {
    text;
    queueMicrotask(measure);
  });

  onMount(() => {
    const observer = new ResizeObserver(measure);
    if (box) observer.observe(box);
    if (inner) observer.observe(inner);
    measure();
    return () => observer.disconnect();
  });

  // Travel time plus the held ends (the keyframes hold 20% at each end).
  const durationS = $derived(Math.max(4, overflowPx / SPEED_PX_PER_S / 0.6));
</script>

<!-- Fits: shown as is. Too wide: scrolls back and forth, pausing at each end; never "…". -->
<span class="scroll-box" bind:this={box} title={text}>
  <span
    class="scroll-inner"
    class:scrolling={overflowPx > 0 && !reducedMotion}
    bind:this={inner}
    style="--shift: {-overflowPx}px; --duration: {durationS}s"
  >
    {text}
  </span>
</span>

<style>
  .scroll-box {
    display: block;
    overflow: hidden;
    white-space: nowrap;
  }

  .scroll-inner {
    display: inline-block;
    white-space: nowrap;
  }

  .scrolling {
    animation: scroll-text var(--duration) ease-in-out infinite alternate;
  }

  @keyframes scroll-text {
    0%,
    20% {
      transform: translateX(0);
    }
    80%,
    100% {
      transform: translateX(var(--shift));
    }
  }
</style>
