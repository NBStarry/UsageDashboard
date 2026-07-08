<script lang="ts">
  import { barColor } from '$lib/theme';

  interface Props {
    pct: number;
  }
  let { pct }: Props = $props();

  const clamped = $derived(Math.min(100, Math.max(0, pct)));
  const fill = $derived(barColor(clamped));
</script>

<!-- 7px tall progress bar; track rgba(255,255,255,0.12); fill = barColor(pct); corner radius 4 -->
<div class="track">
  <div class="fill" style="width: {clamped}%; background-color: {fill};"></div>
</div>

<style>
.track {
  position: relative;
  width: 100%;
  height: 7px;
  background: rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  overflow: hidden;
}

.fill {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  border-radius: 4px;
  transition: width 0.4s ease-out;
}
</style>
