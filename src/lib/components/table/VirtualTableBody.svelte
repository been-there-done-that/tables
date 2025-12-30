<script lang="ts">
    import type { TableController } from "./store.svelte";
    import TableRow from "./TableRow.svelte";

    let { table }: { table: TableController } = $props();

    let scrollContainer: HTMLDivElement;

    function onScroll(e: UIEvent) {
        const target = e.target as HTMLDivElement;
        table.handleScroll(target.scrollTop, target.clientHeight);
    }
</script>

<div
    bind:this={scrollContainer}
    class="w-full h-full overflow-auto relative"
    onscroll={onScroll}
>
    <!-- 
         The 'phantom' spacer div determines the total scrollable height.
         Virtual rows are placed absolutely.
    -->
    <div
        style="height: {table.totalHeight}px; width: 100%; position: relative;"
    >
        {#each table.virtualState.visibleItems as row (row._rowId)}
            <div
                class="absolute left-0 right-0 flex border-b divide-x last:border-b-0 hover:bg-muted/20"
                style="top: {(table.virtualState.startIndex +
                    table.virtualState.visibleItems.indexOf(row)) *
                    32}px; height: 32px;"
            >
                <TableRow
                    {table}
                    {row}
                    rowIndex={table.virtualState.startIndex +
                        table.virtualState.visibleItems.indexOf(row)}
                />
            </div>
        {/each}
    </div>
</div>
