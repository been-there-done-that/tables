<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import IconMinimize from "@tabler/icons-svelte/icons/minus";
  import IconMaximize from "@tabler/icons-svelte/icons/square";
  import IconRestore from "@tabler/icons-svelte/icons/copy"; // Often used for restore/unmaximize
  import IconClose from "@tabler/icons-svelte/icons/x";
  import { cn } from "$lib/utils";

  const appWindow = getCurrentWindow();
  let isMaximized = $state(false);

  // Monitor maximization state
  $effect(() => {
    const updateState = async () => {
        isMaximized = await appWindow.isMaximized();
    };
    
    updateState();
    
    // Listen for resize events to update maximized state
    const unlisten = appWindow.listen("tauri://resize", updateState);
    
    return () => {
        unlisten.then(f => f());
    }
  });

  async function minimize() {
    await appWindow.minimize();
  }

  async function toggleMaximize() {
    await appWindow.toggleMaximize();
    isMaximized = await appWindow.isMaximized();
  }

  async function close() {
    await appWindow.close();
  }
</script>

<div class="flex items-center h-full gap-1 mr-2 no-drag">
  <button
    class="h-6 w-8 flex items-center justify-center rounded-md hover:bg-white/10 transition-colors"
    onclick={minimize}
    title="Minimize"
  >
    <IconMinimize class="size-4 opacity-80" />
  </button>
  <button
    class="h-6 w-8 flex items-center justify-center rounded-md hover:bg-white/10 transition-colors"
    onclick={toggleMaximize}
    title={isMaximized ? "Restore" : "Maximize"}
  >
    {#if isMaximized}
        <IconRestore class="size-4 opacity-80" />
    {:else}
        <IconMaximize class="size-4 opacity-80" />
    {/if}
  </button>
  <button
    class="h-6 w-8 flex items-center justify-center rounded-md hover:bg-red-500 hover:text-white transition-colors group"
    onclick={close}
    title="Close"
  >
    <IconClose class="size-4 opacity-80 group-hover:opacity-100" />
  </button>
</div>

<style>
    /* Prevent dragging on these buttons since they sit in a drag region */
    .no-drag {
        -webkit-app-region: no-drag;
    }
</style>
