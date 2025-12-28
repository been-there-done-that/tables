<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listConnections } from "$lib/commands/client";
  import type { Connection } from "$lib/commands/types";
  import { onMount } from "svelte";
  import IconDatabase from "@tabler/icons-svelte/icons/database";

  let showDatasource = $state(false);
  let connections = $state<Connection[]>([]);
  let loading = $state(true);

  const openSettingsWindow = async () => {
    try {
      await invoke("open_appearance_window");
    } catch (e) {
      console.error("Failed to open appearance window:", e);
    }
  };

  async function loadConnections() {
    loading = true;
    try {
      const result = await listConnections();
      if (result.success && result.data) {
        connections = result.data;
      }
    } catch (err) {
      console.error("Failed to load connections:", err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadConnections();
  });
</script>

<div
  class="h-full flex flex-col p-6 bg-[--theme-bg-primary] text-[--theme-fg-primary]"
>
  <div class="flex justify-between items-center mb-6">
    <h1 class="text-xl font-medium tracking-tight">Connections</h1>
    <button
      class="px-3 py-1.5 text-xs font-medium bg-[--theme-bg-tertiary] hover:bg-[--theme-bg-secondary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] rounded transition-colors"
      onclick={loadConnections}
    >
      Refresh
    </button>
  </div>

  {#if loading}
    <div class="text-[--theme-fg-secondary] text-sm">
      Loading connections...
    </div>
  {:else if connections.length === 0}
    <div
      class="flex flex-col items-center justify-center py-12 text-[--theme-fg-tertiary]"
    >
      <IconDatabase class="size-12 mb-3 opacity-20" />
      <p class="text-sm">No connections found</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each connections as conn}
        <div
          class="p-4 rounded-lg border border-[--theme-border-default] bg-[--theme-bg-secondary]/30 hover:bg-[--theme-bg-secondary]/50 transition-colors group"
        >
          <div class="flex items-start justify-between mb-2">
            <div class="flex items-center gap-2">
              <div
                class="size-8 rounded flex items-center justify-center bg-[--theme-bg-tertiary] text-[--theme-fg-secondary]"
              >
                <IconDatabase class="size-5" />
              </div>
              <div>
                <div class="font-medium text-sm text-[--theme-fg-primary]">
                  {conn.name}
                </div>
                <div class="text-xs text-[--theme-fg-tertiary] capitalize">
                  {conn.engine}
                </div>
              </div>
            </div>

            {#if conn.is_favorite}
              <div class="size-1.5 rounded-full bg-amber-400"></div>
            {/if}
          </div>

          <div
            class="mt-4 pt-3 border-t border-[--theme-border-default] flex justify-between items-center text-xs text-[--theme-fg-tertiary]"
          >
            <span>{conn.host || "Local"}</span>
            <span>{conn.port || "-"}</span>
          </div>

          <div class="mt-3 pt-3 border-t border-[--theme-border-default]">
            <pre
              class="text-[10px] leading-tight text-[--theme-fg-tertiary] overflow-x-auto p-2 bg-[--theme-bg-tertiary]/50 rounded">{JSON.stringify(
                conn,
                null,
                2,
              )}</pre>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
