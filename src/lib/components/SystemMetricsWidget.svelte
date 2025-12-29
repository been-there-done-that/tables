<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import type { SystemMetrics } from "$lib/commands/types";

  let metrics = $state<SystemMetrics | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let unlisten: UnlistenFn | null = null;

  onMount(() => {
    (async () => {
      try {
        unlisten = await listen<SystemMetrics>("metrics:update", (event) => {
          metrics = event.payload;
          loading = false;
          error = null;
        });
      } catch (err) {
        error = String(err);
        loading = false;
      }
    })();
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
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
    <span title="CPU Usage (normalized)">
      {metrics.cpu_percent.toFixed(1)}%
    </span>
    <span title="Threads">{metrics.threads}</span>
    <span title="PID">PID {metrics.pid}</span>
  </div>
{/if}
