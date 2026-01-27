<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import { cn } from "$lib/utils";
    import SqlResultPanel from "./SqlResultPanel.svelte";

    const activeSession = $derived(windowState.activeSession);
    // Only show tabs for SQL editors in the bottom panel
    const editorViews = $derived(
        activeSession?.views.filter((v) => v.type === "editor") || [],
    );
    const activeViewId = $derived(activeSession?.activeViewId);
    const isActiveAnEditor = $derived(
        editorViews.some((v) => v.id === activeViewId),
    );

    function handleTabClick(viewId: string) {
        activeSession?.activateView(viewId);
    }

    function handleCloseClick(e: MouseEvent, viewId: string) {
        e.stopPropagation();
        activeSession?.closeView(viewId);
    }
</script>

<div class="flex h-full flex-col bg-background border-t border-border">
    <!-- Bottom Tabs (Only for SQL Editors) -->
    {#if editorViews.length > 0}
        <div
            class="flex items-center gap-px border-b border-border bg-muted/20 h-8 px-2 overflow-x-auto scrollbar-hide"
        >
            {#each editorViews as view (view.id)}
                <button
                    class={cn(
                        "flex items-center gap-2 px-3 h-full text-[10px] uppercase tracking-wider font-semibold whitespace-nowrap transition-all duration-150 select-none",
                        "border-r border-border/50 relative group outline-none focus:outline-none",
                        view.id === activeViewId
                            ? "bg-background text-foreground"
                            : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
                    )}
                    onclick={() => handleTabClick(view.id)}
                >
                    {#if view.type === "editor"}
                        <IconFileText class="size-3 opacity-60" />
                    {/if}

                    <span class="truncate max-w-[120px]">
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
                        <IconX class="size-2.5" />
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

    <!-- Content Area -->
    <div class="flex-1 overflow-hidden relative">
        {#each editorViews as view (view.id)}
            {#if view.id === activeViewId}
                <div class="absolute inset-0">
                    {#if view.type === "editor"}
                        <SqlResultPanel {view} />
                    {/if}
                </div>
            {/if}
        {/each}

        {#if !isActiveAnEditor && editorViews.length > 0}
            <div
                class="h-full flex items-center justify-center text-muted-foreground text-sm italic"
            >
                Select an editor tab to view its results.
            </div>
        {/if}

        {#if editorViews.length === 0}
            <div
                class="h-full flex items-center justify-center text-muted-foreground text-sm italic"
            >
                Open a query editor to see results here.
            </div>
        {/if}
    </div>
</div>

<style>
    .scrollbar-hide::-webkit-scrollbar {
        display: none;
    }
    .scrollbar-hide {
        -ms-overflow-style: none;
        scrollbar-width: none;
    }
</style>
