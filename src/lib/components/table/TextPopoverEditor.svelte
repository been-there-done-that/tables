<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";

    interface Props {
        value: any;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, anchorEl, onCommit, onCancel }: Props = $props();

    let overlayEl: HTMLElement | null = null;
    let position = $state({ top: 0, left: 0, width: 260 });
    let isVisible = $state(false);
    let inputValue = $derived((value ?? "").toString());

    const originalString = $derived((value ?? "").toString());

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

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = Math.max(rect.width + 60, position.width);
        const overlayHeight = overlayEl?.offsetHeight ?? 120;
        const margin = 4;

        let left = rect.right + margin;
        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
        }
        left = Math.max(margin, Math.min(left, window.innerWidth - width - margin));

        let top = rect.top + rect.height / 2 - overlayHeight / 2;
        const minTop = margin;
        const maxTop = window.innerHeight - overlayHeight - margin;
        top = Math.max(minTop, Math.min(top, maxTop));

        position = { top, left, width };
    }

    function parseValue(val: string) {
        return val;
    }

    function handleKeydown(e: KeyboardEvent) {
        e.stopPropagation();
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
            return;
        }

        const isCmdEnter = (e.metaKey || e.ctrlKey) && e.key === "Enter";
        if (isCmdEnter) {
            e.preventDefault();
            commit();
        }
    }

    function commit() {
        const parsed = parseValue(inputValue);
        const unchanged = inputValue.toString() === originalString;
        if (unchanged) {
            onCancel();
            return;
        }
        onCommit(parsed);
    }

    onMount(() => {
        requestAnimationFrame(updatePosition);
        const handleUpdate = () => requestAnimationFrame(updatePosition);
        window.addEventListener("resize", handleUpdate);
        window.addEventListener("scroll", handleUpdate, true);
        const containerGetter: (() => HTMLElement | null | undefined) | undefined =
            getContext("table-container");
        const containerEl = containerGetter?.();
        containerEl?.addEventListener("scroll", handleUpdate, { passive: true });
        document.addEventListener("mousedown", handleClickOutside);

        queueMicrotask(() => {
            overlayEl?.focus();
            const first = overlayEl?.querySelector("input,textarea");
            if (first instanceof HTMLInputElement || first instanceof HTMLTextAreaElement) {
                first.focus();
            }
            isVisible = true;
        });

        return () => {
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("scroll", handleUpdate, true);
            containerEl?.removeEventListener("scroll", handleUpdate);
            document.removeEventListener("mousedown", handleClickOutside);
        };
    });

    function handleClickOutside(event: MouseEvent) {
        const target = event.target as Node;
        if (overlayEl?.contains(target)) return;
        if (anchorEl?.contains(target)) return;
        onCancel();
    }
</script>

<div
    use:portal
    bind:this={overlayEl}
    role="dialog"
    aria-label="Edit value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "fixed z-1000 bg-popover border rounded-md border-blue-700 flex flex-col p-1",
        isVisible ? "popoverpop" : "opacity-0 pointer-events-none"
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:320px;transform-origin:center`}
    aria-hidden={!isVisible}
>
    <div class="flex flex-col gap-2">
        <textarea
            class="w-full rounded border text-sm bg-background min-h-[120px] resize-none px-2 py-1"
            bind:value={inputValue}
            rows={4}
        ></textarea>
        <div class="flex items-center justify-between text-xs text-muted-foreground">
            <span>Cmd/Ctrl+Enter to save · Esc to cancel</span>
            <span class="text-[10px] uppercase tracking-wide">text</span>
        </div>
    </div>
    <div class="flex items-center justify-end border-t px-2 py-1 gap-2 bg-popover">
        <button
            type="button"
            class="px-2 py-1 text-sm rounded bg-secondary text-secondary-foreground hover:bg-secondary/80 transition"
            onclick={onCancel}
        >
            Cancel
        </button>
        <button
            type="button"
            class="px-2 py-1 text-sm rounded bg-primary text-primary-foreground hover:bg-primary/90 transition"
            onclick={commit}
        >
            Save
        </button>
    </div>
</div>
