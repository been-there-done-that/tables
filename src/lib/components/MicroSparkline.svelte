<script lang="ts">
  export let values: number[] = [];
  export let max = 100;
  export let width = 32;
  export let height = 12;
  export let padding = 1;
  export let alpha = 0.35; // EMA smoothing factor

  function ema(samples: number[], a = alpha) {
    let prev = samples[0] ?? 0;
    return samples.map((v) => (prev = a * v + (1 - a) * prev));
  }

  $: smoothed = values.length ? ema(values, alpha) : [];

  $: path = (() => {
    if (smoothed.length === 0) return "";

    // If only one point, draw a flat line across to make it visible.
    if (smoothed.length === 1) {
      const v = smoothed[0];
      const y = height - padding - (Math.min(v, max) / max) * (height - padding * 2);
      return `M${padding},${y} L${width - padding},${y}`;
    }

    return smoothed
      .map((v, i) => {
        const x =
          padding + (i / Math.max(smoothed.length - 1, 1)) * (width - padding * 2);
        const y =
          height - padding - (Math.min(v, max) / max) * (height - padding * 2);
        return `${i === 0 ? "M" : "L"}${x},${y}`;
      })
      .join(" ");
  })();
</script>

<svg
  viewBox={`0 0 ${width} ${height}`}
  width={width}
  height={height}
  class="h-3 w-8 text-[--theme-fg-tertiary]"
>
  <path
    d={path}
    fill="none"
    stroke="currentColor"
    stroke-width="1"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="opacity-80"
  />
</svg>
