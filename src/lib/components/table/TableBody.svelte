<script lang="ts">
    import { tick, setContext } from "svelte";
    import type { Column, RowSelection, CellSelection } from "./types";
    import TableRow from "./TableRow.svelte";
    import VirtualScroller from "./VirtualScroller.svelte";
    import TableEmptyState from "./TableEmptyState.svelte";
    import TableLoadingState from "./TableLoadingState.svelte";

    interface Props {
        rows: any[];
        columns: Column[];
        selectedRows: RowSelection;
        selectedCells: CellSelection[];
        focusedCell: { rowIndex: number; columnIndex: number } | null;
        editingCell: CellSelection | null;
        pendingEdits: Record<string, any>;
        deletedRowIds: Set<string>;
        getRowKey: (row: any) => string;
        onRowSelect: (rowId: any, multi: boolean, range: boolean) => void;
        loading?: boolean;
        emptyTitle?: string;
        emptyDescription?: string;
        onScroll?: (e: Event) => void;
        onCellClick?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
        onCellMouseDown?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
        onCellMouseEnter?: (rowIndex: number, columnIndex: number) => void;
        onCellDoubleClick?: (rowIndex: number, columnIndex: number) => void;
        onCellContextMenu?: (
            rowIndex: number,
            columnIndex: number,
            event: MouseEvent,
        ) => void;
        onEditComplete?: (
            rowIndex: number,
            columnIndex: number,
            newValue: any,
        ) => void;
        onEditCancel?: () => void;
        header: import("svelte").Snippet;
    }

    let {
        rows,
        columns,
        selectedRows,
        selectedCells,
        focusedCell,
        editingCell,
        pendingEdits,
        onRowSelect,
        loading,
        emptyTitle,
        emptyDescription,
        onScroll,
        onCellClick,
        onCellMouseDown,
        onCellMouseEnter,
        onCellDoubleClick,
        onCellContextMenu,
        onEditComplete,
        onEditCancel,
        deletedRowIds,
        getRowKey,
        header,
    }: Props = $props();

    // Determine what state to show
    const showLoading = $derived(loading && rows.length === 0);
    const showEmpty = $derived(!loading && rows.length === 0);
    const showData = $derived(rows.length > 0);

    let measuredItemHeight = $state(36);
    let container: HTMLDivElement;
    // svelte-ignore non_reactive_update
    let virtualScroller: VirtualScroller;

    // Measure first row to keep virtual scroll math aligned with DOM
    $effect(() => {
        // Track rows to re-measure when data arrives
        const rowCount = rows.length;
        if (rowCount === 0) return;

        tick().then(() => {
            const firstRow = container?.querySelector(
                "[data-row]",
            ) as HTMLElement | null;
            if (!firstRow) return;

            const h = firstRow.getBoundingClientRect().height;
            if (h && Math.abs(h - measuredItemHeight) > 0.5) {
                measuredItemHeight = h;
                console.info("[TableBody] reactive measured itemHeight", {
                    h,
                    rowCount,
                });
            }
        });
    });

    export function scrollToIndex(
        index: number,
        align: "start" | "end" | "center" | "auto" = "auto",
    ) {
        console.info("[TableBody] scrollToIndex", {
            index,
            align,
            itemHeight: measuredItemHeight,
            scrollTop: virtualScroller?.getScrollTop?.(),
        });
        virtualScroller?.scrollToIndex(index, align);
    }

    export function scrollToLeft(left: number) {
        console.info("[TableBody] scrollToLeft", { left });
        virtualScroller?.scrollToLeft(left);
    }

    export function getContainer() {
        return virtualScroller?.getContainer?.();
    }

    export function focusCell(rowIndex: number, columnIndex: number) {
        // Find the cell within the virtual scroller container
        const scrollerContainer = virtualScroller?.getContainer?.();
        if (!scrollerContainer) {
            return false;
        }

        const selector = `[data-row-index="${rowIndex}"][data-col-index="${columnIndex}"]`;
        const cell = scrollerContainer.querySelector(selector) as HTMLElement;

        if (cell) {
            cell.focus({ preventScroll: true });
            return true;
        } else {
            return false;
        }
    }

    setContext("table-container", () => virtualScroller?.getContainer?.());
</script>

<div class="h-full w-full relative" bind:this={container} data-table-container>
    {#if showLoading}
        <TableLoadingState {columns} />
    {:else}
        <div class="flex flex-col h-full w-full overflow-hidden">
            {#if showEmpty}
                <!-- For empty state, we still want the header to show if we have columns -->
                <div
                    class="border-b border-border bg-surface w-fit sticky top-0 z-10"
                >
                    {@render header()}
                </div>
                <div class="flex-1 overflow-auto">
                    <TableEmptyState
                        title={emptyTitle}
                        description={emptyDescription}
                    />
                </div>
            {:else}
                <VirtualScroller
                    bind:this={virtualScroller}
                    items={rows}
                    itemHeight={measuredItemHeight}
                    class="h-full w-full text-foreground"
                    {onScroll}
                    {header}
                >
                    {#snippet children(row: any, index: number)}
                        <TableRow
                            {row}
                            {columns}
                            rowIndex={index}
                            selected={!!selectedRows[getRowKey(row)]}
                            {selectedCells}
                            {focusedCell}
                            {editingCell}
                            {pendingEdits}
                            {deletedRowIds}
                            {getRowKey}
                            disabled={loading}
                            {onRowSelect}
                            {onCellClick}
                            {onCellMouseDown}
                            {onCellMouseEnter}
                            {onCellDoubleClick}
                            {onCellContextMenu}
                            {onEditComplete}
                            {onEditCancel}
                        />
                    {/snippet}
                </VirtualScroller>
            {/if}
        </div>
    {/if}
</div>
