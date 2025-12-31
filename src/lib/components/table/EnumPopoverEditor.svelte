<script lang="ts">
    import { getContext, onMount } from "svelte";
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
    let overlayEl: HTMLElement | null = null;
    let position = $state({ top: 0, left: 0, width: 140 });
    let isVisible = $state(false);

    const GUTTER = 4; // consistent spacing from the cell on every side
    const originalValue = $derived(value);

    function focusButton(idx: number) {
        if (!overlayEl) return;
        const buttons = overlayEl.querySelectorAll("button");
        buttons[idx]?.focus();
    }

    // Move node to document.body to avoid clipping by ancestors.
    function portal(node: HTMLElement) {
        if (typeof document === "undefined") return {};
        const target = document.body;
        target.appendChild(node);
        return {
            destroy() {
                if (node.parentNode === target) target.removeChild(node);
            },
        };
    }

    const getContainer: (() => HTMLElement | null | undefined) | undefined =
        getContext("table-container");
    let containerEl: HTMLElement | null | undefined;

    let lastSyncedValue: any;
    $effect(() => {
        const idx = options.findIndex((opt) => opt === value);
        if (idx === -1) return;
        if (options[idx] === lastSyncedValue) return;
        lastSyncedValue = options[idx];
        selectedIndex = idx;
        queueMicrotask(() => focusButton(idx));
    });

    // keep focus on the currently selected option when visible
    $effect(() => {
        if (!isVisible || options.length === 0) return;
        queueMicrotask(() => focusButton(selectedIndex));
    });

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = Math.max(rect.width, 100);
        const overlayHeight = overlayEl?.offsetHeight ?? rect.height;
        const margin = GUTTER;

        // Prefer positioning to the right of the cell; fall back left if needed.
        let left = rect.right + margin;
        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
        }
        left = Math.max(
            margin,
            Math.min(left, window.innerWidth - width - margin),
        );

        // Align vertically with cell center while keeping within viewport using the same gutter.
        let top = rect.top + rect.height / 2 - overlayHeight / 2;
        const minTop = margin;
        const maxTop = window.innerHeight - overlayHeight - margin;
        top = Math.max(minTop, Math.min(top, maxTop));

        position = { top, left, width };
    }

    function handleSelect(newValue: any) {
        onCommit(newValue);
    }

    function handleClickOutside(event: MouseEvent) {
        const target = event.target as Node;
        if (overlayEl?.contains(target)) return;
        if (anchorEl?.contains(target)) return;
        onCancel();
    }

    function handleKeydown(e: KeyboardEvent) {
        e.stopPropagation();
        if (!overlayEl) return;
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
        } else if (e.key === "ArrowDown") {
            if (!options.length) return;
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % options.length;
            focusButton(selectedIndex);
        } else if (e.key === "ArrowUp") {
            if (!options.length) return;
            e.preventDefault();
            selectedIndex =
                (selectedIndex - 1 + options.length) % options.length;
            focusButton(selectedIndex);
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
            focusButton(selectedIndex);
        }
    }

    onMount(() => {
        requestAnimationFrame(updatePosition);
        const handleUpdate = () => requestAnimationFrame(updatePosition);
        window.addEventListener("resize", handleUpdate);
        window.addEventListener("scroll", handleUpdate, true);
        document.addEventListener("mousedown", handleClickOutside);
        containerEl = getContainer?.();
        containerEl?.addEventListener("scroll", handleUpdate, {
            passive: true,
        });

        queueMicrotask(() => {
            overlayEl?.focus();
            if (options.length) overlayEl?.querySelector("button")?.focus();
            isVisible = true;
        });

        return () => {
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("scroll", handleUpdate, true);
            document.removeEventListener("mousedown", handleClickOutside);
            containerEl?.removeEventListener("scroll", handleUpdate);
        };
    });
</script>

<div
    use:portal
    bind:this={overlayEl}
    role="menu"
    aria-label="Select value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "fixed bg-surface border border-border-focus rounded-md shadow-2xl",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;transform-origin:center;z-index:1000`}
    aria-hidden={!isVisible}
>
    <div class="flex flex-col gap-1 p-1">
        {#each options as option, i}
            <button
                type="button"
                role="menuitemradio"
                aria-checked={selectedIndex === i}
                tabindex={selectedIndex === i ? 0 : -1}
                class={cn(
                    "pl-2 py-1 text-sm rounded text-left transition-colors flex items-center gap-1",
                    selectedIndex === i
                        ? "bg-active text-foreground"
                        : "hover:bg-muted hover:text-foreground",
                )}
                onclick={() => handleSelect(option)}
                onmouseenter={() => (selectedIndex = i)}
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
</div>
