<script lang="ts">
    import { onMount, tick } from "svelte";
    import type { Column } from "./types";
    import {
        IconSearch,
        IconX,
        IconCheck,
        IconEye,
        IconEyeOff,
    } from "@tabler/icons-svelte";
    import { cn } from "$lib/utils";

    interface Props {
        open: boolean;
        columns: Column[];
        allColumns: Column[];
        hiddenColumnIds: Set<string>;
        onSelect: (columnId: string) => void;
        onToggleVisibility: (columnId: string) => void;
        onUnhideAll: () => void;
        onClose: () => void;
    }

    let {
        open,
        columns,
        allColumns,
        hiddenColumnIds,
        onSelect,
        onToggleVisibility,
        onUnhideAll,
        onClose,
    }: Props = $props();

    let searchValue = $state("");
    let inputEl = $state<HTMLInputElement | null>(null);
    let selectedIndex = $state(0);
    let dialogEl = $state<HTMLDivElement | null>(null);

    // Show all columns, not just visible ones
    const filteredColumns = $derived(
        searchValue.trim() === ""
            ? allColumns
            : allColumns.filter(
                  (c) =>
                      c.label
                          .toLowerCase()
                          .includes(searchValue.toLowerCase()) ||
                      c.id.toLowerCase().includes(searchValue.toLowerCase()),
              ),
    );

    const hiddenCount = $derived(hiddenColumnIds.size);
    const visibleCount = $derived(allColumns.length - hiddenColumnIds.size);

    // Check if a specific column can be hidden (min 1 must remain visible)
    function canHideColumn(columnId: string): boolean {
        const isCurrentlyHidden = hiddenColumnIds.has(columnId);
        if (isCurrentlyHidden) return true; // Can always unhide
        return visibleCount > 1; // Can hide only if more than 1 visible
    }

    $effect(() => {
        if (open) {
            searchValue = "";
            selectedIndex = 0;
            tick().then(() => {
                inputEl?.focus();
            });
        }
    });

    // Reset selection when filter changes
    $effect(() => {
        if (filteredColumns.length > 0) {
            if (selectedIndex >= filteredColumns.length) {
                selectedIndex = 0;
            }
        }
    });

    function handleKeyDown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            onClose();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            if (selectedIndex < filteredColumns.length - 1) {
                selectedIndex++;
                scrollToSelected();
            }
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            if (selectedIndex > 0) {
                selectedIndex--;
                scrollToSelected();
            }
        } else if (e.key === "Enter") {
            e.preventDefault();
            if (filteredColumns[selectedIndex]) {
                handleSelect(filteredColumns[selectedIndex].id);
            }
        } else if (e.key === " ") {
            e.preventDefault();
            if (filteredColumns[selectedIndex]) {
                const colId = filteredColumns[selectedIndex].id;
                if (canHideColumn(colId)) {
                    onToggleVisibility(colId);
                }
            }
        }
    }

    function scrollToSelected() {
        tick().then(() => {
            const selected = dialogEl?.querySelector("[data-selected=true]");
            selected?.scrollIntoView({ block: "nearest" });
        });
    }

    function handleSelect(columnId: string) {
        // Only navigate if the column is visible
        if (!hiddenColumnIds.has(columnId)) {
            onSelect(columnId);
            onClose();
        }
    }

    function handleClickOutside(e: MouseEvent) {
        if (dialogEl && !dialogEl.contains(e.target as Node)) {
            onClose();
        }
    }

    onMount(() => {
        window.addEventListener("mousedown", handleClickOutside);
        return () => {
            window.removeEventListener("mousedown", handleClickOutside);
        };
    });
</script>

{#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="fixed inset-0 z-1600 bg-black/20 backdrop-blur-[1px] flex items-start justify-center pt-[15vh]"
        onkeydown={handleKeyDown}
    >
        <div
            bind:this={dialogEl}
            class="bg-(--theme-bg-secondary) border border-(--theme-border-default) rounded-lg shadow-xl w-[340px] max-h-[400px] flex flex-col overflow-hidden"
        >
            <!-- Search Header -->
            <div class="px-2 py-1.5 border-b border-(--theme-border-default)">
                <div class="flex items-center justify-between mb-1">
                    <span
                        class="text-[10px] font-medium text-(--theme-fg-secondary) opacity-70 uppercase tracking-wider"
                    >
                        Columns
                    </span>
                    {#if hiddenCount > 0}
                        <button
                            type="button"
                            class="text-[9px] text-(--theme-accent-primary) hover:underline"
                            onclick={onUnhideAll}
                        >
                            Show all ({hiddenCount} hidden)
                        </button>
                    {/if}
                </div>
                <div class="relative">
                    <IconSearch
                        class="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 text-(--theme-fg-secondary) opacity-60"
                    />
                    <input
                        bind:this={inputEl}
                        bind:value={searchValue}
                        type="text"
                        placeholder="Search columns..."
                        class="w-full pl-7 pr-6 py-1 text-xs bg-transparent border border-(--theme-border-default) rounded text-(--theme-fg-default) placeholder:text-(--theme-fg-secondary)/40 focus:outline-none focus:ring-1 focus:ring-(--theme-accent-primary)"
                    />
                    {#if searchValue}
                        <button
                            type="button"
                            class="absolute right-1.5 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-(--theme-bg-hover)"
                            onclick={() => (searchValue = "")}
                        >
                            <IconX class="size-3 text-(--theme-fg-secondary)" />
                        </button>
                    {/if}
                </div>
            </div>

            <!-- Column List -->
            <div class="flex-1 overflow-y-auto py-0.5">
                {#if filteredColumns.length === 0}
                    <div
                        class="px-2 py-4 text-center text-xs text-(--theme-fg-secondary) opacity-60"
                    >
                        No columns found
                    </div>
                {:else}
                    {#each filteredColumns as column, i (column.id)}
                        {@const isHidden = hiddenColumnIds.has(column.id)}
                        {@const canToggle = canHideColumn(column.id)}
                        <div
                            data-selected={i === selectedIndex}
                            class={cn(
                                "w-full flex items-center gap-1.5 px-2 py-0.5 text-xs transition-colors",
                                i === selectedIndex
                                    ? "bg-(--theme-bg-active)"
                                    : "hover:bg-(--theme-bg-hover)",
                                isHidden ? "opacity-50" : "",
                            )}
                            onmouseenter={() => (selectedIndex = i)}
                        >
                            <!-- Visibility Toggle -->
                            <button
                                type="button"
                                class={cn(
                                    "p-0.5 rounded transition-colors",
                                    isHidden
                                        ? "text-(--theme-fg-secondary) hover:text-(--theme-fg-default)"
                                        : canToggle
                                          ? "text-(--theme-accent-primary) hover:text-(--theme-accent-primary)"
                                          : "text-(--theme-accent-primary) opacity-50 cursor-not-allowed",
                                )}
                                onclick={(e) => {
                                    e.stopPropagation();
                                    if (canToggle) {
                                        onToggleVisibility(column.id);
                                    }
                                }}
                                title={isHidden
                                    ? "Show column"
                                    : canToggle
                                      ? "Hide column"
                                      : "Cannot hide (min 1 required)"}
                                disabled={!canToggle}
                            >
                                {#if isHidden}
                                    <IconEyeOff class="size-3.5" />
                                {:else}
                                    <IconEye class="size-3.5" />
                                {/if}
                            </button>

                            <!-- Column name (clickable to navigate) -->
                            <button
                                type="button"
                                class={cn(
                                    "flex-1 text-left truncate",
                                    isHidden
                                        ? "text-(--theme-fg-secondary) cursor-default"
                                        : "text-(--theme-fg-default) cursor-pointer",
                                )}
                                onclick={() => handleSelect(column.id)}
                                disabled={isHidden}
                            >
                                {column.label}
                            </button>

                            <span
                                class="text-[9px] text-(--theme-fg-secondary) opacity-60 font-mono shrink-0"
                            >
                                {column.rawType || column.type}
                            </span>
                        </div>
                    {/each}
                {/if}
            </div>

            <!-- Footer hint -->
            <div
                class="px-2 py-1 border-t border-(--theme-border-default) text-[9px] text-(--theme-fg-secondary) opacity-50 flex justify-between"
            >
                <span>↑↓ Space:toggle</span>
                <span>↵ Go to</span>
                <span>Esc</span>
            </div>
        </div>
    </div>
{/if}
