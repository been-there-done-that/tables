<script lang="ts">
  import { windowState } from "$lib/stores/window.svelte";
  import MicroBarSparkline from "$lib/components/MicroBarSparkline.svelte";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";

  // Configuration
  const MAX_BARS = 60;

  // Clean state: Default to valid empty values if null
  let metrics = $derived(
    windowState.metrics || { cpu_percent: 0, pid: 0, threads: 0 },
  );
  let history = $derived(windowState.cpuHistory);

  // Animation for numbers
  const displayedCpu = new Tween(0, { duration: 600, easing: cubicOut });

  $effect(() => {
    displayedCpu.target = metrics.cpu_percent;
    console.log(history.length);
  });
</script>

<div
  class="flex items-center gap-2 text-xs text-(--theme-fg-tertiary) h-full select-none"
>
  <!-- CPU Section -->
  <div class="flex items-center gap-2" title="CPU Usage">
    <span class="font-mono tabular-nums"
      >{displayedCpu.current.toFixed(1)}%</span
    >
    <div class="group flex items-center pt-1 border rounded-md px-1">
      <MicroBarSparkline
        values={history}
        maxBars={MAX_BARS}
        barWidth={4}
        gap={1}
        height={120}
      />
    </div>
  </div>

  <span title="Process ID" class="font-mono opacity-50">PID {metrics.pid}</span>
</div>
