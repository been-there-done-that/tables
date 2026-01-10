<script lang="ts">
    import { IconFilter, IconArrowsSort } from "@tabler/icons-svelte";
    import type { SuggestionItem } from "./types";

    interface Props {
        value?: string;
        placeholder?: string;
        suggestions?: (string | SuggestionItem)[];
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
    let listRef: HTMLDivElement | null = $state(null);
    let showSuggestions = $state(false);
    let highlightedIndex = $state(-1);
    let inputFocused = $state(false);

    // Normalize suggestions to SuggestionItem[]
    const normalizedSuggestions = $derived(
        suggestions.map((s) => {
            if (typeof s === "string") {
                return { value: s, type: undefined };
            }
            return s;
        }),
    );

    // Get the current word being typed (after last space or operator)
    const currentWord = $derived(() => {
        const parts = value.split(/[\s=><,()]+/);
        return parts[parts.length - 1]?.toLowerCase() || "";
    });

    // Filter suggestions based on current word
    const filteredSuggestions = $derived(() => {
        const word = currentWord();
        const all = normalizedSuggestions;
        if (!word || word.length < 1) return all.slice(0, 10);
        return all
            .filter((s) => s.value.toLowerCase().includes(word))
            .slice(0, 10);
    });

    let containerRef: HTMLDivElement | null = $state(null);

    // Show suggestions when focused and have suggestions
    // We removed the effect that forced showSuggestions based on focus to allow manual closing via Escape

    function handleWindowMouseDown(e: MouseEvent) {
        if (
            showSuggestions &&
            containerRef &&
            !containerRef.contains(e.target as Node)
        ) {
            showSuggestions = false;
            highlightedIndex = -1;
        }
    }

    $effect(() => {
        if (showSuggestions && highlightedIndex >= 0 && listRef) {
            const item = listRef.children[highlightedIndex] as HTMLElement;
            if (item) {
                item.scrollIntoView({ block: "nearest" });
            }
        }
    });

    function handleInput(e: Event) {
        const target = e.target as HTMLInputElement;
        value = target.value;
        onchange?.(value);
        if (filteredSuggestions().length > 0) {
            showSuggestions = true;
            highlightedIndex = 0;
        } else {
            showSuggestions = false;
        }
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
                selectSuggestion(suggestions[highlightedIndex].value);
            } else {
                // Submit on Enter if no valid suggestion selected
                onsubmit?.();
            }
        } else if (e.key === "Escape") {
            showSuggestions = false;
            highlightedIndex = -1;
        } else if (e.key === "Tab" && highlightedIndex >= 0) {
            e.preventDefault();
            selectSuggestion(suggestions[highlightedIndex].value);
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
        if (filteredSuggestions().length > 0) {
            showSuggestions = true;
            highlightedIndex = 0;
        }
    }

    function handleBlur() {
        // We handle closing via window click now for robustness
        inputFocused = false;
    }
</script>

<svelte:window onmousedown={handleWindowMouseDown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
    bind:this={containerRef}
    class="relative flex items-center gap-1 {widthClass} min-w-0 cursor-text group"
>
    <div
        class="flex items-center gap-1 text-muted-foreground shrink-0 select-none"
    >
        {#if icon === "filter"}
            <IconFilter class="size-4" />
        {:else}
            <IconArrowsSort class="size-4" />
        {/if}
        <span class="font-semibold uppercase tracking-wider">
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
            spellcheck="false"
            autocorrect="off"
            autocapitalize="off"
            oninput={handleInput}
            onkeydown={handleKeyDown}
            onfocus={handleFocus}
            onblur={handleBlur}
            onclick={(e) => {
                e.stopPropagation();
                if (filteredSuggestions().length > 0) {
                    showSuggestions = true;
                    if (highlightedIndex === -1) highlightedIndex = 0;
                }
            }}
        />

        {#if showSuggestions}
            <div
                bind:this={listRef}
                class="absolute left-0 top-full mt-1 z-50 w-full min-w-[180px] max-h-[200px] overflow-auto border border-(--theme-border-default) bg-(--theme-bg-secondary) rounded-md shadow-lg backdrop-blur-none"
            >
                {#each filteredSuggestions() as suggestion, idx}
                    <button
                        class="w-full text-left px-2.5 py-1.5 text-xs cursor-pointer transition-colors flex items-center justify-between gap-4 {idx ===
                        highlightedIndex
                            ? 'bg-accent text-accent-foreground'
                            : 'text-foreground hover:bg-accent/10'}"
                        onmouseenter={() => (highlightedIndex = idx)}
                        onmousedown={(e) => {
                            e.preventDefault(); // Prevent blur
                            selectSuggestion(suggestion.value);
                        }}
                    >
                        <span>{suggestion.value}</span>
                        {#if suggestion.type}
                            <span
                                class="opacity-50 text-[10px] font-mono lowercase"
                                >{suggestion.type}</span
                            >
                        {/if}
                    </button>
                {/each}
            </div>
        {/if}
    </div>
</div>
