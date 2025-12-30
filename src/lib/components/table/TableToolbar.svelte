<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { Button } from "$lib/components/ui/button";
    import {
        IconRefresh,
        IconClock,
        IconPlus,
        IconMinus,
        IconEye,
        IconDatabase,
        IconFilter,
        IconSortAscending,
        IconDownload,
        IconCopy,
        IconChartLine,
    } from "@tabler/icons-svelte";
    import type { Column, SortState } from "./types";

    type ExportFormat = "csv" | "tsv" | "json";

    interface Props {
        tableRef?: any;
        onRefresh?: () => void;
        onCopy?: () => void;
        onExport?: (format: ExportFormat) => void;
        onShowDdl?: () => void;
        exportFormats?: ExportFormat[];
        contextLabel?: string; // e.g. table name
        columns?: Column[];
    }

    const dispatch = createEventDispatcher();

    let {
        tableRef,
        onRefresh,
        onCopy,
        onExport,
        onShowDdl,
        exportFormats = ["csv", "tsv", "json"],
        contextLabel = "",
        columns = [],
    }: Props = $props();

    let exportFormat = $state<ExportFormat>("csv");
    let filterOpen = $state(false);
    let orderOpen = $state(false);
    let filterDraft = $state<
        Record<string, { type: "contains"; value: string }>
    >({});
    let sortDraft = $state<SortState[]>([]);

    $effect(() => {
        exportFormat = exportFormats[0] ?? "csv";
    });

    function hydrateDrafts() {
        const state = tableRef?.getState?.();
        filterDraft = { ...(state?.filters ?? {}) };
        sortDraft = [...(state?.sortState ?? [])];
    }

    function handleRefresh() {
        if (onRefresh) onRefresh();
        else tableRef?.refresh?.();
        dispatch("refresh");
    }

    function handleCopy() {
        if (onCopy) onCopy();
        else tableRef?.copySelection?.();
        dispatch("copy");
    }

    function handleExport() {
        if (onExport) onExport(exportFormat);
        dispatch("export", { format: exportFormat });
    }

    function applyFilters() {
        const cleaned = Object.fromEntries(
            Object.entries(filterDraft).filter(
                ([, v]) =>
                    v &&
                    v.value !== undefined &&
                    v.value !== null &&
                    v.value !== "",
            ),
        );
        tableRef?.setFilters?.(cleaned);
        filterOpen = false;
    }

    function applySort() {
        const cleaned = sortDraft.filter((s) => s.columnId && s.direction);
        tableRef?.setSort?.(cleaned);
        orderOpen = false;
    }

    function updateSort(
        idx: number,
        columnId: string,
        direction: "asc" | "desc",
    ) {
        const next = [...sortDraft];
        next[idx] = { columnId, direction };
        sortDraft = next;
    }
</script>

<div
    class="relative flex items-center gap-2 px-2 py-1 border-b bg-muted/40 text-xs"
>
    <div class="flex items-center gap-1">
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Refresh"
            onclick={handleRefresh}
        >
            <IconRefresh class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="History (stub)"
            onclick={() => dispatch("history")}
        >
            <IconClock class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Add (stub)"
            onclick={() => dispatch("add")}
        >
            <IconPlus class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Remove (stub)"
            onclick={() => dispatch("remove")}
        >
            <IconMinus class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Visibility (stub)"
            onclick={() => dispatch("visibility")}
        >
            <IconEye class="h-4 w-4" />
        </Button>
        <div
            class="flex items-center gap-1 px-2 py-1 rounded border bg-background text-[11px] uppercase tracking-wide"
        >
            <span class="text-muted-foreground">Tx:</span>
            <span>Auto</span>
        </div>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Table DDL"
            onclick={() => onShowDdl?.()}
        >
            <IconDatabase class="h-4 w-4" />
        </Button>
    </div>

    <div class="flex items-center gap-2 ml-2">
        <Button
            variant="ghost"
            class="h-7 px-3 rounded-full border border-dashed"
            onclick={() => {
                hydrateDrafts();
                filterOpen = !filterOpen;
                orderOpen = false;
            }}
            title="Edit filters"
        >
            <div class="flex items-center gap-1">
                <IconFilter class="h-4 w-4" />
                <span>WHERE - {filterOpen}</span>
            </div>
        </Button>
        <Button
            variant="ghost"
            class="h-7 px-3 rounded-full border border-dashed"
            onclick={() => {
                hydrateDrafts();
                orderOpen = !orderOpen;
                filterOpen = false;
            }}
            title="Edit sort"
        >
            <div class="flex items-center gap-1">
                <IconSortAscending class="h-4 w-4" />
                <span>ORDER BY</span>
            </div>
        </Button>
    </div>

    <div class="ml-auto flex items-center gap-2">
        {#if contextLabel}
            <span class="text-muted-foreground text-[11px] uppercase"
                >{contextLabel}</span
            >
        {/if}
        <select
            class="border rounded px-1 py-[2px] bg-background text-xs"
            bind:value={exportFormat}
        >
            {#each exportFormats as fmt}
                <option value={fmt}>{fmt.toUpperCase()}</option>
            {/each}
        </select>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Export"
            onclick={handleExport}
        >
            <IconDownload class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Copy selection"
            onclick={handleCopy}
        >
            <IconCopy class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7"
            title="Stats (stub)"
            onclick={() => dispatch("stats")}
        >
            <IconChartLine class="h-4 w-4" />
        </Button>
    </div>
</div>

{#if filterOpen}
    <div
        class="absolute left-2 top-full mt-1 z-20 w-[420px] border rounded bg-popover p-3 shadow-lg space-y-2"
    >
        <div
            class="flex items-center justify-between text-xs font-semibold text-muted-foreground"
        >
            <span>Filter rows</span>
            <div class="flex gap-2">
                <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2"
                    onclick={() => {
                        filterDraft = {};
                        applyFilters();
                    }}
                >
                    Clear
                </Button>
                <Button size="sm" class="h-7 px-2" onclick={applyFilters}
                    >Apply</Button
                >
            </div>
        </div>
        <div class="max-h-64 overflow-auto space-y-2">
            {#if columns.length === 0}
                <div class="text-xs text-muted-foreground">
                    No columns available
                </div>
            {:else}
                {#each columns as col}
                    <div class="flex items-center gap-2">
                        <span class="w-32 truncate text-xs"
                            >{col.label ?? col.id}</span
                        >
                        <input
                            class="flex-1 h-7 px-2 rounded border bg-background text-xs"
                            placeholder="contains..."
                            value={filterDraft[col.id]?.value ?? ""}
                            oninput={(e) => {
                                filterDraft = {
                                    ...filterDraft,
                                    [col.id]: {
                                        type: "contains",
                                        value: (e.target as HTMLInputElement)
                                            .value,
                                    },
                                };
                            }}
                        />
                    </div>
                {/each}
            {/if}
        </div>
    </div>
{/if}

{#if orderOpen}
    <div
        class="absolute left-2 top-full mt-1 z-20 w-[360px] border rounded bg-popover p-3 shadow-lg space-y-3"
    >
        <div
            class="flex items-center justify-between text-xs font-semibold text-muted-foreground"
        >
            <span>Order by</span>
            <div class="flex gap-2">
                <Button
                    size="sm"
                    variant="ghost"
                    class="h-7 px-2"
                    onclick={() => {
                        sortDraft = [];
                        applySort();
                    }}
                >
                    Clear
                </Button>
                <Button size="sm" class="h-7 px-2" onclick={applySort}
                    >Apply</Button
                >
            </div>
        </div>
        <div class="space-y-2">
            {#each [0, 1] as idx}
                <div class="flex items-center gap-2">
                    <select
                        class="flex-1 h-7 px-2 rounded border bg-background text-xs"
                        value={sortDraft[idx]?.columnId ?? ""}
                        oninput={(e) => {
                            const colId = (e.target as HTMLSelectElement).value;
                            updateSort(
                                idx,
                                colId,
                                sortDraft[idx]?.direction ?? "asc",
                            );
                        }}
                    >
                        <option value="">(none)</option>
                        {#each columns as col}
                            <option value={col.id}>{col.label ?? col.id}</option
                            >
                        {/each}
                    </select>
                    <select
                        class="w-24 h-7 px-2 rounded border bg-background text-xs"
                        value={sortDraft[idx]?.direction ?? "asc"}
                        oninput={(e) => {
                            updateSort(
                                idx,
                                sortDraft[idx]?.columnId ?? "",
                                (e.target as HTMLSelectElement).value as
                                    | "asc"
                                    | "desc",
                            );
                        }}
                    >
                        <option value="asc">ASC</option>
                        <option value="desc">DESC</option>
                    </select>
                </div>
            {/each}
        </div>
    </div>
{/if}
