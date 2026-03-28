<script lang="ts">
    import PopoverShell from "./PopoverShell.svelte";
    import {
        DEFAULT_TOKEN,
        NULL_TOKEN,
        normalizeIncomingBoolean,
        displayBooleanValue,
    } from "./valueUtils";
    import { cn } from "$lib/utils";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    const options = [true, false, NULL_TOKEN, DEFAULT_TOKEN];
    let selectedIndex = $state(0);
    const originalValue = $derived(normalizeIncomingBoolean(value));

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
        const normalized = normalizeIncomingBoolean(value);
        if (normalized === lastSyncedValue) return;
        lastSyncedValue = normalized;
        const idx = options.findIndex((opt) => opt === normalized);
        if (idx !== -1) {
            selectedIndex = idx;
        }
    });

    function handleSelect(newValue: any) {
        onCommit(newValue);
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "ArrowDown") {
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % options.length;
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            selectedIndex =
                (selectedIndex - 1 + options.length) % options.length;
        } else if (e.key === "Enter") {
            e.preventDefault();
            handleSelect(options[selectedIndex]);
        } else if (e.key === "Tab") {
            e.preventDefault();
            const dir = e.shiftKey ? -1 : 1;
            selectedIndex =
                (selectedIndex + dir + options.length) % options.length;
        }
    }
</script>

<PopoverShell {anchorEl} {onCancel} minWidth={160} maxWidth={200}>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="flex flex-col gap-1 p-1" role="group" onkeydown={handleKeydown}>
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
                        normalizeIncomingBoolean(option as any) ===
                            originalValue
                            ? "bg-accent"
                            : "invisible",
                    )}
                    aria-hidden="true"
                ></span>
                {displayBooleanValue(option as any)}
            </button>
        {/each}
    </div>
</PopoverShell>
