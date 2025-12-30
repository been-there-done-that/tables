<script lang="ts">
    import { onMount } from "svelte";
    import type { TableController } from "./store.svelte";
    import TableRow from "./TableRow.svelte";

    let { table }: { table: TableController } = $props();

    let viewport: HTMLDivElement;

    function onScroll(e: Event) {
        const target = e.target as HTMLElement;
        table.handleScroll(target.scrollTop, target.clientHeight);
    }

    onMount(() => {
        if (viewport) {
            table.handleScroll(viewport.scrollTop, viewport.clientHeight);
        }
    });
</script>

<div
    bind:this={viewport}
    bind:clientHeight={table.containerHeight}
    class="flex-1 w-full overflow-y-auto overflow-x-hidden relative outline-none"
    onscroll={onScroll}
    role="rowgroup"
>
    <!-- Spacer -->
    <div
        style="height: {table.totalHeight}px; width: 100%; position: relative;"
    >
        {#each table.virtualState.visibleItems as row (row._rowId)}
            <div
                class="absolute left-0 right-0 flex border-b divide-x last:border-b-0"
                style="
                    top: {(table.virtualState.startIndex +
                    table.virtualState.visibleItems.indexOf(row)) *
                    32}px;
                    height: 32px;
                    border-color: var(--theme-border-subtle);
                "
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
