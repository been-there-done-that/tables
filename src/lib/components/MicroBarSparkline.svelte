<script lang="ts">
  /* ================= PROPS (RUNES) ================= */

  const {
    values = [],
    width = 56,
    height = 14,
    padding = 1,
    levels = 5,          // ▁▂▃▄▅
    maxBars = 10,
    barWidth = 4,
    gap = 1,
    maxAlpha = 0.15,
    floor = 0.1,
  } = $props<{
    values?: number[];
    width?: number;
    height?: number;
    padding?: number;
    levels?: number;
    maxBars?: number;
    barWidth?: number;
    gap?: number;
    maxAlpha?: number;
    floor?: number;
  }>();

  /* ================= HELPERS ================= */

  function computeRelativeMax(samples: number[], floorValue: number) {
    if (!samples.length) return floorValue;
    const sorted = [...samples].sort((a, b) => a - b);
    const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;
    return Math.max(p95 * 1.2, floorValue);
  }

  function smooth(prev: number, next: number, a: number) {
    return prev === 0 ? next : prev + a * (next - prev);
  }

  function quantize(v: number, max: number) {
    if (max <= 0) return 1;
    return Math.max(1, Math.round(Math.min(v / max, 1) * levels));
  }

  /* ================= STATE ================= */

  const floorValue = $derived(floor);
  let dynamicMax = $state(0);

  // keep only last N samples
  const samples = $derived(
    values.length > maxBars
      ? values.slice(-maxBars)
      : values
  );

  const target = $derived.by<number>(() => computeRelativeMax(samples, floorValue));
  $effect(() => {
    dynamicMax = smooth(dynamicMax, target, maxAlpha);
  });

  // keep dynamicMax seeded to current floor
  $effect(() => {
    if (dynamicMax < floorValue) dynamicMax = floorValue;
  });

  /* ================= BAR LAYOUT ================= */

  const bars = $derived.by(() => {
    if (!samples.length || dynamicMax <= 0) return [];

    return samples.map((v: number, i: number) => {
      const level = quantize(v, dynamicMax);
      const barHeight =
        Math.round((level / levels) * (height - padding * 2));

      return {
        x: padding + i * (barWidth + gap),
        y: height - padding - barHeight,
        w: barWidth,
        h: barHeight,
      };
    });
  });

  // index of newest bar
  const lastIndex = $derived(bars.length - 1);

  /* ================= COLOR ================= */

  const pressure = $derived(
    samples.length && dynamicMax
      ? Math.min(samples[samples.length - 1] / dynamicMax, 1)
      : 0
  );

  const color = $derived(() => {
    if (pressure < 0.5) return "var(--theme-fg-tertiary)";
    if (pressure < 0.8) return "var(--theme-accent)";
    return "var(--theme-accent-strong)";
  });
</script>

<svg
  viewBox={`0 0 ${width} ${height}`}
  width={width}
  height={height}
  class="h-3 w-14"
  shape-rendering="crispEdges"
  style="color: {color}"
>
  <style>
    /* ONLY the newest bar animates */
    .animate-in {
      transition:
        y 180ms ease-out,
        height 180ms ease-out;
    }
  </style>

  {#each bars as b, i (i)}
    <rect
      x={b.x}
      y={b.y}
      width={b.w}
      height={b.h}
      fill="currentColor"
      opacity="0.9"
      class:animate-in={i === lastIndex}
    />
  {/each}
</svg>
