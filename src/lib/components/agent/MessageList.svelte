<script lang="ts">
    import { agentStore } from "$lib/stores/agent.svelte";
    import MessageBubble from "./MessageBubble.svelte";
    import ToolCallCard from "./ToolCallCard.svelte";
    import ThinkingBlock from "./ThinkingBlock.svelte";
    import IconLoader from "@tabler/icons-svelte/icons/loader-2";

    interface Props {
        onRunQuery?: (sql: string) => void;
    }

    let { onRunQuery }: Props = $props();
    let container: HTMLDivElement;
    let stickToBottom = $state(true);

    // Interleave messages and tool calls by timestamp
    const timeline = $derived.by(() => {
        const msgs = agentStore.messages.map((m) => ({ kind: "message" as const, item: m, ts: m.timestamp }));
        const tools = agentStore.toolCalls.map((t) => ({ kind: "tool" as const, item: t, ts: t.timestamp }));
        return [...msgs, ...tools].sort((a, b) => a.ts - b.ts);
    });

    const showThinking = $derived(
        agentStore.status === "running" &&
            !agentStore.messages.some((m) => m.role === "assistant" && m.streaming && m.content.length > 0),
    );

    function onScroll() {
        if (!container) return;
        const { scrollTop, scrollHeight, clientHeight } = container;
        stickToBottom = scrollHeight - scrollTop - clientHeight < 80;
    }

    $effect(() => {
        // Trigger on any message/tool change
        void agentStore.messages.length;
        void agentStore.toolCalls.length;
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
            {#each timeline as entry (entry.kind + (entry.kind === "message" ? entry.item.id : entry.item.id))}
                {#if entry.kind === "message"}
                    {@const msg = entry.item}
                    {#if msg.thinking}
                        <div class="px-3">
                            <ThinkingBlock content={msg.thinking} streaming={msg.thinkingStreaming} />
                        </div>
                    {/if}
                    <MessageBubble message={msg} />
                {:else}
                    <div class="px-3">
                        <ToolCallCard toolCall={entry.item} onRun={onRunQuery} />
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
