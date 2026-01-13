<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import { cn } from "$lib/utils";

    interface Props {
        value: any;
        options: string[];
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, options, anchorEl, onCommit, onCancel }: Props = $props();

    let selectedIndex = $state(0);
    const originalValue = $derived(value);

    function autoFocus(node: HTMLElement, focused: boolean) {
        if (focused) node.focus();
        return {
            update(newFocused: boolean) {
                if (newFocused) node.focus();
            },
        };
    }

    let lastSyncedValue: any;
    $effect(() => {
        const idx = options.findIndex((opt) => opt === value);
        if (idx === -1) return;
        if (options[idx] === lastSyncedValue) return;
        lastSyncedValue = options[idx];
        selectedIndex = idx;
    });

    function handleSelect(newValue: any) {
        onCommit(newValue);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "ArrowDown") {
            if (!options.length) return;
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % options.length;
        } else if (e.key === "ArrowUp") {
            if (!options.length) return;
            e.preventDefault();
            selectedIndex =
                (selectedIndex - 1 + options.length) % options.length;
        } else if (e.key === "Enter") {
            if (!options.length) return;
            e.preventDefault();
            handleSelect(options[selectedIndex]);
        } else if (e.key === "Tab") {
            if (!options.length) return;
            e.preventDefault();
            const dir = e.shiftKey ? -1 : 1;
            selectedIndex =
                (selectedIndex + dir + options.length) % options.length;
        }
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={140} maxWidth={280}>
    <div class="flex flex-col gap-1 p-1" onkeydown={handleKeydown}>
        {#each options as option, i}
            <button
                type="button"
                role="menuitemradio"
                aria-checked={selectedIndex === i}
                tabindex={selectedIndex === i ? 0 : -1}
                class={cn(
                    "pl-2 py-1 text-sm rounded-sm text-left transition-colors flex items-center gap-1 outline-none",
                    selectedIndex === i
                        ? "bg-accent/10 text-foreground"
                        : "hover:bg-accent/10 hover:text-foreground",
                )}
                onclick={() => handleSelect(option)}
                onmouseenter={() => (selectedIndex = i)}
                use:autoFocus={selectedIndex === i}
            >
                <span
                    class={cn(
                        "inline-block size-1 rounded-full mr-1",
                        option === originalValue ? "bg-accent" : "invisible",
                    )}
                    aria-hidden="true"
                ></span>
                {option}
            </button>
        {/each}
    </div>
</PopoverShell>
