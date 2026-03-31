<script lang="ts">
import IconSettings from "@tabler/icons-svelte/icons/settings";
  import IconAi from "@tabler/icons-svelte/icons/ai";
  import IconSettingsFilled from "@tabler/icons-svelte/icons/settings-filled";
  import IconLayoutSidebar from "@tabler/icons-svelte/icons/layout-sidebar";
  import IconLayoutSidebarFilled from "@tabler/icons-svelte/icons/layout-sidebar-filled";
  import IconLayoutSidebarRight from "@tabler/icons-svelte/icons/layout-sidebar-right";
  import IconLayoutSidebarRightFilled from "@tabler/icons-svelte/icons/layout-sidebar-right-filled";
  import SystemMetricsWidget from "$lib/components/SystemMetricsWidget.svelte";

  import IconLayoutBottombar from "@tabler/icons-svelte/icons/layout-bottombar";
  import IconLayoutBottombarFilled from "@tabler/icons-svelte/icons/layout-bottombar-filled";

  import IconMessageReport from "@tabler/icons-svelte/icons/message-report";
  import IconRestore from "@tabler/icons-svelte/icons/restore";
  import PlaylistAdd from "@tabler/icons-svelte/icons/playlist-add";
  import IconPlus from "@tabler/icons-svelte/icons/plus";
  import Logs from "@tabler/icons-svelte/icons/logs";
  import { invoke } from "@tauri-apps/api/core";
  import { windowState } from "$lib/stores/window.svelte";
  import { schemaStore } from "$lib/stores/schema.svelte";
  import { logsStore } from "$lib/stores/logs.svelte";
  import { cn } from "$lib/utils";
  import ConnectionPicker from "$lib/components/ConnectionPicker.svelte";
  import * as Menu from "$lib/components/ui/dropdown-menu";
  import IconDatabaseAlt from "$lib/svg/IconDatabaseAlt.svelte";
  import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
  import IconLoader from "@tabler/icons-svelte/icons/loader";

  import WindowControls from "$lib/components/WindowControls.svelte";
  import UpdateChip from '$lib/components/UpdateChip.svelte';
  import { updaterStore } from '$lib/stores/updater.svelte';

  let { isFullScreen } = $props();
  // let icons = $state(false);
  let isDbPickerOpen = $state(false);

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

  function handleKeydown(e: KeyboardEvent) {
    if (schemaStore.activeConnection?.engine === "postgres") {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "p") {
        e.preventDefault();
        isDbPickerOpen = !isDbPickerOpen;
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

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
        {#if !["datasource-window", "appearance-window", "feedback-window"].includes(windowState.label)}
          <ConnectionPicker />

          {#if schemaStore.activeConnection?.engine === "postgres"}
            <div class="h-4 w-px bg-border mx-1"></div>
            <Menu.Root bind:open={isDbPickerOpen}>
              <Menu.Trigger
                class={cn(
                  "group flex items-center gap-2 p-1 text-xs font-medium rounded-md transition-all duration-200",
                  "hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)",
                  "border border-transparent focus:outline-none",
                  isDbPickerOpen ? "bg-(--theme-bg-active)" : "",
                )}
              >
                <div class="flex items-center gap-2 px-2">
                  <IconDatabaseAlt
                    class="size-3.5 text-(--theme-accent-primary)"
                  />
                  <span class="text-(--theme-fg-primary)"
                    >{schemaStore.selectedDatabase || "Select DB"}</span
                  >
                  {#if schemaStore.status === "connecting" || schemaStore.status === "refreshing" || schemaStore.databases.find((d) => d.name === schemaStore.selectedDatabase)?.is_loading}
                    <div class="flex items-center justify-center size-4">
                      <IconLoader class="size-3.5 animate-spin opacity-50" />
                    </div>
                  {/if}
                  <IconChevronDown
                    class={cn(
                      "size-4 opacity-50 transition-transform duration-200",
                      isDbPickerOpen && "rotate-180",
                    )}
                  />
                </div>
              </Menu.Trigger>
              <Menu.Content
                align="center"
                sideOffset={8}
                class="min-w-48 w-max max-w-[400px] max-h-80 overflow-auto z-50 p-1 bg-(--theme-bg-secondary) border border-(--theme-border-default)"
                onCloseAutoFocus={(e) => e.preventDefault()}
              >
                <Menu.Label
                  class="px-2 py-1.5 text-xs font-semibold text-muted-foreground"
                  >Databases</Menu.Label
                >
                <Menu.Separator class="my-1 h-px bg-border" />
                <Menu.RadioGroup
                  value={schemaStore.selectedDatabase ??
                    schemaStore.activeConnection?.database ??
                    ""}
                  onValueChange={(val) => schemaStore.selectDatabase(val)}
                >
                  {#each schemaStore.databases as db (db.name)}
                    <Menu.RadioItem
                      value={db.name}
                      class="flex items-center gap-2 pl-8 pr-2 py-1.5 text-xs outline-none select-none data-[state=checked]:bg-accent/5 data-[state=checked]:text-accent-foreground cursor-default"
                    >
                      <span class="flex-1 truncate">{db.name}</span>
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
        <UpdateChip />
        {#if !["datasource-window", "appearance-window", "feedback-window"].includes(windowState.label)}
          <button
            class="inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2rem] px-2 rounded-md border border-border bg-background shadow-sm transition-colors hover:bg-accent hover:text-foreground text-muted-foreground"
            onclick={async () => {
              try {
                await invoke("create_new_window");
              } catch (e) {
                console.error("Failed to create new window:", e);
              }
            }}
            title="New Window"
          >
            <IconPlus class="size-4" />
            <span class="text-[9px] leading-none opacity-60">Window</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.5rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              windowState.datasourceWindowOpen
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={openDatasourceWindow}
            title="New Datasource Connection"
            id="datasource-btn"
          >
            <PlaylistAdd class="size-4" />
            <span class="text-[9px] leading-none opacity-60">Connect</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.5rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              windowState.layout.left
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={() => (windowState.layout.left = !windowState.layout.left)}
            title="Toggle Explorer"
          >
            {#if windowState.layout.left}
              <IconLayoutSidebarFilled class="size-4" />
            {:else}
              <IconLayoutSidebar class="size-4" />
            {/if}
            <span class="text-[9px] leading-none opacity-60">Explorer</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.5rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              windowState.layout.bottom
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={() =>
              (windowState.layout.bottom = !windowState.layout.bottom)}
            title="Toggle Output Panel"
          >
            {#if windowState.layout.bottom}
              <IconLayoutBottombarFilled class="size-4" />
            {:else}
              <IconLayoutBottombar class="size-4" />
            {/if}
            <span class="text-[9px] leading-none opacity-60">Output</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              logsStore.isOpen
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={() => logsStore.toggle()}
            title="Toggle Query Logs"
          >
            <Logs class="size-4" />
            <span class="text-[9px] leading-none opacity-60">Logs</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.25rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              windowState.layout.right
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={() =>
              (windowState.layout.right = !windowState.layout.right)}
            title="Toggle Agent Panel"
          >
            {#if windowState.layout.right}
              <IconLayoutSidebarRightFilled class="size-4" />
            {:else}
              <IconLayoutSidebarRight class="size-4" />
            {/if}
            <span class="text-[9px] leading-none opacity-60">Agent</span>
          </button>

          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.75rem] px-2 rounded-md border shadow-sm transition-colors text-muted-foreground",
              windowState.settingsWindowOpen
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground",
            )}
            onclick={openSettingsWindow}
            title="Settings"
          >
            {#if windowState.settingsWindowOpen}
              <IconSettingsFilled class="size-4" />
            {:else}
              <IconSettings class="size-4" />
            {/if}
            <span class="text-[9px] leading-none opacity-60">Settings</span>
          </button>

          <!-- Feedback -->
          <button
            class="inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[3rem] px-2 rounded-md border border-border bg-background shadow-sm transition-colors hover:bg-accent hover:text-foreground text-muted-foreground"
            onclick={async () => {
              try {
                await invoke("open_feedback_window");
              } catch (e) {
                console.error("Failed to open feedback window:", e);
              }
            }}
            title="Send Feedback"
          >
            <IconMessageReport class="size-4" />
            <span class="text-[9px] leading-none opacity-60">Feedback</span>
          </button>

          <!-- AI Assistant -->
          <button
            class={cn(
              "inline-flex flex-col items-center justify-center gap-0.5 h-7 min-w-[2.25rem] px-2 rounded-md border shadow-sm transition-colors",
              windowState.layout.right && windowState.activeRightPanel === "claude"
                ? "bg-accent text-accent-foreground border-accent/50"
                : "bg-background border-border hover:bg-accent hover:text-foreground text-muted-foreground",
            )}
            onclick={() => windowState.toggleRightPanel("claude")}
            title="AI Assistant"
          >
            <IconAi class="size-4 transition-colors" />
            <span class="text-[9px] leading-none opacity-60">Claude</span>
          </button>
        {/if}

        <button
          class="inline-flex h-6 w-6 items-center justify-center rounded-md border border-border bg-background shadow-sm transition-colors hover:bg-accent hover:text-foreground text-muted-foreground"
          onclick={() => window.location.reload()}
          title="Reload Window"
        >
          <IconRestore class="size-4" />
        </button>

        {#if isWindows}
          <WindowControls />
        {/if}
      </div>
    </div>
    {#if updaterStore.status === 'downloading'}
      <div class="absolute bottom-0 left-0 right-0 h-[2px]" style="background: var(--theme-border-subtle);">
        <div
          class="h-full transition-[width] duration-200"
          style="width: {updaterStore.downloadPercent}%; background: var(--theme-accent-primary);"
        ></div>
      </div>
    {/if}
  </div>
{/if}

