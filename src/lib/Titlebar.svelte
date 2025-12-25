<script lang="ts">
  import IconSettings from "@tabler/icons-svelte/icons/settings";
  import IconSettingsFilled from "@tabler/icons-svelte/icons/settings-filled";
  import IconLayoutSidebar from "@tabler/icons-svelte/icons/layout-sidebar";
  import IconLayoutSidebarFilled from "@tabler/icons-svelte/icons/layout-sidebar-filled";
  import IconLayoutSidebarRight from "@tabler/icons-svelte/icons/layout-sidebar-right";
  import IconLayoutSidebarRightFilled from "@tabler/icons-svelte/icons/layout-sidebar-right-filled";
  import IconRestore from "@tabler/icons-svelte/icons/restore";
  import Logs from "@tabler/icons-svelte/icons/logs";

  let { isFullScreen } = $props();
  let icons = $state(false);
</script>

{#if !isFullScreen}
  <div
    class="fixed top-0 left-0 right-0 z-50 h-8 border-b"
    style="background: var(--theme-bg-secondary); border-color: var(--theme-border-default); color: var(--theme-fg-primary);"
  >
    <div data-tauri-drag-region class="absolute inset-0 h-full w-full"></div>

    <div
      class="relative z-10 flex h-full items-center justify-center gap-2 px-2 pointer-events-none"
      style="background: var(--theme-bg-secondary);"
    >
      <div class="flex items-center gap-2 pointer-events-auto"></div>

      <div class="absolute right-2 flex items-center gap-1 pointer-events-auto">
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
{/if}
