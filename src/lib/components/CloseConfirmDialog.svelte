<script lang="ts">
    import { onMount } from "svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";

    let showDialog = $state(false);

    // const isMac = navigator.platform.toUpperCase().includes("MAC");
    // const closeShortcut = isMac ? "⌘W" : "Ctrl+W";

    const appWindow = getCurrentWindow();
    const isExcluded = ["datasource-window", "appearance-window"].includes(
        appWindow.label,
    );

    onMount(() => {
        if (isExcluded) return;

        const unlisten = appWindow.onCloseRequested((event) => {
            event.preventDefault();
            showDialog = true;
        });

        return () => {
            unlisten.then((u) => u());
        };
    });

    const handleKeydown = (e: KeyboardEvent) => {
        if (!showDialog) return;

        if (e.key === "Escape") {
            e.preventDefault();
            showDialog = false;
        }

        if (e.key === "Enter") {
            e.preventDefault();
            handleConfirm();
        }
    };

    import { fade, scale } from "svelte/transition";

    const handleConfirm = async () => {
        showDialog = false;
        try {
            await appWindow.destroy();
        } catch (e) {
            console.error("Failed to destroy window:", e);
            // Fallback to close if destroy fails (though permissions should be fixed)
            await appWindow.close();
        }
    };
</script>

<svelte:window onkeydown={handleKeydown} />

{#if showDialog}
    <div
        class="fixed inset-0 z-9999 flex items-center justify-center bg-black/40 backdrop-blur-xs"
        role="dialog"
        aria-modal="true"
        onmousedown={() => (showDialog = false)}
        transition:fade={{ duration: 150 }}
    >
        <div
            class="w-[300px] overflow-hidden rounded-md
                   bg-(--theme-bg-secondary) border border-(--theme-border-default)
                   shadow-2xl shadow-black/50"
            onmousedown={(e) => e.stopPropagation()}
            transition:scale={{ duration: 150, start: 0.98 }}
        >
            <div class="px-6 pt-6 pb-2 text-center">
                <h2
                    class="text-[15px] font-medium text-(--theme-fg-primary) tracking-tight"
                >
                    Close window?
                </h2>
            </div>

            <div class="p-4 flex gap-2">
                <button
                    class="flex-1 h-9 flex items-center justify-center gap-2 rounded-lg
                           text-xs font-medium text-(--theme-fg-muted)
                           hover:bg-(--theme-bg-hover) hover:text-(--theme-fg-primary)
                           transition-all active:scale-[0.97]"
                    onclick={() => (showDialog = false)}
                >
                    <span class="text-[0.6rem] opacity-30 font-mono">(esc)</span
                    >
                    Cancel
                </button>

                <button
                    class="flex-1 h-9 flex items-center justify-center gap-2 rounded-lg
                           bg-red-500/10 text-red-500 text-xs font-medium
                           hover:bg-red-500/20
                           transition-all active:scale-[0.97]"
                    onclick={handleConfirm}
                >
                    <span class="text-[0.6rem] opacity-40 font-mono"
                        >(Enter)</span
                    >
                    Close
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    /* Ensure the backdrop is truly on top */
    :global(.z-\[9999\]) {
        z-index: 9999 !important;
    }
</style>
