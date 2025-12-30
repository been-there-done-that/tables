<script lang="ts">
    import type { TableController } from "./store.svelte";
    import type { Row } from "./types";
    import TableCell from "./TableCell.svelte";
    import { cn } from "$lib/utils";

    let {
        table,
        row,
        rowIndex,
    }: { table: TableController; row: Row; rowIndex: number } = $props();

    let isSelected = $derived(table.selectedRowIds.has(row._rowId));
</script>

{#each table.columns as column, colIndex}
    <TableCell {table} {row} {column} {rowIndex} {colIndex} {isSelected} />
{/each}
