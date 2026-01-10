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
        pendingEdits: Record<number, any>;
        onRowSelect: (rowId: number, multi: boolean, range: boolean) => void;
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
    }: Props = $props();

    // Determine what state to show
    const showLoading = $derived(loading && rows.length === 0);
    const showEmpty = $derived(!loading && rows.length === 0);
    const showData = $derived(rows.length > 0);

    let measuredItemHeight = $state(36);
    let container: HTMLDivElement;
    let virtualScroller: VirtualScroller;

    // Measure first row to keep virtual scroll math aligned with DOM
    $effect(() => {
        tick().then(() => {
            const firstRow = container?.querySelector(
                "[data-row]",
            ) as HTMLElement | null;
            if (!firstRow) return;
            const h = firstRow.getBoundingClientRect().height;
            if (h && Math.abs(h - measuredItemHeight) > 0.5) {
                measuredItemHeight = h;
                console.info("[TableBody] measured itemHeight", { h });
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

    setContext("table-container", () => virtualScroller?.getContainer?.());
</script>

<div class="h-full w-full relative" bind:this={container} data-table-container>
    {#if showLoading}
        <TableLoadingState {columns} />
    {:else if showEmpty}
        <TableEmptyState title={emptyTitle} description={emptyDescription} />
    {:else}
        <VirtualScroller
            bind:this={virtualScroller}
            items={rows}
            itemHeight={measuredItemHeight}
            class="h-full w-full"
            {onScroll}
        >
            {#snippet children(row: any, index: number)}
                <TableRow
                    {row}
                    {columns}
                    rowIndex={index}
                    selected={!!selectedRows[row._rowId]}
                    {selectedCells}
                    {focusedCell}
                    {editingCell}
                    {pendingEdits}
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
