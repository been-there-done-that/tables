<!-- src/lib/components/agent/ComposerDropdown.svelte -->
<script lang="ts">
    import IconFileText from "@tabler/icons-svelte/icons/file-text";
    import IconTable from "@tabler/icons-svelte/icons/table";
    import IconListSearch from "@tabler/icons-svelte/icons/list-search";

    export interface DropdownItem {
        type: "file" | "table" | "result";
        label: string;
        sublabel?: string;
        path?: string;
        tableName?: string;
        toolId?: string;
    }

    interface Props {
        items: DropdownItem[];
        onSelect: (item: DropdownItem) => void;
        onClose: () => void;
    }

    let { items, onSelect, onClose }: Props = $props();
    let activeIndex = $state(0);

    export function handleKey(event: KeyboardEvent): boolean {
        if (event.key === "ArrowDown") {
            activeIndex = Math.min(activeIndex + 1, items.length - 1);
            return true;
        }
        if (event.key === "ArrowUp") {
            activeIndex = Math.max(activeIndex - 1, 0);
            return true;
        }
        if (event.key === "Enter" || event.key === "Tab") {
            if (items[activeIndex]) onSelect(items[activeIndex]);
            return true;
        }
        if (event.key === "Escape") {
            onClose();
            return true;
        }
        return false;
    }

    $effect(() => { activeIndex = 0; });
</script>

<div
    class="z-50 min-w-[200px] max-w-[280px] rounded-md border border-border bg-popover shadow-lg overflow-hidden"
    role="listbox"
>
    {#if items.length === 0}
        <div class="px-3 py-2 text-xs text-muted-foreground">No matches</div>
    {:else}
        {#each items as item, i}
            <button
                class="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left hover:bg-accent {i === activeIndex ? 'bg-accent' : ''}"
                onclick={() => onSelect(item)}
                role="option"
                aria-selected={i === activeIndex}
            >
                {#if item.type === "file"}
                    <IconFileText size={13} class="shrink-0 text-blue-400" />
                {:else if item.type === "table"}
                    <IconTable size={13} class="shrink-0 text-purple-400" />
                {:else}
                    <IconListSearch size={13} class="shrink-0 text-green-400" />
                {/if}
                <span class="truncate text-foreground">{item.label}</span>
                {#if item.sublabel}
                    <span class="ml-auto shrink-0 text-muted-foreground text-[10px]">{item.sublabel}</span>
                {/if}
            </button>
        {/each}
    {/if}
</div>
