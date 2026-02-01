<script lang="ts">
    import { agentStore } from "$lib/agent/agent.svelte";
    import IconTrash from "@tabler/icons-svelte/icons/trash";
    import IconMessage from "@tabler/icons-svelte/icons/message";
    import { fade, slide } from "svelte/transition";

    let { onClose } = $props<{ onClose: () => void }>();

    function formatDate(ts: number) {
        return new Date(ts * 1000).toLocaleDateString(undefined, {
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    async function handleSelect(session: any) {
        await agentStore.selectSession(session);
        onClose();
    }

    async function handleDelete(e: Event, sessionId: string) {
        e.stopPropagation();
        if (confirm("Are you sure you want to delete this chat?")) {
            await agentStore.deleteSession(sessionId);
        }
    }
</script>

<div
    class="absolute top-10 right-2 w-64 max-h-96 bg-popover border border-border rounded-md shadow-md z-50 flex flex-col overflow-hidden"
    transition:fade={{ duration: 150 }}
>
    <div
        class="p-2 border-b border-border bg-muted/50 text-xs font-medium text-muted-foreground flex justify-between items-center"
    >
        <span>History</span>
        <button class="text-xs hover:text-foreground" onclick={onClose}
            >Close</button
        >
    </div>

    <div class="overflow-y-auto flex-1 p-1 space-y-0.5">
        {#if agentStore.sessions.length === 0}
            <div class="text-xs text-muted-foreground p-4 text-center">
                No past sessions
            </div>
        {/if}

        {#each agentStore.sessions as session (session.id)}
            <div
                class="w-full text-left flex items-center gap-2 p-2 hover:bg-accent rounded-sm group relative text-xs transition-colors cursor-pointer select-none"
                class:bg-accent={agentStore.currentSession?.id === session.id}
                onclick={() => handleSelect(session)}
                onkeydown={(e) => e.key === "Enter" && handleSelect(session)}
                role="button"
                tabindex="0"
            >
                <IconMessage class="size-3.5 text-muted-foreground shrink-0" />
                <div class="flex-1 min-w-0">
                    <div
                        class="truncate font-medium {agentStore.currentSession
                            ?.id === session.id
                            ? 'text-foreground'
                            : 'text-muted-foreground group-hover:text-foreground'}"
                    >
                        {session.title || "Untitled Session"}
                    </div>
                    <div class="text-[10px] text-muted-foreground/60 truncate">
                        {formatDate(session.created_at)}
                    </div>
                </div>

                <button
                    class="opacity-0 group-hover:opacity-100 p-1 hover:bg-destructive/10 hover:text-destructive rounded transition-all absolute right-1 top-1/2 -translate-y-1/2"
                    onclick={(e) => handleDelete(e, session.id)}
                    title="Delete session"
                >
                    <IconTrash class="size-3.5" />
                </button>
            </div>
        {/each}
    </div>
</div>
