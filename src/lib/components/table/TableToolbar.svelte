<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { Button } from "$lib/components/ui/button";
    import {
        IconPlayerPlay,
        IconChevronLeft,
        IconChevronRight,
        IconRefresh,
        IconDownload,
    } from "@tabler/icons-svelte";
    import * as Popover from "$lib/components/ui/popover";
    import AutocompleteInput from "./AutocompleteInput.svelte";
    import type { Column, SortState } from "./types";

    type ExportFormat = "csv" | "tsv" | "json" | "sql";

    interface Props {
        tableRef?: any;
        onExecute?: () => void;
        onRefresh?: () => void;
        onExport?: (format: ExportFormat) => void;
        onShowDdl?: () => void;
        onPageChange?: (offset: number) => void;
        columns?: Column[];
        // Pagination props
        currentOffset?: number;
        totalRows?: number;
        pageSize?: number;
        // Filter/sort state
        whereClause?: string;
        orderByClause?: string;
        onWhereChange?: (value: string) => void;
        onOrderByChange?: (value: string) => void;
    }

    const dispatch = createEventDispatcher();

    let {
        tableRef = $bindable(),
        onExecute,
        onRefresh,
        onExport,
        onShowDdl,
        onPageChange,
        columns = [],
        currentOffset = 0,
        totalRows = 0,
        pageSize = 500,
        whereClause = $bindable(""),
        orderByClause = $bindable(""),
        onWhereChange,
        onOrderByChange,
    }: Props = $props();

    let exportOpen = $state(false);

    // Column names for autocomplete
    const columnNames = $derived(columns.map((c) => c.label || c.id));

    // Add common SQL operators and keywords to suggestions
    const whereSuggestions = $derived([
        ...columnNames,
        "AND",
        "OR",
        "NOT",
        "IS NULL",
        "IS NOT NULL",
        "LIKE",
        "IN",
        "BETWEEN",
    ]);

    const orderBySuggestions = $derived([
        ...columnNames,
        "ASC",
        "DESC",
        "NULLS FIRST",
        "NULLS LAST",
    ]);

    // Pagination calculations
    const startRow = $derived(currentOffset + 1);
    const endRow = $derived(Math.min(currentOffset + pageSize, totalRows));
    const hasPrev = $derived(currentOffset > 0);
    const hasNext = $derived(currentOffset + pageSize < totalRows);

    // Filter section width (resizable)

    function handleExecute() {
        if (onWhereChange) onWhereChange(whereClause);
        if (onOrderByChange) onOrderByChange(orderByClause);
        if (onExecute) onExecute();
        else tableRef?.refresh?.();
        dispatch("execute");
    }

    function handleRefresh() {
        if (onRefresh) onRefresh();
        else tableRef?.refresh?.();
        dispatch("refresh");
    }

    function handlePrev() {
        if (hasPrev && onPageChange) {
            onPageChange(Math.max(0, currentOffset - pageSize));
        }
    }

    function handleNext() {
        if (hasNext && onPageChange) {
            onPageChange(currentOffset + pageSize);
        }
    }

    function handleExport(format: ExportFormat) {
        if (onExport) onExport(format);
        dispatch("export", { format });
        exportOpen = false;
    }
</script>

<div class="flex items-center gap-2 px-2 h-8 text-xs w-full">
    <!-- Execute Button -->
    <!-- Execute Button -->
    <div class="flex items-center gap-1">
        <Button
            variant="ghost"
            size="icon"
            class="h-7 w-7 text-green-500 hover:text-green-600 hover:bg-green-500/10"
            title="Execute (⌘+Enter)"
            onclick={handleExecute}
        >
            <IconPlayerPlay class="h-4 w-4" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            title="Refresh"
            onclick={handleRefresh}
        >
            <IconRefresh class="size-4" />
        </Button>
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Pagination -->
    <div class="flex items-center">
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Previous page"
            disabled={!hasPrev}
            onclick={handlePrev}
        >
            <IconChevronLeft class="size-4" />
        </Button>
        <span
            class="text-muted-foreground text-[11px] min-w-[70px] text-center tabular-nums"
        >
            {#if totalRows > 0}
                {startRow}-{endRow} of {totalRows}{totalRows >= pageSize
                    ? "+"
                    : ""}
            {:else}
                0 rows
            {/if}
        </span>
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Next page"
            disabled={!hasNext}
            onclick={handleNext}
        >
            <IconChevronRight class="size-4" />
        </Button>
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Filter Section -->
    <div class="flex items-center gap-3 flex-1 px-3 h-full">
        <!-- WHERE (60%) -->
        <AutocompleteInput
            bind:value={whereClause}
            placeholder="column = value"
            suggestions={whereSuggestions}
            icon="filter"
            widthClass="w-[60%] h-full"
            onchange={(v) => onWhereChange?.(v)}
            onsubmit={handleExecute}
        />

        <!-- ORDER BY (40%) -->
        <AutocompleteInput
            bind:value={orderByClause}
            placeholder="column ASC"
            suggestions={orderBySuggestions}
            icon="sort"
            widthClass="w-[40%] h-full"
            onchange={(v) => onOrderByChange?.(v)}
            onsubmit={handleExecute}
        />
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Spacer -->

    <!-- Export Popover -->
    <Popover.Root bind:open={exportOpen}>
        <Popover.Trigger>
            <Button variant="ghost" size="icon" class="h-7 w-7" title="Export">
                <IconDownload class="h-4 w-4" />
            </Button>
        </Popover.Trigger>
        <Popover.Content
            class="w-36 p-1 bg-popover text-popover-foreground"
            align="end"
        >
            <div
                class="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider px-2 py-1.5"
            >
                Copy as
            </div>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2 transition-colors"
                onclick={() => handleExport("csv")}
            >
                CSV
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2 transition-colors"
                onclick={() => handleExport("tsv")}
            >
                TSV
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2 transition-colors"
                onclick={() => handleExport("json")}
            >
                JSON
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2 transition-colors"
                onclick={() => handleExport("sql")}
            >
                SQL INSERT
            </button>
        </Popover.Content>
    </Popover.Root>
</div>
