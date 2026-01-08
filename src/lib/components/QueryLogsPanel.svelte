<script lang="ts">
    import { logsStore, type LogEntry } from "$lib/stores/logs.svelte";
    import { cn } from "$lib/utils";
    import IconClearAll from "@tabler/icons-svelte/icons/clear-all";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconAlertCircle from "@tabler/icons-svelte/icons/alert-circle";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";

    function formatTime(ts: number) {
        return new Date(ts).toLocaleTimeString(undefined, {
            hour12: false,
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    }

    let expandedId = $state<number | undefined>(undefined);

    function toggleExpand(id: number | undefined) {
        if (!id) return;
        expandedId = expandedId === id ? undefined : id;
    }

    let scrollContainer: HTMLDivElement;

    $effect(() => {
        // Dependency tracking
        logsStore.logs.length;
        // Scroll to bottom when logs update
        if (scrollContainer) {
            scrollContainer.scrollTop = scrollContainer.scrollHeight;
        }
    });
</script>

<div class="flex h-full flex-col bg-muted/10 border-l border-border">
    <div
        class="flex h-9 flex-none items-center justify-between border-b border-border px-3 bg-muted/20"
    >
        <h2
            class="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
        >
            Query History
        </h2>
        <div class="flex items-center gap-1">
            <button
                class="rounded-sm p-1.5 hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                onclick={() => (logsStore.logs = [])}
                title="Clear Logs"
            >
                <IconClearAll class="size-3.5" />
            </button>
            <button
                class="rounded-sm p-1.5 hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                onclick={() => (logsStore.isOpen = false)}
                title="Close"
            >
                <IconX class="size-3.5" />
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
                        <span class="text-muted-foreground opacity-40">›</span>
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

                            <!-- Rows info removed as requested -->
                        </div>
                    </div>
                {/if}
            </div>
        {/each}
    </div>
</div>
