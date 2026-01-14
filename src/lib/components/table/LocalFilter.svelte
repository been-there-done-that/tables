<script lang="ts">
    import { onMount, tick } from "svelte";
    import {
        IconSearch,
        IconX,
        IconLoader2,
        IconCheck,
    } from "@tabler/icons-svelte";
    import { cn } from "$lib/utils";
    import type { Column } from "./types";

    interface Props {
        column: Column;
        uniqueValues?: { value: any; count: number }[];
        currentFilter: any;
        onFilterChange: (value: any) => void;
        onClose: () => void;
    }

    let {
        column,
        uniqueValues = [],
        currentFilter,
        onFilterChange,
        onClose,
    }: Props = $props();

    let searchQuery = $state("");
    let selectedValues = $state<Set<string>>(new Set());
    let inputEl = $state<HTMLInputElement | null>(null);
    let selectedIndex = $state(-1);

    let isLoading = $derived(!uniqueValues || uniqueValues.length === 0);

    // Initialize state from currentFilter
    $effect(() => {
        if (currentFilter?.type === "in") {
            selectedValues = new Set(currentFilter.values.map(String));
        } else {
            selectedValues = new Set();
        }
    });

    // Auto-focus search input on mount
    onMount(() => {
        tick().then(() => inputEl?.focus());
    });

    let filteredValues = $derived(
        uniqueValues.filter((item) =>
            String(item.value)
                .toLowerCase()
                .includes(searchQuery.toLowerCase()),
        ),
    );

    // Calculate counts
    let allCount = $derived(filteredValues.length);
    let selectedCount = $derived(
        filteredValues.filter((item) => selectedValues.has(String(item.value)))
            .length,
    );
    let allSelected = $derived(allCount > 0 && selectedCount === allCount);

    function toggleValue(value: string) {
        const next = new Set(selectedValues);
        if (next.has(value)) {
            next.delete(value);
        } else {
            next.add(value);
        }
        selectedValues = next;
        updateFilter();
    }

    function toggleSelectAll() {
        const next = new Set(selectedValues);
        if (allSelected) {
            // Deselect all filtered
            filteredValues.forEach((item) => next.delete(String(item.value)));
        } else {
            // Select all filtered
            filteredValues.forEach((item) => next.add(String(item.value)));
        }
        selectedValues = next;
        updateFilter();
    }

    function clearFilter() {
        selectedValues = new Set();
        onFilterChange(null);
    }

    function updateFilter() {
        if (selectedValues.size === 0) {
            onFilterChange(null);
        } else {
            onFilterChange({
                type: "in",
                values: Array.from(selectedValues),
            });
        }
    }

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            onClose();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            if (selectedIndex < filteredValues.length - 1) {
                selectedIndex++;
                scrollToSelected();
            }
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            if (selectedIndex > 0) {
                selectedIndex--;
                scrollToSelected();
            }
        } else if (e.key === "Enter" && selectedIndex >= 0) {
            e.preventDefault();
            if (filteredValues[selectedIndex]) {
                toggleValue(String(filteredValues[selectedIndex].value));
            }
        } else if (e.key === " " && selectedIndex >= 0) {
            e.preventDefault();
            if (filteredValues[selectedIndex]) {
                toggleValue(String(filteredValues[selectedIndex].value));
            }
        }
    }

    function scrollToSelected() {
        tick().then(() => {
            const list = document.querySelector("[data-filter-list]");
            const selected = list?.querySelector("[data-highlighted=true]");
            selected?.scrollIntoView({ block: "nearest" });
        });
    }

    function formatValue(value: any): string {
        if (value === null || value === undefined) return "(Blanks)";
        if (value === "") return "(Empty)";
        return String(value);
    }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    class="flex flex-col w-full max-h-[320px] overflow-hidden bg-(--theme-bg-secondary) rounded-md"
    onkeydown={handleKeyDown}
>
    <!-- Header -->
    <div class="px-2 py-1.5 border-b border-(--theme-border-default)">
        <div
            class="text-[10px] font-medium text-(--theme-fg-secondary) opacity-70 uppercase tracking-wider mb-1"
        >
            Filter: {column.label ?? column.id}
        </div>
        <div class="relative">
            <IconSearch
                class="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-(--theme-fg-secondary) opacity-60"
            />
            <input
                bind:this={inputEl}
                bind:value={searchQuery}
                type="text"
                placeholder="Search values..."
                class="w-full pl-7 pr-6 py-1 text-xs bg-transparent border border-(--theme-border-default) rounded text-(--theme-fg-default) placeholder:text-(--theme-fg-secondary)/40 focus:outline-none focus:ring-1 focus:ring-(--theme-accent-primary)"
                disabled={isLoading}
            />
            {#if searchQuery}
                <button
                    type="button"
                    class="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-(--theme-bg-hover)"
                    onclick={() => (searchQuery = "")}
                >
                    <IconX class="size-3 text-(--theme-fg-secondary)" />
                </button>
            {/if}
        </div>
    </div>

    {#if isLoading}
        <div class="flex items-center justify-center py-6">
            <IconLoader2
                class="size-4 animate-spin text-(--theme-fg-secondary) opacity-60"
            />
        </div>
    {:else}
        <!-- Select All Row -->
        <button
            type="button"
            class={cn(
                "flex items-center gap-1.5 px-2 py-0.5 text-xs border-b border-(--theme-border-default) transition-colors",
                "hover:bg-(--theme-bg-hover) text-(--theme-fg-default)",
            )}
            onclick={toggleSelectAll}
        >
            <div
                class={cn(
                    "size-3.5 rounded-sm border flex items-center justify-center transition-colors flex-shrink-0",
                    allSelected
                        ? "bg-(--theme-accent-primary) border-(--theme-accent-primary)"
                        : selectedCount > 0
                          ? "bg-(--theme-accent-primary)/30 border-(--theme-accent-primary)"
                          : "border-(--theme-fg-secondary)/50",
                )}
            >
                {#if allSelected || selectedCount > 0}
                    <IconCheck class="size-2.5 text-white" />
                {/if}
            </div>
            <span class="flex-1 text-left font-medium">Select All</span>
            <span class="text-[9px] text-(--theme-fg-secondary) opacity-60">
                {selectedCount}/{allCount}
            </span>
        </button>

        <!-- Values List -->
        <div class="flex-1 overflow-y-auto py-0.5" data-filter-list>
            {#if filteredValues.length === 0}
                <div
                    class="px-2 py-4 text-center text-xs text-(--theme-fg-secondary) opacity-60"
                >
                    No values found
                </div>
            {:else}
                {#each filteredValues as item, i (String(item.value) + i)}
                    {@const isChecked = selectedValues.has(String(item.value))}
                    {@const isHighlighted = i === selectedIndex}
                    <button
                        type="button"
                        data-highlighted={isHighlighted}
                        class={cn(
                            "w-full flex items-center gap-1.5 px-2 py-0.5 text-xs transition-colors",
                            isHighlighted
                                ? "bg-(--theme-bg-active)"
                                : "hover:bg-(--theme-bg-hover)",
                            "text-(--theme-fg-default)",
                        )}
                        onclick={() => toggleValue(String(item.value))}
                        onmouseenter={() => (selectedIndex = i)}
                    >
                        <div
                            class={cn(
                                "size-3.5 rounded-sm border flex items-center justify-center transition-colors flex-shrink-0",
                                isChecked
                                    ? "bg-(--theme-accent-primary) border-(--theme-accent-primary)"
                                    : "border-(--theme-fg-secondary)/50",
                            )}
                        >
                            {#if isChecked}
                                <IconCheck class="size-2.5 text-white" />
                            {/if}
                        </div>
                        <span class="flex-1 text-left truncate">
                            {formatValue(item.value)}
                        </span>
                        <span
                            class="text-[9px] text-(--theme-fg-secondary) opacity-60 font-mono flex-shrink-0"
                        >
                            {item.count}
                        </span>
                    </button>
                {/each}
            {/if}
        </div>

        <!-- Footer -->
        <div
            class="px-2 py-1 border-t border-(--theme-border-default) flex items-center justify-between text-[9px] text-(--theme-fg-secondary) opacity-50"
        >
            <span>↑↓ Space ↵</span>
            {#if selectedValues.size > 0}
                <button
                    type="button"
                    class="px-1 py-0.5 rounded text-red-400 hover:bg-red-500/10 transition-colors"
                    onclick={clearFilter}
                >
                    Clear
                </button>
            {/if}
        </div>
    {/if}
</div>
