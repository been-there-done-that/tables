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
  import Logs from "@tabler/icons-svelte/icons/logs";
  import { invoke } from "@tauri-apps/api/core";
  import DataSource from "./components/datasource/DataSource.svelte";

  let { isFullScreen } = $props();
  let icons = $state(false);
  let datasourceWindowOpen = $state(true);


  const openDatasourceWindow = async () => {
    try {
      await invoke("open_datasource_window");
    } catch (e) {
      console.error("Failed to open datasource window:", e);
    }
  };
</script>

{#if !isFullScreen}
  <div
    class="fixed top-0 left-0 right-0 z-50 h-8 border-b"
    style="background: var(--theme-bg-secondary); border-color: var(--theme-border-default); color: var(--theme-fg-primary);"
  >
    <div
      class="relative z-10 flex h-full items-center justify-center gap-2 px-2 pointer-events-none"
      style="background: var(--theme-bg-secondary);"
    >
    <div data-tauri-drag-region class="flex items-center gap-2 pointer-events-auto w-full ml-20">
      <button class="h-6 w-6 rounded-md active:bg-accent" onclick={() => false}>
          <Logs class="size-4" />
        </button>

        <div class="absolute right-2 flex items-center gap-1 pointer-events-auto">

          <button
            class="h-6 w-6 rounded-md active:bg-accent mr-1 outline-none focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
            onclick={() => (datasourceWindowOpen = true)}
          >
            <PlaylistAdd class="size-6" />
          </button>

          <button class="h-6 w-6 rounded-md active:bg-accent" onclick={() => false}>
            <Logs class="size-4" />
          </button>

        <button class="h-6 w-6" onclick={() => false}>
          {#if false}
            <IconLayoutSidebarFilled class="size-4.5" />
          {:else}
            <IconLayoutSidebar class="size-4.5" />
          {/if}
        </button>

        <button class="h-6 w-6" onclick={() => false}>
          {#if false}
            <IconLayoutSidebarRightFilled class="size-4.5" />
          {:else}
            <IconLayoutSidebarRight class="size-4.5" />
          {/if}
        </button>

        <button class="h-6 w-6" onclick={() => window.location.reload()}>
          <IconRestore class="size-4.5" />
        </button>

        <button class="h-6 w-6" onclick={() => (icons = !icons)}>
          {#if icons}
            <IconSettingsFilled class="size-4.5" />
          {:else}
            <IconSettings class="size-4.5" />
          {/if}
        </button>
      </div>
    </div>
    </div>
  </div>
{/if}

<ResizableWindow
  title="New datasource"
  bind:open={datasourceWindowOpen}
  modal
  minWidth={920}
  minHeight={520}
  closeOnOverlayClick={false}
  contentClass="p-4 space-y-3"
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
