<script lang="ts">
  /* ---------------- props (runes) ---------------- */

  const {
    values = [],
    width = 32,
    height = 12,
    padding = 1,
    maxAlpha = 0.15, // smoothing for scale envelope
    floor = 0.1,     // minimum visible scale
  } = $props<{
    values?: number[];
    width?: number;
    height?: number;
    padding?: number;
    maxAlpha?: number;
    floor?: number;
  }>();

  /* ---------------- helpers ---------------- */

  function computeRelativeMax(samples: number[]) {
    if (samples.length === 0) return floor;

    // p95 gives good spike sensitivity without noise
    const sorted = [...samples].sort((a, b) => a - b);
    const p95 = sorted[Math.floor(sorted.length * 0.95)] ?? 0;

    return Math.max(p95 * 1.2, floor);
  }

  function smooth(prev: number, next: number, a: number) {
    return prev === 0 ? next : prev + a * (next - prev);
  }

  /* ---------------- derived state ---------------- */

  let dynamicMax = $state(0);

  // raw samples → spikes preserved
  const samples = $derived(values.length ? values : []);

  // adaptive envelope (stable)
  const target = $derived(computeRelativeMax(samples));
  $effect(() => {
    dynamicMax = smooth(dynamicMax, target, maxAlpha);
  });

  /* ---------------- path generation ---------------- */

  const path = $derived.by<string>(() => {
    if (samples.length === 0 || dynamicMax <= 0) return "";

    if (samples.length === 1) {
      const v = samples[0];
      const y =
        height -
        padding -
        Math.min(v / dynamicMax, 1) *
          (height - padding * 2);

      return `M${padding},${y} L${width - padding},${y}`;
    }

    return samples
      .map((v: number, i: number) => {
        const x =
          padding +
          (i / (samples.length - 1)) *
            (width - padding * 2);

        const y =
          height -
          padding -
          Math.min(v / dynamicMax, 1) *
            (height - padding * 2);

        return `${i === 0 ? "M" : "L"}${x},${y}`;
      })
      .join(" ");
  });

  /* ---------------- pressure-based color ---------------- */

  // relative to current envelope (0–1)
  const pressure = $derived(
    samples.length && dynamicMax
      ? Math.min(samples[samples.length - 1] / dynamicMax, 1)
      : 0
  );

  // calm, non-alarmist color shift
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
  class="h-3 w-8"
  style="color: {color}"
>
  <!-- baseline (idle reference) -->
  <line
    x1={padding}
    x2={width - padding}
    y1={height - padding}
    y2={height - padding}
    stroke="currentColor"
    stroke-opacity="0.15"
    stroke-width="0.5"
  />

  <!-- spiky sparkline -->
  <path
    d={path}
    fill="none"
    stroke="currentColor"
    stroke-width="0.75"
    stroke-linecap="round"
    stroke-linejoin="round"
    class="opacity-80"
  />
</svg>
