<script lang="ts">
    import { onDestroy } from "svelte";
    import type { AgentToolCall } from "$lib/stores/agent.svelte";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";

    interface Props {
        toolCall: AgentToolCall;
        onRun?: (sql: string) => void;
    }

    let { toolCall, onRun }: Props = $props();
    let expanded = $state(false);
    let elapsed = $state(0);
    let intervalId: ReturnType<typeof setInterval> | null = null;

    // SQL-producing tools that show the Run button
    const SQL_TOOLS = new Set(["run_query", "sample_table", "count_rows", "explain_query"]);

    $effect(() => {
        if (toolCall.status === "running") {
            elapsed = Date.now() - toolCall.startedAt;
            intervalId = setInterval(() => {
                elapsed = Date.now() - toolCall.startedAt;
            }, 100);
        } else {
            if (intervalId !== null) {
                clearInterval(intervalId);
                intervalId = null;
            }
            elapsed = Date.now() - toolCall.startedAt;
        }
        return () => {
            if (intervalId !== null) {
                clearInterval(intervalId);
                intervalId = null;
            }
        };
    });

    function formatElapsed(ms: number): string {
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(1)}s`;
    }

    function getSql(): string | null {
        if (!SQL_TOOLS.has(toolCall.toolName)) return null;
        const inp = toolCall.input as Record<string, unknown> | null;
        if (!inp) return null;
        if (toolCall.toolName === "run_query" || toolCall.toolName === "explain_query") {
            return (inp.sql as string) ?? null;
        }
        if (toolCall.toolName === "sample_table") {
            const schema = inp.schema ?? "public";
            const n = inp.n ?? 20;
            return `SELECT * FROM "${schema}"."${inp.table}" LIMIT ${n}`;
        }
        if (toolCall.toolName === "count_rows") {
            const schema = inp.schema ?? "public";
            const where = inp.where ? ` WHERE ${inp.where}` : "";
            return `SELECT COUNT(*) FROM "${schema}"."${inp.table}"${where}`;
        }
        return null;
    }

    onDestroy(() => {
        if (intervalId !== null) clearInterval(intervalId);
    });
</script>

<div class="my-1 rounded-lg border border-border bg-muted/10 text-[12px]">
    <!-- Header -->
    <button
        class="flex w-full items-center gap-2 px-3 py-2 text-left"
        onclick={() => (expanded = !expanded)}
    >
        {#if toolCall.status === "running"}
            <IconLoader2 size={12} class="shrink-0 animate-spin text-accent" />
        {:else if toolCall.status === "done"}
            <IconCheck size={12} class="shrink-0 text-green-500" />
        {:else}
            <IconX size={12} class="shrink-0 text-destructive" />
        {/if}

        <span class="flex-1 truncate font-mono text-muted-foreground">{toolCall.toolName}</span>

        <!-- Elapsed time -->
        <span
            class="font-mono text-[10px] {toolCall.status === 'running'
                ? 'text-accent'
                : 'text-muted-foreground/60'}"
        >
            {formatElapsed(elapsed)}
        </span>

        {#if expanded}
            <IconChevronDown size={12} class="shrink-0 text-muted-foreground" />
        {:else}
            <IconChevronRight size={12} class="shrink-0 text-muted-foreground" />
        {/if}
    </button>

    <!-- Expandable output -->
    {#if expanded}
        <div class="border-t border-border px-3 py-2">
            {#if toolCall.input}
                <div class="mb-2">
                    <div class="mb-1 text-[10px] uppercase tracking-wide text-muted-foreground">Input</div>
                    <pre class="overflow-x-auto whitespace-pre-wrap break-all text-[11px] text-foreground/80">{JSON.stringify(toolCall.input, null, 2)}</pre>
                </div>
            {/if}
            {#if toolCall.output}
                <div>
                    <div class="mb-1 flex items-center justify-between">
                        <span class="text-[10px] uppercase tracking-wide text-muted-foreground">Output</span>
                        {#if getSql() && onRun && toolCall.status === "done"}
                            <button
                                onclick={(e) => { e.stopPropagation(); onRun?.(getSql()!); }}
                                class="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-accent hover:bg-accent/10"
                            >
                                <IconPlayerPlay size={9} />
                                Run
                            </button>
                        {/if}
                    </div>
                    <pre
                        class="max-h-48 overflow-y-auto whitespace-pre-wrap break-all text-[11px] {toolCall.status ===
                        'error'
                            ? 'text-destructive'
                            : 'text-foreground/80'}">{toolCall.output}</pre>
                </div>
            {/if}
        </div>
    {/if}
</div>
