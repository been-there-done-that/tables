<script lang="ts">
    import { logsStore, type LogEntry } from "$lib/stores/logs.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { cn } from "$lib/utils";
    import XIcon from "@tabler/icons-svelte/icons/x";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";
    import IconLoader from "@tabler/icons-svelte/icons/loader";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconEraser from "@tabler/icons-svelte/icons/eraser";
    import IconTrash from "@tabler/icons-svelte/icons/trash";
    import ConfirmationModal from "./ConfirmationModal.svelte";

    function formatTime(ts: number) {
        return new Date(ts).toLocaleTimeString(undefined, {
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    }

    import { untrack } from "svelte";

    let expandedId = $state<number | null>(null);
    let copiedId = $state<number | null>(null);
    let scrollContainer = $state<HTMLElement | null>(null);
    let confirmClearAll = $state(false);
    let isDeleting = $state(false);

    function getDateLabel(timestamp: number) {
        const date = new Date(timestamp);
        const today = new Date();
        const yesterday = new Date();
        yesterday.setDate(today.getDate() - 1);

        if (date.toDateString() === today.toDateString()) {
            return "Today";
        } else if (date.toDateString() === yesterday.toDateString()) {
            return "Yesterday";
        } else {
            return date.toLocaleDateString(undefined, {
                weekday: "long",
                year: "numeric",
                month: "long",
                day: "numeric",
            });
        }
    }

    const groupedLogs = $derived(() => {
        const groups: { label: string; logs: LogEntry[] }[] = [];
        const map = new Map<string, LogEntry[]>();

        for (const log of logsStore.logs) {
            const label = getDateLabel(log.timestamp);
            if (!map.has(label)) {
                map.set(label, []);
                groups.push({ label, logs: map.get(label)! });
            }
            map.get(label)!.push(log);
        }

        return groups;
    });

    // Auto-scroll to bottom when logs change
    $effect(() => {
        if (scrollContainer && logsStore.logs.length) {
            // Use a slight timeout to ensure DOM update
            setTimeout(() => {
                if (scrollContainer) {
                    scrollContainer.scrollTop = scrollContainer.scrollHeight;
                }
            }, 0);
        }
    });

    // Refresh logs when active connection changes
    $effect(() => {
        const connId = schemaStore.activeConnection?.id;
        untrack(() => logsStore.init(connId));
    });

    function toggleExpand(id: number) {
        if (expandedId === id) {
            expandedId = null;
        } else {
            expandedId = id;
        }
    }

    function clearSession() {
        logsStore.clearSession(schemaStore.activeConnection?.id || null);
    }

    async function handleClearAll() {
        const connId = schemaStore.activeConnection?.id || null;
        if (!connId) return;

        isDeleting = true;
        try {
            await logsStore.clearAll(connId);
            confirmClearAll = false;
        } finally {
            isDeleting = false;
        }
    }
</script>

{#if logsStore.isOpen}
    <div class="flex h-full w-full flex-col bg-background">
        <div
            class="h-8 border-b border-border flex items-center justify-between px-3 bg-muted/30"
        >
            <span class="text-xs font-semibold text-muted-foreground"
                >Query History</span
            >
            <div class="flex items-center gap-0.5">
                <button
                    class="h-6 px-1.5 flex items-center justify-center hover:bg-accent rounded text-muted-foreground gap-1 transition-colors"
                    onclick={clearSession}
                    title="Clear Session (UI only)"
                >
                    <IconEraser class="h-3.5 w-3.5" />
                    <span class="text-[9px] font-medium">Session</span>
                </button>
                <div class="w-px h-3 bg-border mx-1 opacity-50"></div>
                <button
                    class="h-6 px-1.5 flex items-center justify-center hover:bg-accent rounded text-muted-foreground gap-1 hover:text-red-400 transition-colors"
                    onclick={() => (confirmClearAll = true)}
                    title="Clear All (Delete from DB)"
                >
                    <IconTrash class="h-3.5 w-3.5" />
                    <span class="text-[9px] font-medium">Clear All</span>
                </button>
                <div class="w-px h-3 bg-border mx-1 opacity-50"></div>
                <button
                    class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
                    onclick={() => logsStore.toggle()}
                >
                    <XIcon class="h-4 w-4" />
                </button>
            </div>
        </div>

        <div
            bind:this={scrollContainer}
            class="flex-1 overflow-auto font-mono text-[10px] leading-relaxed select-text"
        >
            {#if logsStore.logs.length === 0}
                <div
                    class="p-8 text-center text-muted-foreground italic opacity-50"
                >
                    No queries recorded
                </div>
            {/if}

            {#each groupedLogs() as group}
                <div
                    class="sticky top-0 z-10 bg-muted/90 backdrop-blur-sm px-3 py-1.5 border-b border-border/20 text-[9px] font-bold text-muted-foreground uppercase tracking-widest flex items-center gap-2"
                >
                    <div
                        class="size-1 rounded-full bg-accent-primary opacity-50"
                    ></div>
                    {group.label}
                </div>

                {#each group.logs as log, i (log.timestamp + i)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <div
                        class={cn(
                            "group flex flex-col border-b border-border/40 hover:bg-muted/30 cursor-pointer transition-colors",
                            expandedId === log.timestamp && "bg-muted/40",
                        )}
                        onclick={() => toggleExpand(log.timestamp)}
                    >
                        <!-- Concise Row -->
                        <div class="flex items-center gap-2 p-2 min-w-0">
                            <div
                                class={cn(
                                    "transition-colors",
                                    log.status === "error"
                                        ? "text-red-400"
                                        : log.status === "running"
                                          ? "text-blue-400"
                                          : "text-emerald-400/80",
                                )}
                            >
                                {#if log.status === "error"}
                                    <IconAlertCircle class="size-3" />
                                {:else if log.status === "running"}
                                    <IconLoader class="size-3 animate-spin" />
                                {:else}
                                    <IconCheck class="size-3" />
                                {/if}
                            </div>

                            <span
                                class="text-muted-foreground opacity-60 whitespace-nowrap"
                                >{formatTime(log.timestamp)}</span
                            >

                            <div
                                class="flex-1 truncate min-w-0 flex items-center gap-1.5"
                            >
                                <span
                                    class="text-accent-foreground font-semibold opacity-80"
                                    >{log.database}</span
                                >
                                <span class="text-muted-foreground opacity-40"
                                    >›</span
                                >
                                <span
                                    class={cn(
                                        "truncate",
                                        log.status === "error" &&
                                            "text-red-300 line-through opacity-70",
                                    )}
                                >
                                    {log.query.replace(/\s+/g, " ")}
                                </span>
                            </div>

                            <div
                                class="text-right whitespace-nowrap pl-2 text-muted-foreground opacity-50"
                            >
                                {#if log.status === "running"}
                                    ...
                                {:else}
                                    {log.durationMs}ms
                                {/if}
                            </div>
                        </div>

                        <!-- Expanded Details -->
                        {#if expandedId === log.timestamp}
                            <div
                                class="px-2 pb-3 pt-0 animate-in slide-in-from-top-1 duration-200"
                            >
                                <div
                                    class="p-1 rounded bg-background/50 border border-border/50 text-xs overflow-x-auto relative group/code"
                                >
                                    <button
                                        class="absolute bg-background cursor-pointer rounded-md right-1 top-1 p-1 opacity-0 group-hover/code:opacity-100 text-muted-foreground hover:text-foreground transition-opacity"
                                        onclick={(e) => {
                                            e.stopPropagation();
                                            navigator.clipboard.writeText(
                                                log.query,
                                            );
                                            copiedId = log.timestamp;
                                            setTimeout(
                                                () => (copiedId = null),
                                                2000,
                                            );
                                        }}
                                        title="Copy Query"
                                    >
                                        {#if copiedId === log.timestamp}
                                            <IconCheck
                                                class="size-3 text-emerald-500"
                                            />
                                        {:else}
                                            <IconCopy class="size-3" />
                                        {/if}
                                    </button>
                                    <pre
                                        class="whitespace-pre-wrap break-all text-foreground/90 p-1">{log.query}</pre>

                                    {#if log.status === "error"}
                                        <div
                                            class="mt-2 text-red-400 border-t border-red-500/20 pt-2 font-sans"
                                        >
                                            Error: {log.error}
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        {/if}
                    </div>
                {/each}
            {/each}
        </div>
    </div>
{/if}

<ConfirmationModal
    bind:open={confirmClearAll}
    title="Clear Query History"
    message="Are you sure you want to permanently delete all query history for this connection? This cannot be undone."
    confirmText="Delete All"
    variant="danger"
    isLoading={isDeleting}
    onConfirm={handleClearAll}
/>
