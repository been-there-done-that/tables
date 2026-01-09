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
    } from "@tabler/icons-svelte";
    import * as Popover from "$lib/components/ui/popover";
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
        whereClause = "",
        orderByClause = "",
        onWhereChange,
        onOrderByChange,
    }: Props = $props();

    let exportOpen = $state(false);
    let localWhere = $state(whereClause);
    let localOrderBy = $state(orderByClause);

    // Sync external props to local state
    $effect(() => {
        localWhere = whereClause;
    });
    $effect(() => {
        localOrderBy = orderByClause;
    });

    // Pagination calculations
    const startRow = $derived(currentOffset + 1);
    const endRow = $derived(Math.min(currentOffset + pageSize, totalRows));
    const hasPrev = $derived(currentOffset > 0);
    const hasNext = $derived(currentOffset + pageSize < totalRows);

    function handleExecute() {
        if (onWhereChange) onWhereChange(localWhere);
        if (onOrderByChange) onOrderByChange(localOrderBy);
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

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            handleExecute();
        }
    }
</script>

<div class="flex items-center gap-2 px-2 h-8 border-b bg-muted/40 text-xs">
    <!-- Execute Button -->
    <Button
        variant="ghost"
        size="icon"
        class="h-6 w-6 text-green-500 hover:text-green-600 hover:bg-green-500/10"
        title="Execute (⌘+Enter)"
        onclick={handleExecute}
    >
        <IconPlayerPlay class="h-4 w-4" />
    </Button>

    <div class="w-px h-4 bg-border"></div>

    <!-- Pagination -->
    <div class="flex items-center gap-1">
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Previous page"
            disabled={!hasPrev}
            onclick={handlePrev}
        >
            <IconChevronLeft class="h-4 w-4" />
        </Button>
        <span
            class="text-muted-foreground min-w-[80px] text-center tabular-nums"
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
            <IconChevronRight class="h-4 w-4" />
        </Button>
    </div>

    <div class="w-px h-4 bg-border"></div>

    <!-- Refresh -->
    <Button
        variant="ghost"
        size="icon"
        class="h-6 w-6"
        title="Refresh"
        onclick={handleRefresh}
    >
        <IconRefresh class="h-4 w-4" />
    </Button>

    <!-- DDL -->
    <Button
        variant="ghost"
        size="icon"
        class="h-6 w-6"
        title="Show DDL"
        onclick={() => onShowDdl?.()}
    >
        <IconDatabase class="h-4 w-4" />
    </Button>

    <div class="w-px h-4 bg-border"></div>

    <!-- WHERE Input -->
    <div class="flex items-center gap-1.5">
        <span class="text-muted-foreground font-medium">WHERE</span>
        <input
            type="text"
            class="h-6 px-2 rounded border bg-background text-xs min-w-[120px] max-w-[200px] focus:outline-none focus:ring-1 focus:ring-primary"
            placeholder="column = value"
            bind:value={localWhere}
            onkeydown={handleKeyDown}
        />
    </div>

    <!-- ORDER BY Input -->
    <div class="flex items-center gap-1.5">
        <span class="text-muted-foreground font-medium">ORDER BY</span>
        <input
            type="text"
            class="h-6 px-2 rounded border bg-background text-xs min-w-[100px] max-w-[160px] focus:outline-none focus:ring-1 focus:ring-primary"
            placeholder="column ASC"
            bind:value={localOrderBy}
            onkeydown={handleKeyDown}
        />
    </div>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Export Popover -->
    <Popover.Root bind:open={exportOpen}>
        <Popover.Trigger>
            <Button variant="ghost" size="icon" class="h-6 w-6" title="Export">
                <IconDownload class="h-4 w-4" />
            </Button>
        </Popover.Trigger>
        <Popover.Content class="w-40 p-1" align="end">
            <div class="text-xs font-medium text-muted-foreground px-2 py-1">
                Export As
            </div>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2"
                onclick={() => handleExport("csv")}
            >
                <span class="text-muted-foreground">📋</span> CSV
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2"
                onclick={() => handleExport("tsv")}
            >
                <span class="text-muted-foreground">📋</span> TSV
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2"
                onclick={() => handleExport("json")}
            >
                <span class="text-muted-foreground">📋</span> JSON
            </button>
            <button
                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted flex items-center gap-2"
                onclick={() => handleExport("sql")}
            >
                <span class="text-muted-foreground">📋</span> SQL INSERT
            </button>
        </Popover.Content>
    </Popover.Root>
</div>
