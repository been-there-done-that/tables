<script lang="ts">
    import { logsStore, type LogEntry } from "$lib/stores/logs.svelte";
    import { schemaStore } from "$lib/stores/schema.svelte";
    import { cn } from "$lib/utils";
    import XIcon from "@tabler/icons-svelte/icons/x";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";

    function formatTime(ts: number) {
        return new Date(ts).toLocaleTimeString(undefined, {
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    }

    let expandedId = $state<number | null>(null);
    let scrollContainer = $state<HTMLElement | null>(null);

    // Auto-scroll to bottom when logs change
    $effect(() => {
        if (scrollContainer && logsStore.logs.length) {
            scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
    });

    // Refresh logs when active connection changes
    $effect(() => {
        logsStore.init(schemaStore.activeConnection?.id);
    });

    function toggleExpand(id: number) {
        if (expandedId === id) {
            expandedId = null;
        } else {
            expandedId = id;
        }
    }

    function clearLogs() {
        logsStore.logs = [];
    }
</script>

{#if logsStore.isOpen}
    <div
        class="fixed pt-[30px] right-0 top-0 bottom-0 w-[400px] bg-background border-l border-border z-20 flex flex-col shadow-xl"
    >
        <div
            class="h-10 border-b border-border flex items-center justify-between px-3 bg-muted/30"
        >
            <span class="text-xs font-semibold text-muted-foreground"
                >Query History</span
            >
            <div class="flex items-center gap-1">
                <button
                    class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground"
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

            {#each logsStore.logs as log, i (i)}
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
                                <IconLoader2 class="size-3 animate-spin" />
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
                                class="p-1 rounded bg-background/50 border border-border/50 text-xs overflow-x-auto"
                            >
                                <pre
                                    class="whitespace-pre-wrap break-all text-foreground/90">{log.query}</pre>

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
        </div>
    </div>
{/if}
