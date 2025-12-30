<script lang="ts">
    import { onMount } from "svelte";
    import type { TableController } from "./store.svelte";
    import { cn } from "$lib/utils";

    let { table }: { table: TableController } = $props();

    let inputEl: HTMLInputElement;
    let containerEl: HTMLDivElement;

    // Initial value
    let initialValue: any;
    // Find current value
    $effect(() => {
        if (table.editingCell) {
            const row = table.rawRows.find(
                (r) => r._rowId === table.editingCell!.rowId,
            );
            initialValue = row ? row[table.editingCell!.columnId] : "";
            if (inputEl) {
                inputEl.value = String(initialValue ?? "");
                inputEl.focus();
                inputEl.select();
            }
        }
    });

    function onKeyDown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            // parsing based on type?
            // Simple string handling for now.
            // Ideally we check column type and parse (int, float, date wrapper).
            table.commitEditing(inputEl.value);
        } else if (e.key === "Escape") {
            e.preventDefault();
            table.cancelEditing();
        }
    }

    // Click outside to commit or cancel?
    function onWindowClick(e: MouseEvent) {
        if (containerEl && !containerEl.contains(e.target as Node)) {
            table.commitEditing(inputEl.value);
        }
    }
</script>

<svelte:window onmousedown={onWindowClick} />

{#if table.editingCell && table.editingRect}
    <div
        bind:this={containerEl}
        class="fixed z-50 shadow-lg border rounded bg-white dark:bg-zinc-800 anim-pop"
        style="
            top: {table.editingRect.top}px; 
            left: {table.editingRect.left}px; 
            width: {Math.max(table.editingRect.width, 150)}px; 
            min-height: {table.editingRect.height}px;
            border-color: var(--theme-accent-primary);
        "
    >
        <input
            bind:this={inputEl}
            class="w-full h-full px-2 py-1 bg-transparent border-0 outline-none text-sm"
            style="color: var(--theme-fg-primary);"
            onkeydown={onKeyDown}
        />
    </div>
{/if}
