<script lang="ts">
    import { fly } from "svelte/transition";
    import { cn } from "$lib/utils";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconCode from "@tabler/icons-svelte/icons/code";
    import IconEye from "@tabler/icons-svelte/icons/eye";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import type { EditDelta } from "./TableEditManager.svelte";
    import type { Column } from "./types";

    interface Props {
        deltas: EditDelta[];
        tableName: string;
        tableSchema?: string;
        columns: Column[];
        primaryKeyColumns?: string[];
        onClose: () => void;
    }

    let {
        deltas,
        tableName,
        tableSchema,
        columns,
        primaryKeyColumns = [],
        onClose,
    }: Props = $props();

    let activeTab = $state<"visual" | "sql">("visual");

    // Group deltas by rowId
    const groupedDeltas = $derived(() => {
        const groups = new Map<any, EditDelta[]>();
        for (const delta of deltas) {
            if (!groups.has(delta.rowId)) {
                groups.set(delta.rowId, []);
            }
            groups.get(delta.rowId)!.push(delta);
        }
        return groups;
    });

    // Determine operation type for each row
    function getOperationType(rowDeltas: EditDelta[]): "U" | "I" | "D" {
        // For now, assume all are updates. Insert/Delete can be added later.
        return "U";
    }

    // Generate SQL statements
    const generatedSql = $derived(() => {
        const statements: string[] = [];
        const fullTableName = tableSchema
            ? `"${tableSchema}"."${tableName}"`
            : `"${tableName}"`;

        for (const [rowId, rowDeltas] of groupedDeltas()) {
            const setClause = rowDeltas
                .map((d) => {
                    const val = formatSqlValue(d.newValue);
                    return `"${d.columnId}" = ${val}`;
                })
                .join(", ");

            // Build WHERE clause using primary keys if available
            let whereClause = "";
            if (primaryKeyColumns.length > 0) {
                whereClause = primaryKeyColumns
                    .map((pk) => {
                        const delta = rowDeltas.find((d) => d.columnId === pk);
                        const val = delta
                            ? formatSqlValue(delta.oldValue)
                            : `/* unknown ${pk} */`;
                        return `"${pk}" = ${val}`;
                    })
                    .join(" AND ");
            } else {
                // Fallback: use rowId (not ideal)
                whereClause = `/* row ${rowId} - no primary key defined */`;
            }

            statements.push(
                `UPDATE ${fullTableName}\nSET ${setClause}\nWHERE ${whereClause};`,
            );
        }

        return statements.join("\n\n");
    });

    function formatSqlValue(val: any): string {
        if (val === null || val === undefined) return "NULL";
        if (typeof val === "number") return String(val);
        if (typeof val === "boolean") return val ? "TRUE" : "FALSE";
        if (typeof val === "string") return `'${val.replace(/'/g, "''")}'`;
        return `'${JSON.stringify(val).replace(/'/g, "''")}'`;
    }

    function formatDisplayValue(val: any): string {
        if (val === null) return "NULL";
        if (val === undefined) return "undefined";
        if (typeof val === "object") return JSON.stringify(val);
        return String(val);
    }

    async function copyToClipboard() {
        try {
            await navigator.clipboard.writeText(generatedSql());
        } catch (e) {
            console.error("Failed to copy", e);
        }
    }

    function getColumnLabel(colId: string): string {
        const col = columns.find((c) => c.id === colId);
        return col?.label || colId;
    }
</script>

<div
    class="fixed inset-y-0 right-0 w-[380px] bg-surface border-l border-border shadow-2xl flex flex-col z-50"
    transition:fly={{ x: 380, duration: 200 }}
>
    <!-- Header -->
    <div
        class="flex items-center justify-between px-4 py-3 border-b border-border bg-muted/30"
    >
        <h2 class="text-sm font-semibold text-foreground">Pending Changes</h2>
        <button
            type="button"
            class="p-1 rounded hover:bg-muted transition-colors"
            onclick={onClose}
        >
            <IconX class="size-4 text-muted-foreground" />
        </button>
    </div>

    <!-- Tabs -->
    <div class="flex border-b border-border bg-muted/10">
        <button
            type="button"
            class={cn(
                "flex-1 px-4 py-2 text-xs font-medium transition-colors flex items-center justify-center gap-1.5",
                activeTab === "visual"
                    ? "bg-surface border-b-2 border-accent text-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted/30",
            )}
            onclick={() => (activeTab = "visual")}
        >
            <IconEye class="size-3.5" />
            Visual
        </button>
        <button
            type="button"
            class={cn(
                "flex-1 px-4 py-2 text-xs font-medium transition-colors flex items-center justify-center gap-1.5",
                activeTab === "sql"
                    ? "bg-surface border-b-2 border-accent text-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted/30",
            )}
            onclick={() => (activeTab = "sql")}
        >
            <IconCode class="size-3.5" />
            SQL
        </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-auto">
        {#if activeTab === "visual"}
            <div class="p-3 space-y-3">
                {#each [...groupedDeltas()] as [rowId, rowDeltas]}
                    {@const opType = getOperationType(rowDeltas)}
                    <div class="rounded-lg border border-border bg-muted/20">
                        <!-- Row Header -->
                        <div
                            class="flex items-center gap-2 px-3 py-2 border-b border-border/50"
                        >
                            <span
                                class={cn(
                                    "size-5 rounded text-[10px] font-bold flex items-center justify-center",
                                    opType === "U" &&
                                        "bg-amber-500/20 text-amber-500",
                                    opType === "I" &&
                                        "bg-green-500/20 text-green-500",
                                    opType === "D" &&
                                        "bg-red-500/20 text-red-500",
                                )}
                            >
                                {opType}
                            </span>
                            <span class="text-xs text-muted-foreground">
                                {tableSchema
                                    ? `${tableSchema}.`
                                    : ""}{tableName}
                            </span>
                            <span class="text-muted-foreground/50">›</span>
                            <span class="text-xs font-mono text-foreground">
                                row {rowId}
                            </span>
                        </div>

                        <!-- Column Diffs -->
                        <div class="divide-y divide-border/30">
                            {#each rowDeltas as delta}
                                <div class="px-3 py-2 space-y-1">
                                    <div
                                        class="text-[10px] uppercase tracking-wide text-muted-foreground font-semibold"
                                    >
                                        {getColumnLabel(delta.columnId)}
                                    </div>
                                    <div
                                        class="flex items-start gap-2 text-xs font-mono"
                                    >
                                        <span
                                            class="text-red-400 bg-red-500/10 px-1.5 py-0.5 rounded flex-1 break-all"
                                        >
                                            - {formatDisplayValue(
                                                delta.oldValue,
                                            )}
                                        </span>
                                    </div>
                                    <div
                                        class="flex items-start gap-2 text-xs font-mono"
                                    >
                                        <span
                                            class="text-green-400 bg-green-500/10 px-1.5 py-0.5 rounded flex-1 break-all"
                                        >
                                            + {formatDisplayValue(
                                                delta.newValue,
                                            )}
                                        </span>
                                    </div>
                                </div>
                            {/each}
                        </div>
                    </div>
                {/each}

                {#if deltas.length === 0}
                    <div class="text-center text-muted-foreground py-8 text-sm">
                        No pending changes
                    </div>
                {/if}
            </div>
        {:else}
            <!-- SQL Tab -->
            <div class="flex flex-col h-full">
                <div
                    class="flex items-center justify-end px-3 py-2 border-b border-border/50"
                >
                    <button
                        type="button"
                        class="flex items-center gap-1 px-2 py-1 text-xs rounded hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
                        onclick={copyToClipboard}
                    >
                        <IconCopy class="size-3" />
                        Copy
                    </button>
                </div>
                <pre
                    class="flex-1 p-3 text-xs font-mono text-foreground overflow-auto whitespace-pre-wrap">{generatedSql()}</pre>
            </div>
        {/if}
    </div>
</div>
