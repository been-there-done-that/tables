<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getSystemMetrics } from "$lib/commands/client";
  import type { SystemMetrics } from "$lib/commands/types";

  let metrics = $state<SystemMetrics | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let intervalId: number | null = null;

  async function fetchMetrics() {
    try {
      const result = await getSystemMetrics();
      if (result.success && result.data) {
        metrics = result.data;
        error = null;
      } else {
        error = result.error || "Failed to fetch metrics";
      }
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function formatMemory(kb: number): string {
    if (kb < 1024) return `${kb} KB`;
    const mb = kb / 1024;
    if (mb < 1024) return `${mb.toFixed(1)} MB`;
    const gb = mb / 1024;
    return `${gb.toFixed(1)} GB`;
  }

  onMount(() => {
    fetchMetrics();
    // Update every 2 seconds
    intervalId = setInterval(fetchMetrics, 2000);
  });

  onDestroy(() => {
    if (intervalId) {
      clearInterval(intervalId);
    }
  });
</script>

{#if loading}
  <div class="flex items-center gap-2 text-xs text-[--theme-fg-tertiary]">
    <div class="size-2 bg-current rounded-full animate-pulse"></div>
    Loading...
  </div>
{:else if error}
  <div class="text-xs text-red-400" title={error}>Metrics unavailable</div>
{:else if metrics}
  <div class="flex items-center gap-3 text-xs text-[--theme-fg-tertiary]">
    <span title="CPU Usage">{metrics.cpu_usage.toFixed(1)}%</span>
    <span title="Memory Usage">{formatMemory(metrics.memory_kb)}</span>
    <span title="Threads">{metrics.thread_count}</span>
  </div>
{/if}
