<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import { cn } from "$lib/utils";

    const activeSession = $derived(windowState.activeSession);
    const views = $derived(activeSession?.views || []);
    const activeViewId = $derived(activeSession?.activeViewId);

    function handleTabClick(viewId: string) {
        activeSession?.activateView(viewId);
    }

    function handleCloseClick(e: MouseEvent, viewId: string) {
        e.stopPropagation();
        activeSession?.closeView(viewId);
    }
</script>

{#if views.length > 0}
    <div
        class="flex items-center gap-px border-b border-border bg-muted/20 h-9 px-2 overflow-x-auto scrollbar-hide"
    >
        {#each views as view (view.id)}
            <button
                class={cn(
                    "flex items-center gap-2 px-3 h-full text-xs font-medium whitespace-nowrap transition-all duration-150",
                    "border-r border-border/50 relative group",
                    view.id === activeViewId
                        ? "bg-background text-foreground"
                        : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
                )}
                onclick={() => handleTabClick(view.id)}
            >
                {#if view.type === "editor"}
                    <IconFileText class="size-3.5 opacity-60" />
                {:else if view.type === "table"}
                    <IconTable class="size-3.5 opacity-60 text-emerald-500" />
                {/if}

                <span class="truncate max-w-[150px]">
                    {view.title}
                </span>

                <span
                    class={cn(
                        "p-0.5 rounded-sm transition-all duration-150 ml-1 hover:bg-muted-foreground/10",
                        view.id === activeViewId
                            ? "opacity-100"
                            : "opacity-0 group-hover:opacity-100",
                    )}
                    onclick={(e) => handleCloseClick(e, view.id)}
                    role="button"
                    tabindex="0"
                    onkeydown={(e) =>
                        e.key === "Enter" &&
                        handleCloseClick(e as any, view.id)}
                >
                    <IconX class="size-3" />
                </span>

                {#if view.id === activeViewId}
                    <div
                        class="absolute bottom-0 left-0 right-0 h-0.5 bg-primary"
                    ></div>
                {/if}
            </button>
        {/each}
    </div>
{/if}

<style>
    .scrollbar-hide::-webkit-scrollbar {
        display: none;
    }
    .scrollbar-hide {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
