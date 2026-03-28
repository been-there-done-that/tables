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
            event.preventDefault();
            if (items[activeIndex]) onSelect(items[activeIndex]);
            return true;
        }
        if (event.key === "Escape") {
            onClose();
            return true;
        }
        return false;
    }

    $effect(() => {
        // Read items to track changes — reset active index whenever list is rebuilt
        void items.length;
        activeIndex = 0;
    });
</script>

<div
    class="z-50 min-w-[220px] max-w-[300px] rounded-md border border-border bg-popover shadow-xl overflow-hidden max-h-[220px] overflow-y-auto"
    role="listbox"
>
    {#each items as item, i}
        <button
            class="w-full flex items-center gap-2 px-2.5 py-1.5 text-[11px] text-left transition-colors {i === activeIndex ? 'bg-foreground/10 text-foreground' : 'text-foreground/80 hover:bg-foreground/5'}"
            onclick={() => onSelect(item)}
            role="option"
            aria-selected={i === activeIndex}
        >
            {#if item.type === "file"}
                <IconFileText size={12} class="shrink-0 text-blue-400 opacity-80" />
            {:else if item.type === "table"}
                <IconTable size={12} class="shrink-0 text-purple-400 opacity-80" />
            {:else}
                <IconListSearch size={12} class="shrink-0 text-green-400 opacity-80" />
            {/if}
            <span class="truncate font-medium">{item.label}</span>
            {#if item.sublabel}
                <span class="ml-auto shrink-0 text-muted-foreground/60 text-[10px]">{item.sublabel}</span>
            {/if}
        </button>
    {/each}
</div>
