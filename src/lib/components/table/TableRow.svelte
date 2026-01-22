<script lang="ts">
    import type { Column, CellSelection } from "./types";
    import TableCell from "./TableCell.svelte";
    import { cn } from "$lib/utils";

    interface Props {
        row: any;
        columns: Column[];
        rowIndex: number;
        selected: boolean;
        selectedCells: CellSelection[];
        focusedCell: { rowIndex: number; columnIndex: number } | null;
        editingCell: CellSelection | null;
        pendingEdits: Record<string, any>;
        deletedRowIds: Set<string>;
        getRowKey: (row: any) => string;
        onRowSelect: (rowId: any, multi: boolean, range: boolean) => void;
        disabled?: boolean;
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
        row,
        columns,
        rowIndex,
        selected,
        selectedCells,
        focusedCell,
        editingCell,
        pendingEdits,
        deletedRowIds,
        getRowKey,
        onRowSelect,
        disabled = false,
        onCellClick,
        onCellMouseDown,
        onCellMouseEnter,
        onCellDoubleClick,
        onCellContextMenu,
        onEditComplete,
        onEditCancel,
    }: Props = $props();

    function handleClick(e: MouseEvent) {
        if (disabled) return;
        onRowSelect(getRowKey(row), e.metaKey || e.ctrlKey, e.shiftKey);
    }

    function isCellSelected(columnIndex: number): boolean {
        const columnId = columns[columnIndex].id;
        const rKey = getRowKey(row);
        return selectedCells.some(
            (c) => c.rowId === rKey && c.columnId === columnId,
        );
    }

    function isCellFocused(columnIndex: number): boolean {
        return (
            focusedCell?.rowIndex === rowIndex &&
            focusedCell?.columnIndex === columnIndex
        );
    }

    function isCellEditing(columnIndex: number): boolean {
        const columnId = columns[columnIndex].id;
        return (
            editingCell?.rowId === getRowKey(row) &&
            editingCell?.columnId === columnId
        );
    }

    function hasPendingCellEdit(columnIndex: number): boolean {
        const rowId = getRowKey(row);
        if (!pendingEdits[rowId]) return false;

        const columnId = columns[columnIndex].id;
        // Check if this specific column has a value in the pending edits
        return columnId in pendingEdits[rowId];
    }

    const isDeleted = $derived(deletedRowIds.has(getRowKey(row)));
    const isInserted = $derived(!!row._tempId);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class={cn(
        "flex min-w-full w-fit hover:bg-muted cursor-default",
        selected
            ? "bg-surface text-foreground"
            : "bg-background text-foreground",
        isDeleted && "opacity-50 line-through bg-red-500/5 hover:bg-red-500/10",
        isInserted && "bg-green-500/5 hover:bg-green-500/10 italic",
        disabled && "opacity-70 pointer-events-none",
    )}
    onclick={handleClick}
    data-row
    data-row-id={getRowKey(row)}
>
    <!-- Row Number Cell -->
    <div
        class={cn(
            "sticky left-0 z-5 flex items-center justify-center border-r border-border bg-surface px-2 py-1 text-xs text-foreground-muted font-mono select-none",
            isInserted && "text-green-600 font-bold",
            isDeleted && "text-red-500",
        )}
        style="width: 60px; min-width: 60px; flex-shrink: 0;"
    >
        {#if isInserted}
            +
        {:else if isDeleted}
            -
        {:else}
            {row._rowId}
        {/if}
    </div>

    {#each columns as column, columnIndex (column.id)}
        {@const rowId = getRowKey(row)}
        {@const pendingValue = pendingEdits[rowId]?.[column.id]}
        <TableCell
            {row}
            {column}
            {rowIndex}
            {columnIndex}
            isSelected={isCellSelected(columnIndex)}
            isFocused={isCellFocused(columnIndex)}
            isEditing={isCellEditing(columnIndex)}
            isPendingEdit={hasPendingCellEdit(columnIndex)}
            {pendingValue}
            disabled={disabled || isDeleted}
            onClick={onCellClick}
            onMouseDown={onCellMouseDown}
            onMouseEnter={onCellMouseEnter}
            onDoubleClick={onCellDoubleClick}
            onContextMenu={onCellContextMenu}
            {onEditComplete}
            {onEditCancel}
        />
    {/each}
</div>
