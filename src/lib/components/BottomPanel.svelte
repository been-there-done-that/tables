<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import { cn } from "$lib/utils";
    import SqlResultPanel from "./SqlResultPanel.svelte";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { Button } from "./ui/button";
    import IconBolt from "@tabler/icons-svelte/icons/bolt";
    import ExplainPanel from "./ExplainPanel.svelte";

    const activeSession = $derived(windowState.activeSession);
    // Only show tabs for SQL editors in the bottom panel that are marked visible
    const editorViews = $derived(
        activeSession?.views.filter(
            (v) =>
                v.type === "editor" &&
                v.data?.results?.bottomTabVisible !== false,
        ) || [],
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
        // Don't close the view, just hide the tab from the bottom panel
        const view = activeSession?.views.find((v) => v.id === viewId);
        if (view && view.data?.results) {
            view.data.results.bottomTabVisible = false;
        }
    }

    function handleMinimize() {
        settingsStore.sidebarBottomVisible = false;
    }
</script>

<div class="flex h-full flex-col bg-background border-t border-border">
    <!-- Bottom Tabs (Only for SQL Editors) -->
    <div
        class="flex items-center justify-between border-b border-border bg-muted/20 h-8 px-2 overflow-hidden"
    >
        <div
            class="flex items-center gap-px h-full overflow-x-auto scrollbar-hide"
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

        <Button
            variant="ghost"
            size="icon"
            class="size-6 h-full rounded-none hover:bg-background/80"
            onclick={handleMinimize}
            title="Minimize Panel"
        >
            <IconChevronDown class="size-3.5" />
        </Button>
    </div>

    <!-- Content Area -->
    <div class="flex-1 overflow-hidden relative">
        {#each editorViews as view (view.id)}
            {#if view.id === activeViewId}
                <div class="absolute inset-0 flex flex-col">
                    <!-- Sub-tabs: Results | Explain (only shown when explain result exists) -->
                    {#if view.data?.results?.explainResult}
                        <div class="flex items-center border-b border-border bg-muted/10 h-7 px-2 gap-px flex-shrink-0">
                            <button
                                class={cn(
                                    "flex items-center gap-1.5 px-2.5 h-full text-[10px] font-medium transition-colors",
                                    (view.data.results.activeBottomTab ?? "results") === "results"
                                        ? "text-foreground border-b-2 border-primary"
                                        : "text-muted-foreground hover:text-foreground"
                                )}
                                onclick={() => { view.data.results.activeBottomTab = "results"; }}
                            >
                                Results
                            </button>
                            <button
                                class={cn(
                                    "flex items-center gap-1.5 px-2.5 h-full text-[10px] font-medium transition-colors relative",
                                    (view.data.results.activeBottomTab ?? "results") === "explain"
                                        ? "text-orange-400 border-b-2 border-orange-400"
                                        : "text-muted-foreground hover:text-foreground"
                                )}
                                onclick={() => { view.data.results.activeBottomTab = "explain"; }}
                            >
                                <IconBolt class="size-3" />
                                Explain
                                {#if view.data.results.explainResult?.issues?.length > 0}
                                    <span class="ml-1 bg-red-500 text-white text-[9px] font-bold rounded-full px-1 leading-4">
                                        {view.data.results.explainResult.issues.length}
                                    </span>
                                {/if}
                            </button>
                        </div>
                    {/if}

                    <!-- Panel content -->
                    <div class="flex-1 overflow-hidden">
                        {#if view.type === "editor"}
                            {#if view.data?.results?.explainResult && (view.data.results.activeBottomTab ?? "results") === "explain"}
                                <ExplainPanel
                                    result={view.data.results.explainResult}
                                    query={view.data.results.explainQuery ?? ""}
                                />
                            {:else}
                                <SqlResultPanel {view} />
                            {/if}
                        {/if}
                    </div>
                </div>
            {/if}
        {/each}

        {#if !isActiveAnEditor && editorViews.length > 0}
            <div class="h-full flex items-center justify-center text-muted-foreground text-sm italic">
                Select an editor tab to view its results.
            </div>
        {/if}

        {#if editorViews.length === 0}
            <div class="h-full flex items-center justify-center text-muted-foreground text-sm italic">
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
