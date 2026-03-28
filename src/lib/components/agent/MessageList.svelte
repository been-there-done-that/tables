<script lang="ts">
    import { agentStore, type TurnSummary } from "$lib/stores/agent.svelte";
    import MessageBubble from "./MessageBubble.svelte";
    import ToolCallCard from "./ToolCallCard.svelte";
    import ThinkingBlock from "./ThinkingBlock.svelte";
    import IconLoader from "@tabler/icons-svelte/icons/loader-2";
    import IconClock from "@tabler/icons-svelte/icons/clock";

    interface Props {
        onRunQuery?: (sql: string) => void;
    }

    let { onRunQuery }: Props = $props();
    let container: HTMLDivElement;
    let stickToBottom = $state(true);

    function formatDateLabel(ts: number): string {
        const d = new Date(ts);
        const now = new Date();
        if (d.toDateString() === now.toDateString()) return "Today";
        const yesterday = new Date(now);
        yesterday.setDate(yesterday.getDate() - 1);
        if (d.toDateString() === yesterday.toDateString()) return "Yesterday";
        return d.toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" });
    }

    function formatDuration(ms: number): string {
        if (ms < 1000) return `${ms}ms`;
        if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
        const m = Math.floor(ms / 60_000);
        const s = Math.round((ms % 60_000) / 1000);
        return `${m}m ${s}s`;
    }

    function shortModel(model: string): string {
        // "claude-opus-4-5-20251001" → "claude-opus-4-5"
        return model.replace(/-\d{8}$/, "");
    }

    type RawEntry =
        | { kind: "message"; item: (typeof agentStore.messages)[number]; ts: number }
        | { kind: "tool"; item: (typeof agentStore.toolCalls)[number]; ts: number }
        | { kind: "turn-summary"; item: TurnSummary; ts: number };

    type TimelineEntry =
        | { kind: "date"; label: string; ts: number }
        | RawEntry;

    // Interleave messages, tool calls, and turn summaries sorted by timestamp,
    // then inject date separators at day boundaries.
    const timeline = $derived.by((): TimelineEntry[] => {
        const raw: RawEntry[] = [
            ...agentStore.messages
                .filter((m) => m.content.length > 0 || m.thinking)
                .map((m) => ({ kind: "message" as const, item: m, ts: m.timestamp })),
            ...agentStore.toolCalls.map((t) => ({ kind: "tool" as const, item: t, ts: t.timestamp })),
            ...agentStore.turnSummaries.map((s) => ({ kind: "turn-summary" as const, item: s, ts: s.timestamp })),
        ].sort((a, b) => a.ts - b.ts);

        const result: TimelineEntry[] = [];
        let lastDateStr = "";
        for (const entry of raw) {
            const dateStr = new Date(entry.ts).toDateString();
            if (dateStr !== lastDateStr) {
                result.push({ kind: "date", label: formatDateLabel(entry.ts), ts: entry.ts });
                lastDateStr = dateStr;
            }
            result.push(entry);
        }
        return result;
    });

    const showThinking = $derived(
        agentStore.status === "running" &&
            !agentStore.messages.some(
                (m) => m.role === "assistant" && m.streaming && (m.content.length > 0 || m.thinking),
            ),
    );

    function onScroll() {
        if (!container) return;
        const { scrollTop, scrollHeight, clientHeight } = container;
        stickToBottom = scrollHeight - scrollTop - clientHeight < 80;
    }

    $effect(() => {
        void agentStore.messages.length;
        void agentStore.toolCalls.length;
        void agentStore.turnSummaries.length;
        if (stickToBottom && container) {
            container.scrollTop = container.scrollHeight;
        }
    });
</script>

<div
    bind:this={container}
    onscroll={onScroll}
    class="flex flex-1 flex-col overflow-y-auto pb-2"
>
    {#if timeline.length === 0}
        <div class="flex flex-1 flex-col items-center justify-center gap-2 text-center text-muted-foreground px-6">
            <span class="text-[22px] font-light">Ask Claude</span>
            <span class="text-[12px] opacity-60">Schema is loaded — ask questions, write queries, explore your data.</span>
        </div>
    {:else}
        <div class="flex flex-col gap-0.5 py-2">
            {#each timeline as entry (entry.kind + (entry.kind === "date" ? entry.ts : entry.item.id))}
                {#if entry.kind === "date"}
                    <!-- Date separator -->
                    <div class="flex items-center gap-2 px-3 py-1.5">
                        <div class="h-px flex-1 bg-border/40"></div>
                        <span class="text-[10px] font-medium text-muted-foreground/50 select-none">{entry.label}</span>
                        <div class="h-px flex-1 bg-border/40"></div>
                    </div>
                {:else if entry.kind === "message"}
                    {@const msg = entry.item}
                    {#if msg.thinking}
                        <div class="px-3">
                            <ThinkingBlock content={msg.thinking} streaming={msg.thinkingStreaming} />
                        </div>
                    {/if}
                    <MessageBubble message={msg} />
                {:else if entry.kind === "tool"}
                    <div class="px-3">
                        <ToolCallCard toolCall={entry.item} onRun={onRunQuery} />
                    </div>
                {:else if entry.kind === "turn-summary"}
                    {@const s = entry.item}
                    <!-- Turn summary footer -->
                    <div class="flex items-center gap-1.5 px-3 pb-1 pt-0.5 text-[9.5px] text-muted-foreground/35 select-none">
                        <IconClock size={9} />
                        {formatDuration(s.totalMs)}
                        {#if s.model}
                            <span>·</span>
                            {shortModel(s.model)}
                        {/if}
                    </div>
                {/if}
            {/each}

            {#if showThinking}
                <div class="flex items-center gap-2 px-4 py-2 text-[12px] text-muted-foreground">
                    <IconLoader size={13} class="animate-spin" />
                    <span>Thinking…</span>
                </div>
            {/if}
        </div>
    {/if}
</div>
