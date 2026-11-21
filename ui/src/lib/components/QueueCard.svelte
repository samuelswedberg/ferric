<script lang="ts">
  import { cardTextureUrl } from "./textures";
</script>

<div class="queue-card" style="background-image: url({cardTextureUrl})">
  <section id="queue-slot" aria-label="Up next"></section>
</div>

<style>
  .queue-card {
    position: absolute;
    left: 16px;
    top: 410px;
    width: 328px;
    height: 96px;
    transform: rotate(-1.8deg);
    background-size: cover;
    background-position: center;
    background-color: #d6c8ac;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.6);
    padding: 8px 14px;
    color: var(--pencil);
  }

  #queue-slot {
    width: 100%;
    height: 100%;
  }
</style>
