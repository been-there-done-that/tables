<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { portal } from "$lib/actions/portal";
    import { focusTrap } from "$lib/actions/focus-trap";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    import {
        DEFAULT_TOKEN,
        NULL_TOKEN,
        normalizeIncomingBoolean,
        displayBooleanValue,
    } from "./valueUtils";
    import { cn } from "$lib/utils";

    const options = [true, false, NULL_TOKEN, DEFAULT_TOKEN];
    let selectedIndex = $state(0);
    let overlayEl: HTMLElement | null = null;
    let position = $state({ top: 0, left: 0, width: 160 });
    let isVisible = $state(false);
    let placement = $state<"left" | "right">("right");
    let arrowOffset = $state(0);

    const GUTTER = 4; // consistent spacing from the cell on every side
    const originalValue = $derived(normalizeIncomingBoolean(value));

    function focusButton(idx: number) {
        if (!overlayEl) return;
        const buttons = overlayEl.querySelectorAll("button");
        buttons[idx]?.focus();
    }

    const getContainer: (() => HTMLElement | null | undefined) | undefined =
        getContext("table-container");
    let containerEl: HTMLElement | null | undefined;

    let lastSyncedValue: any;
    $effect(() => {
        const normalized = normalizeIncomingBoolean(value);
        if (normalized === lastSyncedValue) return;
        lastSyncedValue = normalized;
        const idx = options.findIndex((opt) => opt === normalized);
        if (idx !== -1) {
            selectedIndex = idx;
            console.info("[BooleanPopover] sync-value", { value, idx });
            queueMicrotask(() => focusButton(idx));
        }
    });

    // keep focus on the currently selected option when visible
    $effect(() => {
        if (!isVisible) return;
        queueMicrotask(() => focusButton(selectedIndex));
    });

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = 160; // Standardized width for boolean
        const overlayHeight = overlayEl?.offsetHeight ?? 140;
        const margin = 8;
        const headerHeight = 36;

        const container = getContainer?.();
        const containerRect = container?.getBoundingClientRect();

        const safeTop = containerRect
            ? containerRect.top + headerHeight
            : headerHeight;
        const safeBottom = containerRect
            ? containerRect.bottom - margin
            : window.innerHeight - margin;
        const safeLeft = containerRect ? containerRect.left + margin : margin;
        const safeRight = containerRect
            ? containerRect.right - margin
            : window.innerWidth - margin;

        let left = rect.right + margin;
        placement = "right";

        const fitsRight = left + width <= safeRight;
        if (!fitsRight) {
            left = rect.left - width - margin;
            placement = "left";
        }

        // Final horizontal clamp
        left = Math.max(safeLeft, Math.min(left, safeRight - width));

        let top = rect.top + rect.height / 2 - overlayHeight / 2;

        // Constrain top to be within safe area
        top = Math.max(safeTop, Math.min(top, safeBottom - overlayHeight));

        // Calculate arrow vertical offset with clamping to avoid corners
        const anchorCenterY = rect.top + rect.height / 2;
        const minArrow = 12;
        const maxArrow = overlayHeight - 12;
        arrowOffset = Math.max(
            minArrow,
            Math.min(anchorCenterY - top, maxArrow),
        );

        position = { top, left, width };
    }

    function handleSelect(newValue: any) {
        onCommit(newValue);
        console.info("[BooleanPopover] select", { newValue });
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
        console.info("[BooleanPopover] keydown", { key: e.key, selectedIndex });
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
        } else if (e.key === "ArrowDown") {
            e.preventDefault();
            selectedIndex = (selectedIndex + 1) % options.length;
            focusButton(selectedIndex);
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            selectedIndex =
                (selectedIndex - 1 + options.length) % options.length;
            focusButton(selectedIndex);
        } else if (e.key === "Enter") {
            e.preventDefault();
            handleSelect(options[selectedIndex]);
        } else if (e.key === "Tab") {
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
            overlayEl?.querySelector("button")?.focus();
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
    use:focusTrap
    bind:this={overlayEl}
    data-placement={placement}
    role="dialog"
    aria-label="Select boolean value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "popover-editor fixed rounded-lg shadow-[0_10px_40px_-10px_rgba(0,0,0,0.5)] flex flex-col p-1",
        "bg-surface border border-accent/20 ring-1 ring-accent/10",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;transform-origin:center;z-index:1000;--arrow-top:${arrowOffset}px`}
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
</div>
