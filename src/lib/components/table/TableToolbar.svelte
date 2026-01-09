<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { Button } from "$lib/components/ui/button";
    import {
        IconPlayerPlay,
        IconChevronLeft,
        IconChevronRight,
        IconRefresh,
        IconDatabase,
        IconDownload,
        IconGripVertical,
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
    let filterWidth = $state(500);
    let isResizing = $state(false);
    let resizeStartX = $state(0);
    let resizeStartWidth = $state(0);

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

    function startResize(e: MouseEvent) {
        isResizing = true;
        resizeStartX = e.clientX;
        resizeStartWidth = filterWidth;
        document.addEventListener("mousemove", handleResize);
        document.addEventListener("mouseup", stopResize);
    }

    function handleResize(e: MouseEvent) {
        if (!isResizing) return;
        const delta = e.clientX - resizeStartX;
        filterWidth = Math.max(300, Math.min(800, resizeStartWidth + delta));
    }

    function stopResize() {
        isResizing = false;
        document.removeEventListener("mousemove", handleResize);
        document.removeEventListener("mouseup", stopResize);
    }
</script>

<div class="flex items-center gap-1 px-2 h-9 border-b bg-muted/30 text-xs">
    <!-- Execute Button -->
    <Button
        variant="ghost"
        size="icon"
        class="h-7 w-7 text-green-500 hover:text-green-600 hover:bg-green-500/10"
        title="Execute (⌘+Enter)"
        onclick={handleExecute}
    >
        <IconPlayerPlay class="h-4 w-4" />
    </Button>

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
            <IconChevronLeft class="h-3.5 w-3.5" />
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
            <IconChevronRight class="h-3.5 w-3.5" />
        </Button>
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Refresh & DDL -->
    <div class="flex items-center">
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Refresh"
            onclick={handleRefresh}
        >
            <IconRefresh class="h-3.5 w-3.5" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Show DDL"
            onclick={() => onShowDdl?.()}
        >
            <IconDatabase class="h-3.5 w-3.5" />
        </Button>
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Filter Section (Resizable) -->
    <div
        class="flex items-center gap-3 px-3 py-1 bg-background/50 rounded-md border border-border/50"
        style="width: {filterWidth}px; min-width: 300px;"
    >
        <!-- WHERE (60%) -->
        <AutocompleteInput
            bind:value={whereClause}
            placeholder="column = value"
            suggestions={whereSuggestions}
            icon="filter"
            widthClass="w-[60%]"
            onchange={(v) => onWhereChange?.(v)}
            onsubmit={handleExecute}
        />

        <div class="w-px h-4 bg-border/30"></div>

        <!-- ORDER BY (40%) -->
        <AutocompleteInput
            bind:value={orderByClause}
            placeholder="column ASC"
            suggestions={orderBySuggestions}
            icon="sort"
            widthClass="w-[40%]"
            onchange={(v) => onOrderByChange?.(v)}
            onsubmit={handleExecute}
        />

        <!-- Resize Handle -->
        <div
            class="flex items-center justify-center cursor-ew-resize text-muted-foreground/50 hover:text-muted-foreground -mr-1"
            onmousedown={startResize}
            role="separator"
            aria-orientation="vertical"
        >
            <IconGripVertical class="h-4 w-4" />
        </div>
    </div>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Export Popover -->
    <Popover.Root bind:open={exportOpen}>
        <Popover.Trigger>
            <Button variant="ghost" size="icon" class="h-7 w-7" title="Export">
                <IconDownload class="h-4 w-4" />
            </Button>
        </Popover.Trigger>
        <Popover.Content class="w-36 p-1" align="end">
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
