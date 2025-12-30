<script lang="ts">
    import type { TableController } from "./store.svelte";
    import { cn } from "$lib/utils";

    let { table }: { table: TableController } = $props();

    // Derived total width for the scrolling container inside header
    // We must match the body's horizontal scroll
    // But for now, we'll rely on synchronization or simplified overflow

    function onHeaderClick(e: MouseEvent, colId: string) {
        table.toggleSort(colId, e.shiftKey || e.metaKey || e.ctrlKey);
    }

    function handleResizeStart(e: MouseEvent, column: any) {
        e.preventDefault();
        e.stopPropagation();
        const startX = e.clientX;
        const startWidth = column.width;

        function onMove(ev: MouseEvent) {
            table.resizeColumn(column.id, startWidth + (ev.clientX - startX));
        }
        function onUp() {
            window.removeEventListener("mousemove", onMove);
            window.removeEventListener("mouseup", onUp);
        }
        window.addEventListener("mousemove", onMove);
        window.addEventListener("mouseup", onUp);
    }
</script>

<div
    class="flex border-b bg-muted/40 font-medium text-xs text-muted-foreground select-none overflow-hidden h-8 shrink-0"
>
    <!-- 
      Ideally, this container should sync horizontal scroll with Body.
      For this MVP step, we assume simple synced layout or overflow-x auto matching body.
      Correct implementation usually involves a ref to this div and syncing scrollLeft from body.
    -->
    <div
        class="flex items-center"
        style="transform: translateX(-{/* We need scrollLeft from store? Or direct sync? */ 0}px)"
    >
        {#each table.columns as column}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
                class="relative flex items-center px-3 h-full border-r last:border-r-0 hover:bg-muted/60 cursor-pointer group transition-colors"
                style="width: {column.width}px; min-width: {column.width}px;"
                onclick={(e) => onHeaderClick(e, column.id)}
            >
                <span class="truncate">{column.label}</span>

                {#if table.sortState.find((s) => s.columnId === column.id)}
                    {@const dir = table.sortState.find(
                        (s) => s.columnId === column.id,
                    )?.direction}
                    <span class="ml-1 text-[10px] opacity-70">
                        {dir === "asc" ? "▲" : "▼"}
                    </span>
                {/if}

                <!-- Resizer Handle -->
                <div
                    class="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-400 z-10"
                    role="separator"
                    tabindex="-1"
                    aria-label="Resize column"
                    onclick={(e) => e.stopPropagation()}
                    onmousedown={(e) => handleResizeStart(e, column)}
                ></div>
            </div>
        {/each}
    </div>
</div>
