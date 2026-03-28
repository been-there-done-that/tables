<script lang="ts">
    import { settingsStore } from "$lib/stores/settings.svelte";
    import { agentStore } from "$lib/stores/agent.svelte";
    import IconSparkles from "@tabler/icons-svelte/icons/sparkles";
    import IconReportAnalytics from "@tabler/icons-svelte/icons/report-analytics";
    import IconClock from "@tabler/icons-svelte/icons/clock";
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import IconInfoCircle from "@tabler/icons-svelte/icons/info-circle";
    import WaterfallChart from "./WaterfallChart.svelte";

    interface PlanNode {
        nodeType: string;
        relationName: string | null;
        indexName: string | null;
        totalMs: number;
        exclusiveMs: number;
        pctOfTotal: number;
        plannedRows: number;
        actualRows: number;
        loops: number;
        buffersHit: number | null;
        buffersRead: number | null;
        depth: number;
        children: PlanNode[];
    }

    interface PlanIssue {
        severity: "danger" | "warning";
        kind: string;
        nodeType: string;
        relation: string | null;
        message: string;
        suggestion: string;
    }

    interface ExplainResult {
        planningMs: number;
        executionMs: number;
        totalRows: number;
        plan: PlanNode;
        issues: PlanIssue[];
    }

    interface Props {
        result: ExplainResult;
        query?: string;
    }

    let { result, query = "" }: Props = $props();

    // Flatten the plan tree depth-first for WaterfallChart
    function flattenPlan(node: PlanNode): PlanNode[] {
        return [node, ...node.children.flatMap(flattenPlan)];
    }

    const flatNodes = $derived(flattenPlan(result.plan));

    function handleAskAi() {
        const slowestNode = [...flatNodes].sort((a, b) => b.pctOfTotal - a.pctOfTotal)[0];
        const issuesSummary = result.issues.map((i: PlanIssue) => i.message).join("; ");

        const message = `I ran EXPLAIN ANALYZE on this query:

\`\`\`sql
${query}
\`\`\`

Execution plan summary:
- Planning time: ${result.planningMs.toFixed(1)}ms, Execution: ${result.executionMs.toFixed(1)}ms
- Slowest node: ${slowestNode?.nodeType ?? "unknown"}${slowestNode?.relationName ? ` on ${slowestNode.relationName}` : ""} (${slowestNode?.pctOfTotal?.toFixed(0) ?? 0}% of time)
${issuesSummary ? `- Issues: ${issuesSummary}` : "- No issues detected"}

Please analyze this execution plan and suggest both query-level optimizations and table-level improvements (indexes, ANALYZE, statistics, etc.).`;

        agentStore.pendingMessage = message;
        settingsStore.sidebarRightVisible = true;
    }

    function formatMs(ms: number): string {
        if (ms < 1) return `${(ms * 1000).toFixed(0)}μs`;
        if (ms < 1000) return `${ms.toFixed(1)}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    }
</script>

<div class="flex flex-col h-full overflow-auto bg-background">
    <div class="flex-1 p-3 space-y-4">

        <!-- Header -->
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
                <IconReportAnalytics class="size-3.5 text-muted-foreground" />
                <span class="text-xs font-semibold text-foreground">Execution Plan</span>

                <div class="flex items-center gap-1.5 text-[10px] text-muted-foreground bg-muted/50 rounded-full px-2 py-0.5 border border-border/50">
                    <IconClock class="size-2.5" />
                    Planning {formatMs(result.planningMs)}
                </div>
                <div class="flex items-center gap-1.5 text-[10px] text-muted-foreground bg-muted/50 rounded-full px-2 py-0.5 border border-border/50">
                    <IconClock class="size-2.5" />
                    Execution {formatMs(result.executionMs)}
                </div>
                <div class="flex items-center gap-1.5 text-[10px] text-muted-foreground bg-muted/50 rounded-full px-2 py-0.5 border border-border/50">
                    {result.totalRows.toLocaleString()} rows
                </div>
            </div>

            <button
                onclick={handleAskAi}
                class="flex items-center gap-1.5 text-[11px] px-2.5 py-1 rounded border border-border bg-muted/30 text-blue-400 hover:bg-blue-500/10 hover:border-blue-500/30 transition-colors"
            >
                <IconSparkles class="size-3" />
                Ask AI
            </button>
        </div>

        <!-- Waterfall -->
        <WaterfallChart nodes={flatNodes} totalMs={result.executionMs} />

        <!-- Issues -->
        {#if result.issues.length > 0}
            <div class="space-y-2">
                <div class="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
                    Issues detected
                </div>
                {#each result.issues as issue}
                    <div class={[
                        "flex items-start gap-2.5 p-2.5 rounded-md border text-xs",
                        issue.severity === "danger"
                            ? "bg-red-500/8 border-red-500/20"
                            : "bg-orange-500/8 border-orange-500/20"
                    ].join(" ")}>
                        {#if issue.severity === "danger"}
                            <IconAlertTriangle class="size-3.5 text-red-400 mt-0.5 flex-shrink-0" />
                        {:else}
                            <IconInfoCircle class="size-3.5 text-orange-400 mt-0.5 flex-shrink-0" />
                        {/if}
                        <div>
                            <div class={issue.severity === "danger" ? "text-red-400 font-medium mb-0.5" : "text-orange-400 font-medium mb-0.5"}>
                                {issue.message}
                            </div>
                            <div class="text-muted-foreground leading-relaxed">{issue.suggestion}</div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}

    </div>
</div>
