<script lang="ts">
    import { IconFilter, IconArrowsSort } from "@tabler/icons-svelte";
    import * as Popover from "$lib/components/ui/popover";

    interface Props {
        value?: string;
        placeholder?: string;
        suggestions?: string[];
        icon?: "filter" | "sort";
        widthClass?: string;
        onchange?: (value: string) => void;
        onsubmit?: () => void;
    }

    let {
        value = $bindable(""),
        placeholder = "",
        suggestions = [],
        icon = "filter",
        widthClass = "flex-1",
        onchange,
        onsubmit,
    }: Props = $props();

    let inputRef: HTMLInputElement | null = $state(null);
    let showSuggestions = $state(false);
    let highlightedIndex = $state(-1);
    let inputFocused = $state(false);

    // Get the current word being typed (after last space or operator)
    const currentWord = $derived(() => {
        const parts = value.split(/[\s=><,()]+/);
        return parts[parts.length - 1]?.toLowerCase() || "";
    });

    // Filter suggestions based on current word
    const filteredSuggestions = $derived(() => {
        const word = currentWord();
        if (!word || word.length < 1) return suggestions.slice(0, 10);
        return suggestions
            .filter((s) => s.toLowerCase().includes(word))
            .slice(0, 10);
    });

    // Show suggestions when focused and have suggestions
    $effect(() => {
        showSuggestions = inputFocused && filteredSuggestions().length > 0;
    });

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        value = target.value;
        onchange?.(value);
        highlightedIndex = -1;
    }

    function handleKeyDown(e: KeyboardEvent) {
        const suggestions = filteredSuggestions();

        if (e.key === "ArrowDown") {
            e.preventDefault();
            highlightedIndex = Math.min(
                highlightedIndex + 1,
                suggestions.length - 1,
            );
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            highlightedIndex = Math.max(highlightedIndex - 1, -1);
        } else if (e.key === "Enter") {
            if (highlightedIndex >= 0 && suggestions[highlightedIndex]) {
                e.preventDefault();
                selectSuggestion(suggestions[highlightedIndex]);
            } else {
                // Submit on Enter if no suggestion selected, or if meta/ctrl held
                onsubmit?.();
            }
        } else if (e.key === "Escape") {
            showSuggestions = false;
            highlightedIndex = -1;
        } else if (e.key === "Tab" && highlightedIndex >= 0) {
            e.preventDefault();
            selectSuggestion(suggestions[highlightedIndex]);
        }
    }

    function selectSuggestion(suggestion: string) {
        // Replace the current word with the suggestion
        const parts = value.split(/(\s+|[=><,()]+)/);
        parts[parts.length - 1] = suggestion;
        value = parts.join("");
        onchange?.(value);
        showSuggestions = false;
        highlightedIndex = -1;
        inputRef?.focus();
    }

    function handleFocus() {
        inputFocused = true;
    }

    function handleBlur() {
        // Delay to allow click on suggestion
        setTimeout(() => {
            inputFocused = false;
        }, 150);
    }
</script>

<div class="relative flex items-center gap-1.5 {widthClass} min-w-0">
    <div class="flex items-center gap-1 text-muted-foreground shrink-0">
        {#if icon === "filter"}
            <IconFilter class="h-3.5 w-3.5" />
        {:else}
            <IconArrowsSort class="h-3.5 w-3.5" />
        {/if}
        <span class="text-[10px] font-semibold uppercase tracking-wider">
            {icon === "filter" ? "WHERE" : "ORDER BY"}
        </span>
    </div>

    <div class="relative flex-1 min-w-0">
        <input
            bind:this={inputRef}
            type="text"
            class="w-full h-6 px-2 text-sm bg-transparent border-0 border-b border-transparent placeholder:text-muted-foreground/50 transition-colors focus:outline-none"
            {placeholder}
            {value}
            autocomplete="off"
            oninput={handleInput}
            onkeydown={handleKeyDown}
            onfocus={handleFocus}
            onblur={handleBlur}
        />

        {#if showSuggestions}
            <div
                class="absolute left-0 top-full mt-1 z-50 w-full min-w-[180px] max-h-[200px] overflow-auto bg-popover border border-border rounded-md shadow-lg backdrop-blur-none"
                style="background-color: hsl(var(--popover));"
            >
                {#each filteredSuggestions() as suggestion, idx}
                    <button
                        class="w-full text-left px-2.5 py-1.5 text-xs hover:bg-muted cursor-pointer transition-colors {idx ===
                        highlightedIndex
                            ? 'bg-muted'
                            : ''}"
                        onmousedown={() => selectSuggestion(suggestion)}
                    >
                        {suggestion}
                    </button>
                {/each}
            </div>
        {/if}
    </div>
</div>
