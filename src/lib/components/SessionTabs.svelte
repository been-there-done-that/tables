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
    class="flex items-center gap-1 border-b border-border bg-muted/30 h-8 px-2 py-1 overflow-x-auto"
>
    {#each sessions as session (session.id)}
        <button
            class={cn(
                "flex items-center gap-1 px-2 py-0.5 rounded-md text-sm transition-colors whitespace-nowrap",
                session.id === activeSessionId
                    ? "bg-background text-foreground shadow-sm border border-border"
                    : "text-muted-foreground hover:bg-background/50 hover:text-foreground",
            )}
            onclick={() => handleTabClick(session.id)}
        >
            <span class="truncate max-w-[150px]">
                {session.connection?.name || "Untitled"}
            </span>
            <span
                class="p-0.5 rounded hover:bg-muted"
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
        <span class="text-xs text-muted-foreground px-2"
            >No active sessions</span
        >
    {/if}
</div>
