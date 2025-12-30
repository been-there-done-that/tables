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
    }: {
        table: TableController;
        row: Row;
        column: Column;
        rowIndex: number;
        colIndex: number;
    } = $props();

    let value = $derived(row[column.id]);

    // Check if this specific cell is selected/focused
    // Optimization: Use Set lookup O(1)
    let rowSelected = $derived(table.selectedRowIds.has(row._rowId));
    let isCellSelected = $derived(
        table.selectedCellSet.has(`${row._rowId}:${column.id}`),
    );
    let isFocused = $derived(
        table.focusedCell?.rowIndex === rowIndex &&
            table.focusedCell?.columnIndex === colIndex,
    );

    let isEditing = $derived(
        table.editingCell?.rowId === row._rowId &&
            table.editingCell?.columnId === column.id,
    );

    function onClick(e: MouseEvent) {
        // Stop propagation to prevent Table container focus reset?
        // No, keep it standard. table.selectCell handles state.
        table.selectCell(
            rowIndex,
            colIndex,
            e.metaKey || e.ctrlKey,
            e.shiftKey,
        );
    }

    function onDblClick(e: MouseEvent) {
        const target = e.currentTarget as HTMLElement;
        const rect = target.getBoundingClientRect();
        table.startEditing(rowIndex, colIndex, rect);
    }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class={cn(
        "flex items-center px-3 text-sm truncate cursor-default select-none h-full outline-none border-r last:border-r-0 group",
        // Default border
    )}
    style="
        width: {column.width}px;
        min-width: {column.width}px;
        border-color: var(--theme-border-subtle);
        background-color: {isCellSelected
        ? 'var(--theme-bg-tertiary)'
        : rowSelected
          ? 'var(--theme-bg-secondary)'
          : 'transparent'};
        color: {isCellSelected ? 'var(--theme-fg-primary)' : 'inherit'};
    "
    onclick={onClick}
    ondblclick={onDblClick}
    role="gridcell"
>
    <!-- Focused Ring Overlay (Only for keyboard focus or active cell indicator) -->
    {#if isFocused && !isEditing}
        <!-- Use a subtle inset shadow or border instead of ring which might look like 'border highlight' user disliked? -->
        <!-- User said: 'row highlight can't be border'. Focused cell usually needs a border. -->
        <!-- I'll use a clean blue border for focused cell. -->
        <div
            class="absolute inset-0 pointer-events-none border-2 border-[var(--theme-accent-primary)] z-10 box-border"
        ></div>
    {/if}

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
