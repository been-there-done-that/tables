<script lang="ts">
  import ResizableWindow from "$lib/components/ResizableWindow.svelte";
  import IconSettings from "@tabler/icons-svelte/icons/settings";
  import IconSettingsFilled from "@tabler/icons-svelte/icons/settings-filled";
  import IconLayoutSidebar from "@tabler/icons-svelte/icons/layout-sidebar";
  import IconLayoutSidebarFilled from "@tabler/icons-svelte/icons/layout-sidebar-filled";
  import IconLayoutSidebarRight from "@tabler/icons-svelte/icons/layout-sidebar-right";
  import IconLayoutSidebarRightFilled from "@tabler/icons-svelte/icons/layout-sidebar-right-filled";
  import IconRestore from "@tabler/icons-svelte/icons/restore";
  import PlaylistAdd from "@tabler/icons-svelte/icons/playlist-add";
  import IconPlus from "@tabler/icons-svelte/icons/plus";
  import Logs from "@tabler/icons-svelte/icons/logs";
  import { invoke } from "@tauri-apps/api/core";
  import { windowState } from "$lib/stores/window.svelte";
  import { cn } from "$lib/utils";
  import DataSource from "./components/datasource/DataSource.svelte";
  import ConnectionPicker from "$lib/components/ConnectionPicker.svelte";

  let { isFullScreen } = $props();
  let icons = $state(false);
  let datasourceWindowOpen = $state(false);

  const openDatasourceWindow = async () => {
    try {
      await invoke("open_datasource_window");
    } catch (e) {
      console.error("Failed to open datasource window:", e);
    }
  };

  const openSettingsWindow = async () => {
    try {
      await invoke("open_appearance_window");
    } catch (e) {
      console.error("Failed to open appearance window:", e);
    }
  };
</script>

{#if !isFullScreen}
  <div
    class="fixed top-0 left-0 right-0 z-50 h-8 border-b select-none"
    style="background: var(--theme-bg-secondary); border-color: var(--theme-border-default); color: var(--theme-fg-primary);"
  >
    <!-- 
      DEDICATED DRAG LAYER 
      This sits behind everything (z-0) and captures dragging events.
      We offset it by 80px to avoid interfering with Mac traffic lights.
    -->
    <div
      data-tauri-drag-region
      class="absolute inset-0 z-0 ml-20 pointer-events-auto"
      title="Draggable Region"
    ></div>

    <!-- 
      CONTENT LAYER
      We set pointer-events-none so dragging works through the empty spaces.
    -->
    <div
      class="relative z-10 flex h-full items-center justify-between px-2 pointer-events-none"
    >
      <!-- Left side (offset for Mac traffic lights) -->
      <div class="flex items-center gap-2 ml-20 pointer-events-auto">
        <!-- Left side items -->
      </div>

      <!-- Center Title (Optional) -->
      <div
        class="absolute inset-x-0 flex justify-center items-center h-full pointer-events-none"
      >
        <!-- Add window specific titles here if needed -->
        <div class="pointer-events-auto">
          {#if windowState.label === "main"}
            <ConnectionPicker />
          {/if}
        </div>
      </div>

      <!-- Right side actions -->
      <div class="flex items-center gap-2 pointer-events-auto pr-1">
        {#if windowState.label === "main"}
          <button
            class={cn(
              "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
              datasourceWindowOpen
                ? "bg-white/10 border-white/20"
                : "hover:bg-white/5 border-transparent",
            )}
            onclick={() => (datasourceWindowOpen = !datasourceWindowOpen)}
            title="New Datasource"
          >
            <PlaylistAdd class="size-6" />
          </button>

          <button
            class={cn(
              "h-6 w-7 flex items-center justify-center rounded-md border transition-all",
              windowState.datasourceWindowOpen
                ? "bg-white/10 border-white/20"
                : "hover:bg-white/5 border-transparent",
            )}
            onclick={openDatasourceWindow}
            title="External Datasource Window"
          >
            <IconPlus class="size-6" />
          </button>

          <button
            class="h-6 text-xs gap-1 flex items-center justify-center rounded-md hover:bg-white/5 active:bg-white/10"
            onclick={() => false}
          >
            {#if false}
              <IconLayoutSidebarFilled class="size-5" />
            {:else}
              <IconLayoutSidebar class="size-5" />
            {/if}
          </button>

          <button
            class="h-6 text-xs gap-1 flex items-center justify-center rounded-md hover:bg-white/5 active:bg-white/10"
            onclick={() => false}
          >
            {#if false}
              <IconLayoutSidebarRightFilled class="size-5" />
            {:else}
              <IconLayoutSidebarRight class="size-5" />
            {/if}
          </button>
        {/if}

        <button
          class="h-6 w-6 text-xs gap-1 flex items-center justify-center rounded-md hover:bg-white/5 active:bg-white/10"
          onclick={() => window.location.reload()}
          title="Reload Window"
        >
          <IconRestore class="size-5" />
        </button>

        {#if windowState.label === "main"}
          <button
            class={cn(
              "h-6 w-7 flex items-center justify-center rounded-md border transition-all",
              windowState.settingsWindowOpen
                ? "bg-white/10 border-white/20"
                : "hover:bg-white/5 border-transparent",
            )}
            onclick={openSettingsWindow}
            title="Settings"
          >
            {#if windowState.settingsWindowOpen}
              <IconSettingsFilled class="size-5" />
            {:else}
              <IconSettings class="size-5" />
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<ResizableWindow
  title="New datasource"
  bind:open={datasourceWindowOpen}
  minWidth={920}
  minHeight={520}
  closeOnOverlayClick={false}
  contentClass="space-y-3"
  debug={true}
  onClose={() => (datasourceWindowOpen = false)}
  openShortcut="ctrl+shift+n"
  closeShortcut="ctrl+shift+w"
>
  {#snippet children()}
    <DataSource />
  {/snippet}
  {#snippet headerActions()}
    <!-- no extra header actions -->
  {/snippet}
</ResizableWindow>
