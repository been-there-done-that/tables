<script lang="ts">
  import { windowState } from "$lib/stores/window.svelte";
  import MicroBarSparkline from "$lib/components/MicroBarSparkline.svelte";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { METRICS } from "$lib/constants";

  // Clean state: Default to valid empty values if null
  let metrics = $derived(
    windowState.metrics || { cpu_percent: 0, pid: 0, threads: 0 },
  );
  let history = $derived(windowState.cpuHistory);

  // Animation for numbers
  const displayedCpu = new Tween(0, { duration: 600, easing: cubicOut });

  $effect(() => {
    displayedCpu.target = metrics.cpu_percent;
  });

  function copyPid(pid: number) {
    console.log(pid);
    navigator.clipboard.writeText(pid.toString());
  }
</script>

<div
  class="flex items-end gap-2 text-xs text-(--theme-fg-tertiary) h-full select-none pointer-events-auto"
>
  <!-- CPU Section -->
  <div class="flex items-end gap-2" title="CPU Usage">
    <span class="font-mono tabular-nums"
      >{displayedCpu.current.toFixed(1)}%</span
    >
    <div class="group flex items-center pt-1 border-b px-1">
      <MicroBarSparkline
        values={history}
        maxBars={METRICS.HISTORY_SIZE}
        barWidth={METRICS.BAR_WIDTH}
        gap={1}
        height={METRICS.CHART_HEIGHT}
      />
    </div>
  </div>
  <button
    class="font-mono hover:text-(--theme-fg-primary)"
    title="Click to copy Process ID"
    onclick={() => copyPid(metrics.pid)}
  >
    (PID: <span class="opacity-100">{metrics.pid}</span>)
  </button>
</div>
