<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconCopy from "@tabler/icons-svelte/icons/copy"; // For Close Others
    import IconArrowBarToRight from "@tabler/icons-svelte/icons/arrow-bar-to-right";
    import IconArrowBarToLeft from "@tabler/icons-svelte/icons/arrow-bar-to-left";
    import IconTrash from "@tabler/icons-svelte/icons/trash";
    import * as ContextMenu from "$lib/components/ui/context-menu";
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

    let scrollContainer = $state<HTMLElement | null>(null);

    let renamingViewId = $state<string | null>(null);
    let renameValue = $state("");
    let renameInputEl: HTMLInputElement;

    function startRename(viewId: string, currentTitle: string, event: MouseEvent) {
        event.stopPropagation();
        renamingViewId = viewId;
        renameValue = currentTitle;
        setTimeout(() => renameInputEl?.focus(), 0);
    }

    function commitRename() {
        if (!renamingViewId) return;
        const trimmed = renameValue.trim();
        if (trimmed) {
            activeSession?.renameView(renamingViewId, trimmed);
        }
        renamingViewId = null;
    }

    $effect(() => {
        if (!activeViewId || !scrollContainer) return;
        console.log("scrollContainer", scrollContainer);
        // Wait for DOM update (though effects usually run after)
        const activeTab = scrollContainer.querySelector(
            `[data-view-id="${activeViewId}"]`,
        );
        if (activeTab) {
            activeTab.scrollIntoView({
                behavior: "smooth",
                block: "nearest",
                inline: "center",
            });
        }
    });
</script>

{#if views.length > 0}
    <div
        bind:this={scrollContainer}
        class="flex items-center gap-px border-b border-border bg-muted/20 h-8 px-2 overflow-x-auto scrollbar-hide"
    >
        {#each views as view (view.id)}
            <ContextMenu.Root>
                <ContextMenu.Trigger>
                    <button
                        data-view-id={view.id}
                        class={cn(
                            "flex items-center gap-2 px-3 h-full text-xs font-medium whitespace-nowrap transition-all duration-150 select-none",
                            "border-r border-border/50 relative group outline-none focus:outline-none",
                            view.id === activeViewId
                                ? "bg-background text-foreground"
                                : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
                        )}
                        onclick={() => handleTabClick(view.id)}
                        oncontextmenu={() => {
                            handleTabClick(view.id);
                        }}
                    >
                        {#if view.type === "editor"}
                            <IconFileText class="size-3.5 opacity-60" />
                        {:else if view.type === "table"}
                            <IconTable
                                class="size-3.5 opacity-60 text-(--theme-accent-primary)"
                            />
                        {/if}

                        {#if renamingViewId === view.id}
                            <input
                                bind:this={renameInputEl}
                                bind:value={renameValue}
                                class="w-24 bg-transparent text-xs outline-none border-b border-primary"
                                onblur={commitRename}
                                onkeydown={(e) => {
                                    if (e.key === "Enter") { e.preventDefault(); commitRename(); }
                                    if (e.key === "Escape") { e.stopPropagation(); renamingViewId = null; }
                                }}
                                onclick={(e) => e.stopPropagation()}
                            />
                        {:else}
                            <span
                                class="truncate max-w-[150px]"
                                ondblclick={(e) => startRename(view.id, view.title, e)}
                            >
                                {view.title}
                            </span>
                        {/if}

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
                </ContextMenu.Trigger>
                <ContextMenu.Content class="w-56 gap-2">
                    <ContextMenu.Item
                        onclick={() => activeSession?.closeView(view.id)}
                    >
                        <IconX class="mr-2 size-4 text-muted-foreground" />
                        Close
                        <ContextMenu.Shortcut>
                            <span>⌘ + W</span>
                        </ContextMenu.Shortcut>
                    </ContextMenu.Item>
                    <ContextMenu.Item
                        onclick={() => activeSession?.closeOtherViews(view.id)}
                    >
                        <IconCopy class="mr-2 size-4 text-muted-foreground" />
                        Close Others
                    </ContextMenu.Item>
                    <ContextMenu.Item
                        onclick={() => activeSession?.closeViewsToLeft(view.id)}
                    >
                        <IconArrowBarToLeft
                            class="mr-2 size-4 text-muted-foreground"
                        />
                        Close to the Left
                    </ContextMenu.Item>
                    <ContextMenu.Item
                        onclick={() =>
                            activeSession?.closeViewsToRight(view.id)}
                    >
                        <IconArrowBarToRight
                            class="mr-2 size-4 text-muted-foreground"
                        />
                        Close to the Right
                    </ContextMenu.Item>
                    <ContextMenu.Separator />
                    <ContextMenu.Item
                        onclick={() => activeSession?.closeAllViews()}
                        class="text-red-500 focus:text-red-500"
                    >
                        <IconTrash class="mr-2 size-4 text-red-500" />
                        Close All
                    </ContextMenu.Item>
                </ContextMenu.Content>
            </ContextMenu.Root>
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
