<script lang="ts">
    import type { TableController } from "./store.svelte";
    import type { Row, Column } from "./types";
    import { cn } from "$lib/utils";

    let {
        table,
        row,
        column,
        rowIndex,
        colIndex,
        isSelected: rowSelected,
    }: {
        table: TableController;
        row: Row;
        column: Column;
        rowIndex: number;
        colIndex: number;
        isSelected: boolean;
    } = $props();

    let value = $derived(row[column.id]);

    // Check if this specific cell is selected/focused
    // Using simple derived state. In massive tables, Set lookups are fast enough.
    // Optimization: Store could return a "isCellSelected" helper
    let isFocused = $derived(
        table.focusedCell?.rowIndex === rowIndex &&
            table.focusedCell?.columnIndex === colIndex,
    );

    let isCellSelected = $derived.by(() => {
        return table.selectedCells.some(
            (c) => c.rowId === row._rowId && c.columnId === column.id,
        );
    });

    function onClick(e: MouseEvent) {
        table.selectCell(
            rowIndex,
            colIndex,
            e.metaKey || e.ctrlKey,
            e.shiftKey,
        );
    }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class={cn(
        "flex items-center px-3 text-sm truncate cursor-default select-none h-full outline-none",
        rowSelected && "bg-blue-50 dark:bg-blue-900/20",
        isCellSelected &&
            "bg-blue-100 dark:bg-blue-900/40 ring-1 ring-inset ring-blue-300 dark:ring-blue-700",
        isFocused && "ring-2 ring-inset ring-blue-600 dark:ring-blue-400 z-10",
    )}
    style="width: {column.width}px; min-width: {column.width}px;"
    onclick={onClick}
>
    <!-- Simple rendering for now - extend based on types -->
    {#if column.type === "boolean"}
        <span
            class={cn(
                "px-1.5 py-0.5 rounded text-[10px] font-medium border",
                value
                    ? "bg-green-100 border-green-200 text-green-700"
                    : "bg-gray-100 border-gray-200 text-gray-600",
            )}
        >
            {value ? "TRUE" : "FALSE"}
        </span>
    {:else if column.type === "date" || column.type === "datetime"}
        <span class="opacity-80 font-mono text-xs">
            {value ? new Date(value).toLocaleDateString() : ""}
        </span>
    {:else}
        {value ?? ""}
    {/if}
</div>
