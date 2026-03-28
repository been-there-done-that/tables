<script lang="ts">
    import IconAlertTriangle from "@tabler/icons-svelte/icons/alert-triangle";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconDatabase from "@tabler/icons-svelte/icons/database";

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
        depth: number;
    }

    let { nodes, totalMs }: { nodes: PlanNode[]; totalMs: number } = $props();

    function barColor(pct: number): string {
        if (pct > 50) return "bg-red-400";
        if (pct > 10) return "bg-orange-400";
        return "bg-green-500";
    }

    function textColor(pct: number): string {
        if (pct > 50) return "text-red-400";
        if (pct > 10) return "text-orange-400";
        return "text-green-500";
    }

    function formatMs(ms: number): string {
        if (ms < 0.1) return "<0.1ms";
        if (ms < 1000) return `${ms.toFixed(1)}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    }

    function hasRowMismatch(node: PlanNode): boolean {
        if (node.plannedRows <= 0 || node.actualRows <= 0) return false;
        const ratio = node.actualRows / node.plannedRows;
        return ratio > 10 || ratio < 0.1;
    }

    function isSlowNode(node: PlanNode): boolean {
        return node.pctOfTotal > 50;
    }
</script>

<div class="space-y-1">
    {#each nodes as node}
        {@const pct = Math.max(node.pctOfTotal, 0)}
        {@const slow = isSlowNode(node)}
        {@const mismatch = hasRowMismatch(node)}

        <!-- Main row -->
        <div
            class="grid items-center gap-2 py-1 px-1 rounded hover:bg-muted/30 transition-colors cursor-default"
            style="grid-template-columns: 200px 1fr 64px 36px"
        >
            <!-- Label -->
            <div class="flex items-center gap-1 min-w-0" style="padding-left: {node.depth * 16}px">
                {#if slow}
                    <IconAlertTriangle class="size-3 text-red-400 flex-shrink-0" />
                {:else if node.nodeType.includes("Index")}
                    <IconCheck class="size-3 text-green-500 flex-shrink-0" />
                {:else}
                    <IconDatabase class="size-3 text-muted-foreground/50 flex-shrink-0" />
                {/if}
                <span class="text-[11px] truncate {textColor(pct)} font-medium">
                    {node.nodeType}{node.relationName ? ` · ${node.relationName}` : ""}
                </span>
            </div>

            <!-- Bar track -->
            <div class="h-3.5 bg-muted/60 rounded-sm overflow-hidden relative">
                <div
                    class="h-full rounded-sm {barColor(pct)} transition-all duration-300"
                    style="width: {Math.max(pct, pct > 0 ? 1 : 0)}%"
                ></div>
            </div>

            <!-- ms -->
            <div class="text-right text-[11px] font-mono font-medium {textColor(pct)}">
                {formatMs(node.exclusiveMs)}
            </div>

            <!-- pct -->
            <div class="text-right text-[10px] font-mono text-muted-foreground">
                {pct.toFixed(0)}%
            </div>
        </div>

        <!-- Hint row for row mismatch -->
        {#if mismatch}
            <div class="grid gap-2 pb-1" style="grid-template-columns: 200px 1fr">
                <div></div>
                <div class="flex items-center gap-1.5 text-[10px] text-muted-foreground" style="padding-left: 4px">
                    <span class="inline-flex items-center gap-1 bg-orange-500/10 border border-orange-500/20 text-orange-400 rounded px-1.5 py-0.5">
                        <svg class="size-2" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M6 2v4M6 8h.01"/>
                        </svg>
                        row estimate off
                    </span>
                    <span>Planned {node.plannedRows.toLocaleString()} · actual {node.actualRows.toLocaleString()}</span>
                </div>
            </div>
        {/if}
    {/each}
</div>
