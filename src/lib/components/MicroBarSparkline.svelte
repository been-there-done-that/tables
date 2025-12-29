<script lang="ts">
  /* ================= PROPS (RUNES) ================= */

  type HistoryItem = { id: number; val: number };

  const {
    values = [],
    // width is now calculated dynamically
    height = 14,
    padding = 1,
    levels = 5, // ▁▂▃▄▅
    maxBars = 30, // Configurable history size
    barWidth = 3, // Thinner bars for header
    gap = 1,
    maxAlpha = 0.15,
    floor = 0.1,
  } = $props<{
    values?: HistoryItem[];
    height?: number;
    padding?: number;
    levels?: number;
    maxBars?: number;
    barWidth?: number;
    gap?: number;
    maxAlpha?: number;
    floor?: number;
  }>();

  /* ================= LAYOUT CALC ================= */

  const totalWidth = $derived(
    padding * 2 + maxBars * barWidth + Math.max(0, maxBars - 1) * gap,
  );

  /* ================= HELPERS ================= */

  function computeRelativeMax(samples: HistoryItem[], floorValue: number) {
    if (!samples.length) return floorValue;
    // Map to numbers
    const nums = samples.map((s) => s.val);
    const sorted = [...nums].sort((a, b) => a - b);
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
    values.length > maxBars ? values.slice(-maxBars) : values,
  );

  const target = $derived.by<number>(() =>
    computeRelativeMax(samples, floorValue),
  );
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

    return samples.map((item: HistoryItem, i: number) => {
      const v = item.val;
      const level = quantize(v, dynamicMax);
      const barHeight = Math.round((level / levels) * (height - padding * 2));

      return {
        id: item.id, // Stable identity
        x: padding + i * (barWidth + gap),
        y: height - padding - barHeight,
        w: barWidth,
        h: barHeight,
      };
    });
  });

  /* ================= COLOR ================= */

  const pressure = $derived(
    samples.length && dynamicMax
      ? Math.min(samples[samples.length - 1].val / dynamicMax, 1)
      : 0,
  );

  const color = $derived(() => {
    if (pressure < 0.5) return "var(--theme-fg-tertiary)";
    if (pressure < 0.8) return "var(--theme-accent)";
    return "var(--theme-accent-strong)";
  });
</script>

<svg
  viewBox={`0 0 ${totalWidth} ${height}`}
  width={totalWidth}
  {height}
  class="overflow-visible"
  shape-rendering="crispEdges"
  style="color: {color}"
>
  <style>
    @keyframes grow {
      from {
        transform: scaleY(0);
      }
      to {
        transform: scaleY(1);
      }
    }
    .animate-in {
      transform-box: fill-box;
      transform-origin: bottom;
      animation: grow 600ms cubic-bezier(0.2, 0, 0, 1) forwards;
    }
    rect {
      /* Smoothly transition position and size for existing bars moving left */
      transition:
        x 0.3s cubic-bezier(0.2, 0, 0, 1),
        y 0.3s cubic-bezier(0.2, 0, 0, 1),
        height 0.3s cubic-bezier(0.2, 0, 0, 1);
    }
  </style>

  {#each bars as b (b.id)}
    <rect
      x={b.x}
      y={b.y}
      width={b.w}
      height={b.h}
      fill="currentColor"
      opacity="0.9"
      class:animate-in={true}
    />
  {/each}
</svg>
