<script lang="ts">
    import { Input } from "$lib/components/ui/input";
    import { Button } from "$lib/components/ui/button";
    import { Checkbox } from "$lib/components/ui/checkbox";
    import { IconSearch, IconLoader2 } from "@tabler/icons-svelte";
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
    let selectAll = $state(false);

    let isLoading = $derived(!uniqueValues || uniqueValues.length === 0);

    // Initialize state from currentFilter
    $effect(() => {
        if (currentFilter?.type === "in") {
            selectedValues = new Set(currentFilter.values.map(String));
        } else {
            selectedValues = new Set();
        }
    });

    let filteredValues = $derived(
        uniqueValues.filter((item) =>
            String(item.value)
                .toLowerCase()
                .includes(searchQuery.toLowerCase()),
        ),
    );

    function toggleValue(value: string) {
        if (selectedValues.has(value)) {
            selectedValues.delete(value);
        } else {
            selectedValues.add(value);
        }
        selectedValues = new Set(selectedValues); // Trigger reactivity
        updateFilter();
    }

    function toggleSelectAll() {
        selectAll = !selectAll;
        if (selectAll) {
            filteredValues.forEach((item) =>
                selectedValues.add(String(item.value)),
            );
        } else {
            selectedValues.clear();
        }
        selectedValues = new Set(selectedValues);
        updateFilter();
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
</script>

<div class="flex flex-col gap-2 p-2 w-full min-w-[260px]">
    <div class="text-sm font-semibold">
        Local Filter for ‘{column.label ?? column.id}’
    </div>

    <div class="relative">
        <IconSearch
            class="absolute left-2 top-2.5 h-4 w-4 text-muted-foreground"
        />
        <Input
            placeholder="Search values..."
            class="pl-8 h-9"
            bind:value={searchQuery}
            disabled={isLoading}
        />
    </div>

    {#if isLoading}
        <div class="flex items-center justify-center py-8">
            <IconLoader2 class="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
    {:else}
        <div
            class="flex items-center space-x-2 py-2 border-b text-sm font-medium"
        >
            <Checkbox
                id="select-all"
                checked={selectAll}
                onCheckedChange={toggleSelectAll}
            />
            <label
                for="select-all"
                class="flex-1 leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
            >
                Value
            </label>
            <span class="text-xs text-muted-foreground">Count</span>
        </div>

        <div class="flex flex-col gap-2 max-h-72 overflow-y-auto pt-2">
            {#each filteredValues as item}
                <div class="flex items-center space-x-2">
                    <Checkbox
                        id={`val-${item.value}`}
                        checked={selectedValues.has(String(item.value))}
                        onCheckedChange={() => toggleValue(String(item.value))}
                    />
                    <label
                        for={`val-${item.value}`}
                        class="text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 flex-1 truncate cursor-pointer"
                    >
                        {item.value === null ? "(Blanks)" : item.value}
                    </label>
                    <span class="text-xs text-muted-foreground"
                        >{item.count}</span
                    >
                </div>
            {/each}
            {#if filteredValues.length === 0 && !isLoading}
                <div class="text-sm text-muted-foreground text-center py-4">
                    No matches found
                </div>
            {/if}
        </div>

        <div class="text-[11px] text-muted-foreground pt-1">
            Select values to filter rows
        </div>
    {/if}
</div>
