<script lang="ts">
    import { windowState } from "$lib/stores/window.svelte";
    import IconX from "@tabler/icons-svelte/icons/x";
    import { cn } from "$lib/utils";

    // Reactive reference to sessions and activeSessionId
    const sessions = $derived(windowState.sessions);
    const activeSessionId = $derived(windowState.activeSessionId);

    function handleTabClick(sessionId: string) {
        windowState.activateSession(sessionId);
    }

    function handleCloseClick(e: MouseEvent, sessionId: string) {
        e.stopPropagation();
        windowState.closeSession(sessionId);
    }
</script>

<div
    class="flex items-center gap-1 border-b border-border bg-muted/30 h-8 px-2 overflow-x-auto scrollbar-hide"
>
    {#each sessions as session (session.id)}
        <button
            class={cn(
                "flex items-center gap-1 px-2 py-0.5 rounded-md text-sm whitespace-nowrap",
                "transition-all duration-200 ease-out",
                "border group",
                session.id === activeSessionId
                    ? "bg-background text-foreground shadow-sm border-border scale-100"
                    : "text-muted-foreground border-transparent hover:bg-background/50 hover:text-foreground hover:border-border/50 scale-95 opacity-70 hover:opacity-100",
            )}
            onclick={() => handleTabClick(session.id)}
        >
            <span class="truncate max-w-[120px]">
                {session.connection?.name || "Untitled"}
            </span>
            <span
                class={cn(
                    "p-0.5 rounded transition-all duration-150 flex items-center justify-center",
                    "hover:bg-muted/80",
                    session.id === activeSessionId
                        ? "opacity-100"
                        : "opacity-0 group-hover:opacity-100",
                )}
                onclick={(e) => handleCloseClick(e, session.id)}
                role="button"
                tabindex="0"
                onkeydown={(e) =>
                    e.key === "Enter" && handleCloseClick(e as any, session.id)}
            >
                <IconX class="size-3" />
            </span>
        </button>
    {/each}

    {#if sessions.length === 0}
        <span class="text-xs text-muted-foreground/30 px-2 italic select-none"
            >No active sessions</span
        >
    {/if}
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
