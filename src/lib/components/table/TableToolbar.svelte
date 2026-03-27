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
        IconClockPlay,
        IconLoader2,
        IconVersions,
        IconDeviceFloppy,
    } from "@tabler/icons-svelte";

    import * as Menu from "$lib/components/ui/dropdown-menu";
    import AutocompleteInput from "./AutocompleteInput.svelte";
    import type { Column } from "./types";

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
        executionTime?: number;
        onCancel?: () => void;
        onCountUpdate?: () => void;
        isCountLoading?: boolean;
        currentBatchSize?: number;
        isExactTotal?: boolean;
        onLastPage?: () => void;
        pendingChangesCount?: number;
        onShowChanges?: () => void;
        onAddRow?: () => void;
        onSaveChanges?: () => Promise<void>;
        isSaving?: boolean;
        hideFilters?: boolean;
        hideExecute?: boolean;
        hidePagination?: boolean;
        extraActions?: import("svelte").Snippet;
        leftActions?: import("svelte").Snippet;
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
        onCancel,
        onCountUpdate,
        isCountLoading = false,
        currentBatchSize = 0,
        isExactTotal = false,
        onLastPage,
        pendingChangesCount = 0,
        onShowChanges,
        onAddRow,
        onSaveChanges,
        isSaving = false,
        hideFilters = false,
        hideExecute = false,
        hidePagination = false,
        extraActions,
        leftActions,
    }: Props = $props();

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
    // If rowCount > 0, start is offset+1. Else 0.
    const startRow = $derived(currentBatchSize > 0 ? currentOffset + 1 : 0);
    // End row is offset + loaded rows logic.
    const endRow = $derived(currentOffset + currentBatchSize);

    // Heuristic for hasNext:
    // If we have an exact total, use standard math (offset + page < total).
    // If NOT exact total, we assume next page exists if current page is FULL (fetched rows == pageSize).
    // Because if fetch limit is 500, and we get 500, there MIGHT be row 501.
    const hasPrev = $derived(currentOffset > 0);
    const hasNext = $derived.by(() => {
        if (isExactTotal) {
            return currentOffset + pageSize < totalRows;
        }
        // Approximate mode: if we filled the page, assume there is more.
        // Unless pageSize is 0 (All), then no next.
        if (pageSize === 0) return false;
        return currentBatchSize >= pageSize;
    });

    // ...

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

    function handleCancel() {
        if (onCancel) onCancel();
        dispatch("cancel");
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
        if (onLastPage) {
            onLastPage();
        } else if (onPageChange && totalRows > 0) {
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
    function handleAutoRefresh(ms: number) {
        if (intervalId) clearInterval(intervalId);
        intervalId = null;
        currentAutoRefresh = ms;

        if (ms > 0) {
            intervalId = setInterval(() => {
                handleRefresh();
            }, ms);
        }
    }

    let intervalId: ReturnType<typeof setInterval> | null = null;
    let currentAutoRefresh = $state(0);

    onDestroy(() => {
        if (intervalId) clearInterval(intervalId);
    });

    const autoRefreshOptions = [
        { label: "Off", value: 0 },
        { label: "5s", value: 5000 },
        { label: "10s", value: 10000 },
        { label: "30s", value: 30000 },
        { label: "1m", value: 60000 },
        { label: "5m", value: 300000 },
    ];

    function onKeyDown(e: KeyboardEvent) {
        if ((e.metaKey || e.ctrlKey) && e.key === "r") {
            e.preventDefault();
            handleRefresh();
        }
    }

    const showPlus = $derived(!isExactTotal && hasNext);
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="flex flex-col border-b border-border/70 relative z-30">
    <!-- Row 1: Pagination, Execute, Export, Timing -->
    <div class="flex items-center gap-2 px-2 h-8 text-xs w-full">
        <!-- Pagination Controls -->
        {#if hidePagination}
            <span class="text-[11px] tabular-nums text-muted-foreground px-1">
                {currentBatchSize} {currentBatchSize === 1 ? "row" : "rows"}
            </span>
        {:else}
        <div class="flex items-center">
            <Button
                variant="ghost"
                size="icon"
                class="h-6 w-6"
                title="First page"
                disabled={!hasPrev}
                onclick={handleFirst}
            >
                <IconPlayerSkipBack class="size-4" stroke={3} />
            </Button>
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

            <Menu.Root bind:open={pageSizeOpen}>
                <Menu.Trigger>
                    <button
                        class="mx-1 h-6 font-normal text-[0.7rem] w-full px-2 hover:bg-muted/60 rounded-md hover:border-border border-0"
                    >
                        <div class="flex justify-between items-center gap-1">
                            <span class="whitespace-nowrap">
                                {#if totalRows > 0}
                                    {startRow}-{endRow}
                                {:else}
                                    0-0
                                {/if}
                            </span>
                            <span>
                                <IconChevronDown class="size-3 opacity-50" />
                            </span>
                        </div>
                    </button>
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
                                    <IconCheck
                                        class="size-3 pointer-events-none"
                                    />
                                {:else}
                                    <div class="size-3"></div>
                                {/if}
                                <span class="font-mono tabular-nums"
                                    >{size.toLocaleString()}</span
                                >
                            </Menu.Item>
                        {/each}
                        <!-- All option -->
                        <Menu.Item
                            class="w-full text-left px-1 py-1.5 text-xs rounded flex items-center gap-2 hover:bg-muted/80 cursor-pointer"
                            onclick={() => handlePageSize(0)}
                        >
                            {#if pageSize === 0}
                                <IconCheck class="size-3 pointer-events-none" />
                            {:else}
                                <div class="size-3"></div>
                            {/if}
                            <span>All</span>
                        </Menu.Item>
                    </div>
                </Menu.Content>
            </Menu.Root>

            <span class="inline-flex items-center text-[11px] tabular-nums">
                <span>of</span>

                <button
                    class="mx-1 h-6 font-normal text-[0.7rem] w-full px-2 hover:bg-muted/60 rounded-md hover:border-border border-0 flex justify-center items-center"
                    title="Click to update (runs SELECT COUNT(*) FROM ...)"
                    onclick={() => onCountUpdate?.()}
                    disabled={isCountLoading}
                >
                    <span class="inline-flex items-center gap-0.5">
                        {#if isCountLoading}
                            <IconLoader2 class="size-3 animate-spin" />
                        {:else}
                            <span>{totalRows || "0"}</span>
                            <span>{showPlus ? "+" : ""}</span>
                        {/if}
                    </span>
                </button>
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
            <Button
                variant="ghost"
                size="icon"
                class="h-6 w-6"
                title="Last page"
                disabled={!showPlus}
                onclick={handleLast}
            >
                <IconPlayerSkipForward class="size-4" stroke={3} />
            </Button>
        </div>
        {/if}

        <div class="w-px h-5 bg-border/50"></div>

        <div class="w-px h-4 bg-border/40 mx-1"></div>

        <div class="flex items-center gap-1">
            {#if !hideExecute}
            <Button
                variant="ghost"
                size="sm"
                class="h-7 px-2 flex items-center gap-1.5 hover:bg-green-500/10 hover:text-green-500 transition-colors"
                title="Run (⌘+Enter)"
                onclick={handleExecute}
            >
                <IconPlayerPlayFilled class="size-5 text-green-500" />
                <span class="text-xs font-medium">Execute</span>
            </Button>

            <Button
                variant="ghost"
                size="sm"
                class="h-7 px-2 flex items-center gap-1.5 hover:bg-blue-500/10 hover:text-blue-500 transition-colors"
                title="Add Row"
                onclick={() => onAddRow?.()}
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="18"
                    height="18"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="text-blue-500"
                >
                    <path stroke="none" d="M0 0h24v24H0z" fill="none" />
                    <path d="M12 5l0 14" />
                    <path d="M5 12l14 0" />
                </svg>
                <span class="text-xs font-medium">Add Row</span>
            </Button>
            {/if}

            {#if isLoading}
                <Button
                    variant="ghost"
                    size="sm"
                    class="h-7 px-2 flex items-center gap-1.5 text-red-500 hover:bg-red-500/10 transition-colors"
                    title="Cancel query (Esc)"
                    onclick={handleCancel}
                >
                    <IconPlayerStopFilled class="size-5" />
                    <span class="text-xs font-medium">Cancel</span>
                </Button>
            {/if}

            <div class="w-px h-4 bg-border/40 mx-1"></div>

            <Button
                variant="ghost"
                size="sm"
                class="h-7 px-2 flex items-center gap-1.5"
                title="Refresh"
                onclick={handleRefresh}
                disabled={isLoading}
            >
                <IconRefresh
                    class="size-5 {isLoading
                        ? 'animate-spin-reverse'
                        : 'opacity-70'}"
                />
                <span class="text-xs font-medium">Refresh</span>
            </Button>

            <Menu.Root>
                <Menu.Trigger>
                    <Button
                        variant="ghost"
                        size="sm"
                        class={cn(
                            "h-7 px-2 flex items-center gap-1.5",
                            currentAutoRefresh > 0 &&
                                "border-2 border-transparent rounded-md bg-origin-border [background-clip:padding-box,border-box] bg-[linear-gradient(var(--theme-bg-secondary),var(--theme-bg-secondary)),linear-gradient(to_right,#ef4444,#f97316,#eab308,#22c55e,#3b82f6,#8b5cf6,#ec4899)]",
                        )}
                        title="Auto Refresh"
                    >
                        <IconClockPlay class="size-5 opacity-70" />
                        <span class="text-xs font-medium">Auto</span>
                    </Button>
                </Menu.Trigger>
                <Menu.Content
                    class="w-32 border border-(--theme-border-default) bg-(--theme-bg-secondary) shadow-lg"
                    align="start"
                >
                    <div
                        class="px-2 py-1.5 text-[10px] uppercase font-bold text-muted-foreground border-b border-border/50 bg-muted/30"
                    >
                        Auto Refresh
                    </div>
                    <div class="p-1 flex flex-col gap-0.5">
                        {#each autoRefreshOptions as option}
                            <Menu.Item
                                class="w-full text-left px-2 py-1.5 text-xs rounded hover:bg-muted cursor-pointer flex items-center justify-between"
                                onclick={() => handleAutoRefresh(option.value)}
                            >
                                <span>{option.label}</span>
                                {#if currentAutoRefresh === option.value}
                                    <IconCheck class="size-3" />
                                {/if}
                            </Menu.Item>
                        {/each}
                    </div>
                </Menu.Content>
            </Menu.Root>

            <div class="w-px h-4 bg-border/40 mx-1"></div>

            <!-- Save Button -->
            <Button
                variant="ghost"
                size="sm"
                disabled={isSaving || pendingChangesCount === 0}
                class="h-7 px-2 flex items-center gap-1.5 hover:text-green-500"
                title="Save all changes"
                onclick={() => onSaveChanges?.()}
            >
                {#if isSaving}
                    <IconLoader2 class="size-5 animate-spin" />
                {:else}
                    <IconDeviceFloppy class="size-5 opacity-70" />
                {/if}
                <span class="text-xs font-medium">Save</span>
            </Button>
            <!-- Changes Button -->
            <Button
                variant="ghost"
                size="sm"
                class="h-7 relative px-2 flex items-center gap-1.5"
                disabled={pendingChangesCount === 0}
                title="View pending changes"
                onclick={() => onShowChanges?.()}
            >
                <IconVersions class="size-5 opacity-70" />
                <span class="text-xs font-medium">Changes</span>
                {#if pendingChangesCount > 0}
                    <span
                        class="size-4 text-[9px] font-bold bg-(--theme-accent-primary) text-white rounded-full flex items-center justify-center ml-0.5"
                    >
                        {pendingChangesCount}
                    </span>
                {/if}
            </Button>
        </div>

        <div class="flex-1"></div>

        <div class="w-px h-5 bg-border/40 mx-1"></div>

        <!-- Export Menu -->

        <Menu.Root bind:open={exportOpen}>
            <Menu.Trigger>
                <Button
                    variant="ghost"
                    size="sm"
                    class="h-7 px-2 flex items-center gap-1.5"
                    title="Export"
                >
                    <IconDownload class="h-5 w-5 opacity-70" />
                    <span class="text-xs font-medium">Export</span>
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

    <!-- Row 2: WHERE and ORDER BY -->
    {#if !hideFilters}
        <div
            class="flex items-center gap-3 px-3 h-7 text-xs w-full border-t border-border/40 bg-muted/20"
        >
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

            <div class="w-px h-4 bg-border/50"></div>

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
    {/if}
</div>
