<script lang="ts">
  import { metrics } from "$lib/stores/metrics.svelte";
  import MicroBarSparkline from "$lib/components/MicroBarSparkline.svelte";
  import { Tween } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  import { METRICS } from "$lib/constants";
  import { toast } from "svelte-sonner";

  // Clean state
  let history = $derived(metrics.cpuHistory);
  let cpuPercent = $derived(metrics.cpu);
  let pid = $derived(metrics.pid);

  // Animation for numbers
  const displayedCpu = new Tween(0, { duration: 600, easing: cubicOut });

  // Format memory
  const memoryFormatted = $derived.by(() => {
    const bytes = metrics.memory ?? 0;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    if (bytes < 1024 * 1024 * 1024)
      return `${Math.round(bytes / (1024 * 1024))} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
  });

  $effect(() => {
    displayedCpu.target = cpuPercent;
  });

  function copyPid(pid: number) {
    console.log(pid);
    navigator.clipboard.writeText(pid.toString());
    toast.success("PID copied to clipboard");
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
    <div class="h-3 w-px bg-(--theme-border) mx-1"></div>
    <span class="font-mono tabular-nums" title="Memory Usage"
      >{memoryFormatted}</span
    >
    <div class="group flex items-center">
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
    onclick={() => copyPid(pid)}
  >
    (PID: <span class="opacity-100">{pid}</span>)
  </button>
</div>
