<script lang="ts">
    import { onDestroy } from "svelte";
    import type { AgentToolCall } from "$lib/stores/agent.svelte";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import IconTool from "@tabler/icons-svelte/icons/tool";

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

    function getWriteFileSummary(): { fileName: string; action: string; lines: number } | null {
        if (toolCall.toolName !== "write_file") return null;
        const inp = toolCall.input as Record<string, unknown> | null;
        if (!inp) return null;
        try {
            const out = JSON.parse(toolCall.output ?? "{}");
            return {
                fileName: inp.fileName as string ?? "file",
                action: out.action ?? "wrote",
                lines: out.lines ?? 0,
            };
        } catch {
            return { fileName: inp.fileName as string ?? "file", action: "wrote", lines: 0 };
        }
    }

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

    function getOutputSummary(): string | null {
        if (!toolCall.output) return null;
        // SQL data tools — parse JSON and summarise row count
        if (SQL_TOOLS.has(toolCall.toolName)) {
            try {
                const out = JSON.parse(toolCall.output);
                const rows = Array.isArray(out?.rows) ? out.rows.length : null;
                const total = out?.total ?? rows;
                const cols = Array.isArray(out?.columns) ? out.columns.length : null;
                if (total != null && cols != null) return `${total} rows · ${cols} columns`;
                if (total != null) return `${total} rows`;
            } catch { /* fall through */ }
        }
        // count_rows returns a plain number or {count: N}
        if (toolCall.toolName === "count_rows") {
            try {
                const out = JSON.parse(toolCall.output);
                const n = out?.count ?? out?.rows?.[0]?.count ?? null;
                if (n != null) return `${n} rows`;
            } catch { /* fall through */ }
        }
        return null;
    }

    onDestroy(() => {
        if (intervalId !== null) clearInterval(intervalId);
    });
</script>

<div class="my-0.5 rounded border border-border/50 bg-muted/5 text-[11px] {expanded ? 'w-full' : 'w-fit max-w-[340px]'}">
    <!-- Header -->
    <div class="flex min-w-[160px] items-center gap-1.5 px-2 py-1 w-full">
        <button
            class="flex flex-1 min-w-0 items-center gap-1.5 text-left"
            onclick={() => (expanded = !expanded)}
        >
            <IconTool size={10} class="shrink-0 text-muted-foreground/40" />

            {#if toolCall.status === "running"}
                <IconLoader2 size={10} class="shrink-0 animate-spin text-accent" />
            {:else if toolCall.status === "done"}
                <IconCheck size={10} class="shrink-0 text-green-500/80" />
            {:else}
                <IconX size={10} class="shrink-0 text-destructive/80" />
            {/if}

            <span class="flex-1 truncate font-mono text-[10.5px] text-muted-foreground/70">
                {toolCall.toolName}
                {#if toolCall.toolName === "write_file"}
                    {@const inp = toolCall.input as Record<string, unknown>}
                    {#if inp?.fileName}
                        <span class="text-foreground/50"> — {inp.fileName}</span>
                    {/if}
                {/if}
            </span>
        </button>

        {#if SQL_TOOLS.has(toolCall.toolName) && onRun}
            {@const sql = getSql()}
            {#if sql}
                <button
                    onclick={() => onRun?.(sql)}
                    title="Open in editor"
                    class="shrink-0 flex items-center gap-0.5 rounded px-1 py-0.5 text-[9px] text-accent/50 hover:text-accent hover:bg-accent/10 transition-colors"
                >
                    <IconPlayerPlay size={8} />
                </button>
            {/if}
        {/if}

        <!-- Elapsed time -->
        <span
            class="shrink-0 font-mono text-[9px] {toolCall.status === 'running'
                ? 'text-accent'
                : 'text-muted-foreground/40'}"
        >
            {formatElapsed(elapsed)}
        </span>

        <button onclick={() => (expanded = !expanded)} class="shrink-0">
            {#if expanded}
                <IconChevronDown size={10} class="text-muted-foreground/50" />
            {:else}
                <IconChevronRight size={10} class="text-muted-foreground/50" />
            {/if}
        </button>
    </div>

    <!-- Expandable output -->
    {#if expanded}
        <div class="border-t border-border/40 px-2 py-1.5">
            {#if toolCall.toolName === "write_file"}
                {@const wf = getWriteFileSummary()}
                {#if wf}
                    <span class="text-[10.5px] text-foreground/60">
                        {wf.action === "created" ? "Created" : "Updated"}
                        <code class="rounded bg-muted px-1 text-foreground/80">{wf.fileName}</code>
                        {#if wf.lines}· {wf.lines} lines{/if}
                    </span>
                {/if}
            {:else if toolCall.output}
                {@const summary = getOutputSummary()}
                {#if toolCall.status === "error"}
                    <pre class="whitespace-pre-wrap break-all text-[10.5px] text-destructive/80 max-h-24 overflow-y-auto">{toolCall.output}</pre>
                {:else if summary}
                    <div class="flex items-center justify-between gap-2">
                        <span class="text-[10.5px] text-foreground/60">{summary}</span>
                        {#if getSql() && onRun && toolCall.status === "done"}
                            <button
                                onclick={(e) => { e.stopPropagation(); onRun?.(getSql()!); }}
                                class="flex shrink-0 items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-accent hover:bg-accent/10"
                            >
                                <IconPlayerPlay size={9} />Run
                            </button>
                        {/if}
                    </div>
                {:else}
                    <pre class="whitespace-pre-wrap break-all text-[10.5px] text-foreground/60 max-h-24 overflow-y-auto">{toolCall.output.slice(0, 300)}{toolCall.output.length > 300 ? "…" : ""}</pre>
                {/if}
            {/if}
        </div>
    {/if}
</div>
