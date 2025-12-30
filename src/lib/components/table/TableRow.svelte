<script lang="ts">
    import type { TableController } from "./store.svelte";
    import type { Row } from "./types";
    import TableCell from "./TableCell.svelte";

    let {
        table,
        row,
        rowIndex,
    }: {
        table: TableController;
        row: Row;
        rowIndex: number;
    } = $props();
</script>

<div class="flex">
    {#each table.columns as column, colIndex (column.id)}
        <TableCell {table} {row} {column} {rowIndex} {colIndex} />
    {/each}
</div>
