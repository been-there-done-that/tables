<script lang="ts">
  import ResizableWindow from "$lib/components/ResizableWindow.svelte";
  import IconSettings from "@tabler/icons-svelte/icons/settings";
  import IconSettingsFilled from "@tabler/icons-svelte/icons/settings-filled";
  import IconLayoutSidebar from "@tabler/icons-svelte/icons/layout-sidebar";
  import IconLayoutSidebarFilled from "@tabler/icons-svelte/icons/layout-sidebar-filled";
  import IconLayoutSidebarRight from "@tabler/icons-svelte/icons/layout-sidebar-right";
  import IconLayoutSidebarRightFilled from "@tabler/icons-svelte/icons/layout-sidebar-right-filled";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";

  import IconLayoutBottombar from "@tabler/icons-svelte/icons/layout-bottombar";
  import IconLayoutBottombarFilled from "@tabler/icons-svelte/icons/layout-bottombar-filled";

  import IconRestore from "@tabler/icons-svelte/icons/restore";
  import PlaylistAdd from "@tabler/icons-svelte/icons/playlist-add";
  import IconPlus from "@tabler/icons-svelte/icons/plus";
  import Logs from "@tabler/icons-svelte/icons/logs";
  import { invoke } from "@tauri-apps/api/core";
  import { windowState } from "$lib/stores/window.svelte";
  import { schemaStore } from "$lib/stores/schema.svelte";
  import { cn } from "$lib/utils";
  import DataSource from "./components/datasource/DataSource.svelte";
  import ConnectionPicker from "$lib/components/ConnectionPicker.svelte";
  import * as Menu from "$lib/components/ui/dropdown-menu";
  import IconDatabase from "@tabler/icons-svelte/icons/database";
  import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

  import WindowControls from "$lib/components/WindowControls.svelte";

  let { isFullScreen } = $props();
  // let icons = $state(false);
  let datasourceWindowOpen = $state(false);

  // Use user agent to detect Windows.
  // Note: This is a simple check. For improved reliability might want to use `@tauri-apps/plugin-os` in future.
  const isWindows = navigator.userAgent.includes("Windows");

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
      class={cn(
        "absolute inset-0 z-0 pointer-events-auto",
        !isWindows && "ml-20",
      )}
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
      <div
        class={cn(
          "flex items-center gap-2 pointer-events-auto",
          !isWindows && "ml-24",
        )}
      >
        {#if !["datasource-window", "appearance-window"].includes(windowState.label)}
          <ConnectionPicker />

          {#if schemaStore.activeConnection?.engine === "postgres"}
            <div class="h-4 w-px bg-border mx-1"></div>
            <!-- Database Picker -->
            <Menu.Root>
              <Menu.Trigger>
                <button
                  class="flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded-md hover:bg-(--theme-bg-hover) text-(--theme-fg-secondary) hover:text-(--theme-fg-primary) transition-colors"
                >
                  <IconDatabase class="size-3.5 opacity-70" />
                  <span>{schemaStore.selectedDatabase || "Select DB"}</span>
                  <IconChevronDown class="size-3 opacity-50" />
                </button>
              </Menu.Trigger>
              <Menu.Content align="start" class="max-h-[300px] overflow-auto">
                <Menu.Label>Databases</Menu.Label>
                <Menu.Separator />
                <Menu.RadioGroup
                  value={schemaStore.selectedDatabase ?? undefined}
                  onValueChange={(val) => schemaStore.selectDatabase(val)}
                >
                  {#each schemaStore.databases as db (db.name)}
                    <Menu.RadioItem value={db.name}>
                      {db.name}
                    </Menu.RadioItem>
                  {/each}
                </Menu.RadioGroup>
              </Menu.Content>
            </Menu.Root>
          {/if}
        {/if}
      </div>

      <!-- Center Title (Optional) -->
      <div
        class="absolute inset-x-0 flex justify-center items-center h-full pointer-events-none"
      >
        <!-- Add window specific titles here if needed -->
        <SystemMetricsWidget />
        <div class="pointer-events-auto"></div>
      </div>

      <!-- Right side actions -->
      <div class="flex items-center gap-3 pointer-events-auto">
        {#if !["datasource-window", "appearance-window"].includes(windowState.label)}
          <button
            class="h-6 w-6 flex items-center justify-center rounded-md border transition-all hover:bg-(--theme-bg-hover) border-transparent"
            onclick={async () => {
              try {
                await invoke("create_new_window");
              } catch (e) {
                console.error("Failed to create new window:", e);
              }
            }}
            title="New Window"
          >
            <IconPlus class="size-6" />
          </button>

          <button
            class={cn(
              "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
              datasourceWindowOpen
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
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
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
            )}
            onclick={openDatasourceWindow}
            title="External Datasource Window"
            id="datasource-btn"
          >
            <IconPlus class="size-6" />
          </button>

          <button
            class={cn(
              "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
              windowState.layout.left
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
            )}
            onclick={() => (windowState.layout.left = !windowState.layout.left)}
          >
            {#if windowState.layout.left}
              <IconLayoutSidebarFilled class="size-5" />
            {:else}
              <IconLayoutSidebar class="size-5" />
            {/if}
          </button>

          <button
            class={cn(
              "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
              windowState.layout.bottom
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
            )}
            onclick={() =>
              (windowState.layout.bottom = !windowState.layout.bottom)}
          >
            {#if windowState.layout.bottom}
              <IconLayoutBottombarFilled class="size-5" />
            {:else}
              <IconLayoutBottombar class="size-5" />
            {/if}
          </button>

          <button
            class={cn(
              "h-6 w-6 flex items-center justify-center rounded-md border transition-all",
              windowState.layout.right
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
            )}
            onclick={() =>
              (windowState.layout.right = !windowState.layout.right)}
          >
            {#if windowState.layout.right}
              <IconLayoutSidebarRightFilled class="size-5" />
            {:else}
              <IconLayoutSidebarRight class="size-5" />
            {/if}
          </button>

          <button
            class={cn(
              "h-6 w-7 flex items-center justify-center rounded-md border transition-all",
              windowState.settingsWindowOpen
                ? "bg-(--theme-bg-active) border-(--theme-border-subtle)"
                : "hover:bg-(--theme-bg-hover) border-transparent",
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

        <button
          class="h-6 w-6 flex items-center justify-center rounded-md border transition-all border-transparent hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)"
          onclick={() => window.location.reload()}
          title="Reload Window"
        >
          <IconRestore class="size-5" />
        </button>

        {#if isWindows}
          <WindowControls />
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
