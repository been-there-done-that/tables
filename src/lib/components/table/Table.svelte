<script lang="ts">
    import { onMount, tick } from "svelte";
    import { TableController } from "./store.svelte";
    import type { Column, Row } from "./types";
    import TableHeader from "./TableHeader.svelte";
    import VirtualTableBody from "./VirtualTableBody.svelte";
    import CellEditor from "./CellEditor.svelte";
    import { cn } from "$lib/utils";

    interface Props {
        data: Row[];
        columns: Column[];
        class?: string;
    }

    let { data, columns, class: className }: Props = $props();

    // Init store
    const table = new TableController();

    // Reactively update store when props change
    $effect(() => {
        table.loadData(data, columns);
    });

    let container: HTMLDivElement;

    // Global Key handler for table navigation
    function onKeyDown(e: KeyboardEvent) {
        // Only if table has focus or contains focus
        if (!container?.contains(document.activeElement)) return;
        table.handleKeydown(e);
    }
</script>

<div
    bind:this={container}
    class={cn(
        "flex flex-col h-full w-full overflow-hidden rounded-md outline-none",
        className,
    )}
    style="background-color: var(--theme-bg-primary);"
    role="grid"
    tabindex="0"
    onkeydown={onKeyDown}
>
    <!-- Header is sticky/fixed height -->
    <TableHeader {table} />

    <!-- Virtual Body takes remaining space -->
    <div class="flex-1 overflow-hidden relative">
        <VirtualTableBody {table} />
    </div>

    <!-- Optional Footer / Status bar could go here -->
    <div
        class="h-6 border-t bg-muted/20 text-xs flex items-center px-2 text-muted-foreground select-none"
    >
        {table.processedRows.length} rows &middot; {table.selectedRowIds.size} selected
    </div>

    <!-- Editors -->
    <CellEditor {table} />
</div>
