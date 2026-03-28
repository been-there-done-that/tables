<script lang="ts">
    import { onDestroy } from "svelte";
    import type { AgentToolCall } from "$lib/stores/agent.svelte";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconChevronRight from "@tabler/icons-svelte/icons/chevron-right";
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import IconTool from "@tabler/icons-svelte/icons/tool";
    import IconShieldCheck from "@tabler/icons-svelte/icons/shield-check";
    import IconShieldX from "@tabler/icons-svelte/icons/shield-x";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconFile from "@tabler/icons-svelte/icons/file";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconLink from "@tabler/icons-svelte/icons/link";

    interface Props {
        toolCall: AgentToolCall;
        isFirst?: boolean;
        isLast?: boolean;
        onRun?: (sql: string) => void;
        onFocusFile?: (fileId: string, lineStart?: number, lineEnd?: number) => void;
        onApprove?: (toolId: string) => void;
        onReject?: (toolId: string) => void;
    }

    let { toolCall, isFirst = true, isLast = true, onRun, onFocusFile, onApprove, onReject }: Props = $props();
    let expanded = $state(false);
    let elapsed = $state(0);
    let intervalId: ReturnType<typeof setInterval> | null = null;

    // SQL-producing tools that show the Run button
    const SQL_TOOLS = new Set(["run_query", "sample_table", "count_rows", "explain_query"]);

    // Human-readable verb + target
    const TOOL_VERB: Record<string, string> = {
        describe_table: "Described",
        sample_table: "Sampled",
        list_tables: "Listed tables",
        list_schemas: "Listed schemas",
        list_databases: "Listed databases",
        run_query: "Ran query",
        explain_query: "Explained query",
        write_file: "Wrote",
        read_file: "Read",
        check_fk_integrity: "Checked FK",
        spawn_subagent: "Spawned subagent",
        count_rows: "Counted rows",
    };

    function getVerbAndTarget(): { verb: string; target: string | null } {
        const inp = (toolCall.input as Record<string, unknown> | null) ?? {};
        const verb = TOOL_VERB[toolCall.toolName] ?? toolCall.toolName;
        switch (toolCall.toolName) {
            case "describe_table":
            case "sample_table":
            case "check_fk_integrity":
            case "count_rows":
                return { verb, target: inp.table ? `${inp.schema ? inp.schema + "." : ""}${inp.table}` : null };
            case "list_tables":
                return { verb, target: inp.schema ? String(inp.schema) : null };
            case "write_file":
            case "read_file":
                return { verb, target: (inp.fileName ?? inp.fileId) as string | null };
            case "run_query":
            case "explain_query":
                return { verb, target: null };
            default:
                return { verb, target: null };
        }
    }

    const verbLabel = $derived(getVerbAndTarget().verb);
    const targetLabel = $derived(getVerbAndTarget().target);

    const toolIcon = $derived.by((): typeof IconTool => {
        switch (toolCall.toolName) {
            case "describe_table":
            case "sample_table":
            case "list_tables":
            case "count_rows":
                return IconTable;
            case "list_schemas":
            case "list_databases":
                return IconDatabase;
            case "run_query":
            case "explain_query":
                return IconPlayerPlay;
            case "write_file":
            case "read_file":
                return IconFile;
            case "check_fk_integrity":
                return IconLink;
            default:
                return IconTool;
        }
    });

    const toolIconClass = $derived.by((): string => {
        switch (toolCall.toolName) {
            case "describe_table":
            case "sample_table":
            case "list_tables":
            case "count_rows":
                return "text-blue-400/60";
            case "run_query":
                return "text-accent/60";
            case "write_file":
                return "text-purple-400/60";
            case "read_file":
                return "text-muted-foreground/50";
            default:
                return "text-muted-foreground/40";
        }
    });

    const dotClass = $derived.by((): string => {
        switch (toolCall.status) {
            case "running":
                return "bg-accent animate-pulse";
            case "done":
                return "bg-foreground/40";
            case "error":
                return "bg-destructive/70";
            case "awaiting":
                return "bg-amber-400";
            default:
                return "bg-muted-foreground/30";
        }
    });

    const fileId = $derived.by((): string | undefined => {
        if (toolCall.toolName !== "write_file" && toolCall.toolName !== "read_file") return undefined;
        const inp = toolCall.input as Record<string, unknown>;
        try {
            const out = JSON.parse(toolCall.output ?? "{}");
            return (out.fileId ?? inp.fileId ?? inp.fileName) as string | undefined;
        } catch {
            return (inp.fileId ?? inp.fileName) as string | undefined;
        }
    });

    const displayName = $derived.by((): string => {
        const inp = toolCall.input as Record<string, unknown>;
        try {
            const out = JSON.parse(toolCall.output ?? "{}");
            return (inp.fileName ?? out.fileName ?? fileId ?? "") as string;
        } catch {
            return (inp.fileName ?? "") as string;
        }
    });

    const lineStart = $derived.by(() => (toolCall.input as Record<string, unknown>)?.lineStart as number | undefined);
    const lineEnd = $derived.by(() => (toolCall.input as Record<string, unknown>)?.lineEnd as number | undefined);
    const lineLabel = $derived.by(() =>
        lineStart != null
            ? `:${lineStart}${lineEnd != null && lineEnd !== lineStart ? `-${lineEnd}` : ""}`
            : "",
    );

    function getWriteFileSummary(): { fileName: string; action: string; lines: number } | null {
        if (toolCall.toolName !== "write_file") return null;
        const inp = toolCall.input as Record<string, unknown> | null;
        if (!inp) return null;
        try {
            const out = JSON.parse(toolCall.output ?? "{}");
            return {
                fileName: (inp.fileName as string) ?? "file",
                action: out.action ?? "wrote",
                lines: out.lines ?? 0,
            };
        } catch {
            return { fileName: (inp.fileName as string) ?? "file", action: "wrote", lines: 0 };
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
            // "awaiting" shows no elapsed time — the wait is on the user, not the tool.
            // For completed/failed: use actual wall-clock duration.
            elapsed =
                toolCall.status === "awaiting"
                    ? 0
                    : toolCall.completedAt != null
                      ? toolCall.completedAt - toolCall.startedAt
                      : 0;
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
            } catch {
                /* fall through */
            }
        }
        // count_rows returns a plain number or {count: N}
        if (toolCall.toolName === "count_rows") {
            try {
                const out = JSON.parse(toolCall.output);
                const n = out?.count ?? out?.rows?.[0]?.count ?? null;
                if (n != null) return `${n} rows`;
            } catch {
                /* fall through */
            }
        }
        return null;
    }

    onDestroy(() => {
        if (intervalId !== null) clearInterval(intervalId);
    });
</script>

<div class="group relative flex items-stretch gap-0 text-[11px]">
    <!-- LEFT RAIL: line + dot + line -->
    <div class="flex w-5 shrink-0 flex-col items-center">
        <!-- top line (hidden for first item) -->
        <div class="w-px {isFirst ? 'opacity-0' : 'bg-muted-foreground/25'} h-2 flex-none"></div>
        <!-- status dot -->
        {#if toolCall.status === "running"}
            <IconLoader2 size={11} class="mt-0.5 shrink-0 animate-spin text-accent" />
        {:else}
            <div class="mt-0.5 h-2 w-2 shrink-0 rounded-full {dotClass}"></div>
        {/if}
        <!-- bottom line (hidden for last item, grows to fill row height) -->
        <div class="w-px flex-1 {isLast ? 'opacity-0' : 'bg-muted-foreground/25'} mt-0.5"></div>
    </div>

    <!-- RIGHT CONTENT -->
    <div class="flex min-w-0 flex-1 flex-col pb-1">
        <!-- ROW HEADER -->
        <div class="flex min-h-[20px] w-full items-center gap-1.5">
            <!-- Clickable area: icon + labels + chevron -->
            <button
                onclick={() => (expanded = !expanded)}
                class="flex min-w-0 flex-1 items-center gap-1.5 text-left"
            >
                <!-- Tool icon -->
                {#each [toolIcon] as Icon (toolCall.toolName)}
                    <Icon size={11} class="shrink-0 {toolIconClass}" />
                {/each}

                <!-- Verb (bold) + target -->
                <span class="flex min-w-0 items-baseline gap-1">
                    <span class="font-medium text-foreground/75">{verbLabel}</span>
                    {#if targetLabel}
                        <span class="truncate font-mono text-[10px] text-foreground/45">{targetLabel}</span>
                    {/if}
                </span>
            </button>

            <span class="flex-1"></span>

            <!-- File jump button for write_file/read_file -->
            {#if (toolCall.toolName === "write_file" || toolCall.toolName === "read_file") && fileId && onFocusFile}
                <button
                    onclick={(e) => { e.stopPropagation(); onFocusFile?.(fileId!, lineStart, lineEnd); }}
                    class="shrink-0 font-mono text-[9.5px] text-accent/50 hover:text-accent hover:underline transition-colors"
                    title="Jump to file">{displayName}{lineLabel}</button>
            {/if}

            <!-- Run button for SQL tools -->
            {#if SQL_TOOLS.has(toolCall.toolName) && onRun && getSql()}
                <button
                    onclick={(e) => { e.stopPropagation(); onRun?.(getSql()!); }}
                    class="shrink-0 rounded px-1 py-0.5 text-[9px] text-accent/40 hover:bg-accent/10 hover:text-accent transition-colors"
                >
                    <IconPlayerPlay size={8} />
                </button>
            {/if}

            <!-- Duration (hidden for awaiting) -->
            {#if toolCall.status !== "awaiting"}
                <span
                    class="shrink-0 font-mono text-[9px] {toolCall.status === 'running'
                        ? 'text-accent'
                        : 'text-muted-foreground/35'}"
                >
                    {formatElapsed(elapsed)}
                </span>
            {/if}

            <!-- Expand chevron -->
            <button onclick={() => (expanded = !expanded)} class="shrink-0">
                {#if expanded}
                    <IconChevronDown size={9} class="text-muted-foreground/35" />
                {:else}
                    <IconChevronRight size={9} class="text-muted-foreground/20 group-hover:text-muted-foreground/40" />
                {/if}
            </button>
        </div>

        <!-- APPROVAL PANEL (awaiting, shown immediately) -->
        {#if toolCall.status === "awaiting"}
            <div class="mb-1 rounded border border-amber-400/20 bg-amber-400/5 px-2 py-2">
                {#if toolCall.toolName === "run_query"}
                    {@const sql = (toolCall.input as Record<string, unknown>)?.sql as string | undefined}
                    {#if sql}
                        <pre class="mb-2 max-h-28 overflow-y-auto whitespace-pre-wrap break-all rounded bg-background/60 px-2 py-1.5 text-[10.5px] font-mono text-foreground/80 border border-border/30">{sql}</pre>
                    {/if}
                {/if}
                <div class="flex items-center gap-2">
                    <span class="flex-1 text-[10px] text-amber-400/70">Awaiting approval</span>
                    <button
                        onclick={() => onReject?.(toolCall.id)}
                        class="flex items-center gap-1 rounded px-2 py-1 text-[10px] text-destructive/70 hover:bg-destructive/10 hover:text-destructive transition-colors"
                    >
                        <IconShieldX size={11} /> Reject
                    </button>
                    <button
                        onclick={() => onApprove?.(toolCall.id)}
                        class="flex items-center gap-1 rounded bg-green-500/15 px-2 py-1 text-[10px] text-green-500 hover:bg-green-500/25 transition-colors"
                    >
                        <IconShieldCheck size={11} /> Approve
                    </button>
                </div>
            </div>
        {/if}

        <!-- EXPANDED OUTPUT -->
        {#if expanded}
            <div class="mb-1 rounded border border-border/30 bg-muted/8 px-2 py-1.5">
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
                        <pre class="max-h-24 overflow-y-auto whitespace-pre-wrap break-all text-[10.5px] text-destructive/80">{toolCall.output}</pre>
                    {:else if summary}
                        <div class="flex items-center justify-between gap-2">
                            <span class="text-[10.5px] text-foreground/60">{summary}</span>
                            {#if getSql() && onRun && toolCall.status === "done"}
                                <button
                                    onclick={(e) => { e.stopPropagation(); onRun?.(getSql()!); }}
                                    class="flex shrink-0 items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-accent hover:bg-accent/10"
                                >
                                    <IconPlayerPlay size={9} /> Run
                                </button>
                            {/if}
                        </div>
                    {:else}
                        <pre class="max-h-24 overflow-y-auto whitespace-pre-wrap break-all text-[10.5px] text-foreground/55">{toolCall.output.slice(0, 300)}{toolCall.output.length > 300 ? "…" : ""}</pre>
                    {/if}
                {/if}
            </div>
        {/if}
    </div>
</div>
