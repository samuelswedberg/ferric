<script lang="ts">
  interface Props {
    message: string | null;
  }

  let { message }: Props = $props();
</script>

{#if message}
  <div class="toast" role="status" aria-live="polite">
    <span class="toast-text">{message}</span>
  </div>
{/if}

<style>
  .toast {
    position: absolute;
    left: 24px;
    right: 24px;
    bottom: 8px;
    z-index: 50;
    padding: 5px 12px;
    background: linear-gradient(180deg, #e5d8b2 0%, #dcd0a8 100%);
    color: #2f2a26;
    border-radius: 2px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.55);
    transform: rotate(-0.4deg);
    text-align: center;
    pointer-events: none;
  }

  .toast-text {
    font-family: "Courier New", monospace;
    font-size: 11px;
    font-weight: 700;
    line-height: 1.3;
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
