<script lang="ts">
  import { onMount } from "svelte";

  interface Props {
    text: string;
    reducedMotion?: boolean;
  }

  let { text, reducedMotion = false }: Props = $props();

  /** Ticker speed for text wider than its box, in px per second. */
  const SPEED_PX_PER_S = 30;
  /** Space between the end of the text and the start of the next copy. */
  const GAP_PX = 36;

  let box: HTMLSpanElement | undefined = $state();
  let inner: HTMLSpanElement | undefined = $state();
  let textWidth = $state(0);
  let overflows = $state(false);

  function measure(): void {
    if (!box || !inner) return;
    textWidth = inner.scrollWidth;
    overflows = textWidth > box.clientWidth + 1;
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

  // One loop moves exactly one text width plus the gap, so the copy lands where the
  // original started and the loop is seamless.
  const durationS = $derived((textWidth + GAP_PX) / SPEED_PX_PER_S);
  const scrolling = $derived(overflows && !reducedMotion);
</script>

<!-- Fits: shown as is. Too wide: loops continuously like a ticker; never "…". -->
<span class="scroll-box" bind:this={box} title={text}>
  <span
    class="track"
    class:scrolling
    style="--loop: {-(textWidth + GAP_PX)}px; --duration: {durationS}s; --gap: {GAP_PX}px"
  >
    <span class="copy" bind:this={inner}>{text}</span>
    {#if scrolling}<span class="copy" aria-hidden="true">{text}</span>{/if}
  </span>
</span>

<style>
  .scroll-box {
    display: block;
    overflow: hidden;
    white-space: nowrap;
  }

  .track {
    display: inline-flex;
    white-space: nowrap;
  }

  .scrolling {
    gap: var(--gap);
    animation: ticker var(--duration) linear infinite;
  }

  .copy {
    flex: none;
  }

  @keyframes ticker {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(var(--loop));
    }
  }
</style>
