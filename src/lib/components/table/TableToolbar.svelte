<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { onDestroy } from "svelte";
    import { Button } from "$lib/components/ui/button";
    import { cn } from "$lib/utils";
    import {
        IconPlayerPlayFilled,
        IconChevronLeft,
        IconChevronRight,
        IconRefresh,
        IconDownload,
        IconPlayerStopFilled,
        IconPlayerSkipBack,
        IconPlayerSkipForward,
        IconChevronDown,
        IconCheck,
        IconStopwatch,
    } from "@tabler/icons-svelte";
    import * as Menu from "$lib/components/ui/dropdown-menu";
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
        onPageSizeChange?: (size: number) => void;
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
        isLoading?: boolean;
    }

    const dispatch = createEventDispatcher();

    let {
        tableRef = $bindable(),
        onExecute,
        onRefresh,
        onExport,
        onShowDdl,
        onPageChange,
        onPageSizeChange,
        columns = [],
        currentOffset = 0,
        totalRows = 0,
        pageSize = 500,
        whereClause = $bindable(""),
        orderByClause = $bindable(""),
        onWhereChange,
        onOrderByChange,
        isLoading = false,
        executionTime = 0,
    }: Props & { executionTime?: number } = $props();

    let exportOpen = $state(false);
    let pageSizeOpen = $state(false);

    // Column names for autocomplete with types
    const columnSuggestions = $derived(
        columns.map((c) => ({
            value: c.label || c.id,
            type: c.rawType || c.type || "unknown",
        })),
    );

    // Add common SQL operators and keywords to suggestions
    const whereSuggestions = $derived([
        ...columnSuggestions,
        { value: "AND", type: "keyword" },
        { value: "OR", type: "keyword" },
        { value: "NOT", type: "keyword" },
        { value: "IS NULL", type: "keyword" },
        { value: "IS NOT NULL", type: "keyword" },
        { value: "LIKE", type: "keyword" },
        { value: "IN", type: "keyword" },
        { value: "BETWEEN", type: "keyword" },
    ]);

    const orderBySuggestions = $derived([
        ...columnSuggestions,
        { value: "ASC", type: "keyword" },
        { value: "DESC", type: "keyword" },
        { value: "NULLS FIRST", type: "keyword" },
        { value: "NULLS LAST", type: "keyword" },
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

    function handleFirst() {
        if (onPageChange) onPageChange(0);
    }

    function handleLast() {
        if (onPageChange && totalRows > 0) {
            const lastOffset =
                Math.floor((totalRows - 1) / pageSize) * pageSize;
            onPageChange(lastOffset);
        }
    }

    function handlePageSize(size: number) {
        if (onPageSizeChange) {
            onPageSizeChange(size);
            pageSizeOpen = false;
        }
    }

    function handleExport(format: ExportFormat) {
        if (onExport) onExport(format);
        dispatch("export", { format });
        exportOpen = false;
    }
    function onKeyDown(e: KeyboardEvent) {
        if ((e.metaKey || e.ctrlKey) && e.key === "r") {
            e.preventDefault();
            handleRefresh();
        }
    }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="flex items-center gap-2 px-2 h-8 text-xs w-full">
    <!-- Pagination -->
    <!-- Pagination -->
    <div class="flex items-center gap-1">
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="First page"
            disabled={!hasPrev}
            onclick={handleFirst}
        >
            <IconPlayerSkipBack class="size-3.5" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Previous page"
            disabled={!hasPrev}
            onclick={handlePrev}
        >
            <IconChevronLeft class="size-3.5" />
        </Button>

        <Menu.Root bind:open={pageSizeOpen}>
            <Menu.Trigger>
                <Button
                    variant="ghost"
                    size="sm"
                    class="h-6 px-1 gap-1 text-[11px] font-normal min-w-[60px]"
                >
                    {#if totalRows > 0}
                        {startRow}-{endRow}
                    {:else}
                        0-0
                    {/if}
                    <IconChevronDown class="size-3 opacity-50" />
                </Button>
            </Menu.Trigger>
            <Menu.Content
                class="w-20 p-0 border border-(--theme-border-default) bg-(--theme-bg-secondary) shadow-lg"
                align="start"
            >
                <div
                    class="px-2 py-1.5 text-[10px] uppercase font-bold text-muted-foreground border-b border-border/50 bg-muted/30"
                >
                    Page Size
                </div>
                <div class="p-1 flex flex-col gap-0.5">
                    {#each [5, 10, 20, 100, 500, 1000] as size}
                        <Menu.Item
                            class={cn(
                                "w-full text-left px-1 py-1.5 text-xs rounded flex items-center gap-2 hover:bg-muted/80 cursor-pointer",
                            )}
                            onclick={() => handlePageSize(size)}
                        >
                            {#if pageSize === size}
                                <IconCheck class="size-3 pointer-events-none" />
                            {:else}
                                <div class="size-3"></div>
                            {/if}
                            <span class="font-mono tabular-nums"
                                >{size.toLocaleString()}</span
                            >
                        </Menu.Item>
                    {/each}
                    <!-- All option (simulated) -->
                    <Menu.Item
                        class="w-full text-left px-2 py-1.5 text-xs rounded flex items-center gap-2 hover:bg-muted/80 cursor-pointer"
                    >
                        <div class="size-3"></div>
                        <span>All</span>
                    </Menu.Item>
                </div>
            </Menu.Content>
        </Menu.Root>

        <span class="text-muted-foreground text-[11px] tabular-nums px-1">
            of {totalRows ? totalRows.toLocaleString() : "0"}
        </span>

        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Next page"
            disabled={!hasNext}
            onclick={handleNext}
        >
            <IconChevronRight class="size-3.5" />
        </Button>
        <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6"
            title="Last page"
            disabled={!hasNext}
            onclick={handleLast}
        >
            <IconPlayerSkipForward class="size-3.5" />
        </Button>
    </div>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Execute Button -->
    <div class="flex items-center">
        <Button
            variant="ghost"
            size="sm"
            class="h-7"
            title="Run (⌘+Enter)"
            onclick={handleExecute}
        >
            <IconPlayerPlayFilled class="size-5 text-green-500" />
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-7"
            title="Run (⌘+Enter)"
            onclick={handleExecute}
        >
            <IconPlayerStopFilled class="size-5 text-red-500" />
        </Button>

        <Button
            variant="ghost"
            size="sm"
            class="h-7"
            title="Refresh"
            onclick={handleRefresh}
            disabled={isLoading}
        >
            <IconRefresh
                class="size-5 {isLoading ? 'animate-spin-reverse' : ''}"
            />
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

        <div class="w-px h-5 bg-border/50"></div>

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

    <!-- Export Menu -->
    <Menu.Root bind:open={exportOpen}>
        <Menu.Trigger>
            <Button variant="ghost" size="icon" class="h-7 w-7" title="Export">
                <IconDownload class="h-4 w-4 opacity-70" />
            </Button>
        </Menu.Trigger>
        <Menu.Content
            class="w-40 border border-(--theme-border-default) bg-(--theme-bg-secondary) shadow-lg"
            align="end"
        >
            <div
                class="px-2 py-1.5 text-[10px] uppercase font-bold text-muted-foreground border-b border-border/50 bg-muted/30"
            >
                Data Format
            </div>
            <div class="p-1 flex flex-col gap-0.5">
                <Menu.Item
                    class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted cursor-pointer"
                    onclick={() => handleExport("csv")}
                >
                    CSV (.csv)
                </Menu.Item>
                <Menu.Item
                    class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted cursor-pointer"
                    onclick={() => handleExport("tsv")}
                >
                    TSV (.tsv)
                </Menu.Item>
                <Menu.Item
                    class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted cursor-pointer"
                    onclick={() => handleExport("json")}
                >
                    JSON (.json)
                </Menu.Item>
                <Menu.Separator class="my-1" />
                <Menu.Item
                    class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted cursor-pointer"
                    onclick={() => handleExport("sql")}
                >
                    SQL Insert
                </Menu.Item>
            </div>
        </Menu.Content>
    </Menu.Root>

    <div class="w-px h-5 bg-border/50"></div>

    <!-- Execution Time -->
    <span
        class="text-[10px] text-muted-foreground mr-2 font-mono tabular-nums flex justify-between items-center gap-1"
    >
        <IconStopwatch class="size-4 opacity-70" />
        <span>
            {executionTime < 1000
                ? `${executionTime.toFixed(0)}ms`
                : `${(executionTime / 1000).toFixed(2)}s`}
        </span>
    </span>
</div>
