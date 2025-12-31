<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";

    import { IconCheck, IconX } from "@tabler/icons-svelte";

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
    let inputValue = $state("");

    const originalString = $derived((value ?? "").toString());

    $effect(() => {
        inputValue = (value ?? "").toString();
    });

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
        const width = Math.max(rect.width + 100, position.width);
        const overlayHeight = overlayEl?.offsetHeight ?? 200;
        const margin = 8;

        let left = rect.right + margin;
        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
        }
        left = Math.max(
            margin,
            Math.min(left, window.innerWidth - width - margin),
        );

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
        const unchanged = inputValue === originalString;
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
        const containerGetter:
            | (() => HTMLElement | null | undefined)
            | undefined = getContext("table-container");
        const containerEl = containerGetter?.();
        containerEl?.addEventListener("scroll", handleUpdate, {
            passive: true,
        });
        document.addEventListener("mousedown", handleClickOutside);

        queueMicrotask(() => {
            overlayEl?.focus();
            const first = overlayEl?.querySelector("input,textarea");
            if (
                first instanceof HTMLInputElement ||
                first instanceof HTMLTextAreaElement
            ) {
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
        "fixed bg-surface border border-border-focus rounded-lg shadow-2xl flex flex-col p-3 gap-3",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:400px;transform-origin:center;z-index:1000`}
    aria-hidden={!isVisible}
>
    <div class="flex flex-col gap-2">
        <textarea
            class="w-full rounded-md border border-border text-sm bg-background text-foreground min-h-[160px] resize-y px-3 py-2 focus:outline-none focus:ring-1 focus:ring-border-focus font-mono"
            bind:value={inputValue}
            rows={6}
            placeholder="Type long text here..."
        ></textarea>
        <div
            class="flex items-center justify-between text-[10px] text-foreground-muted uppercase tracking-widest px-1 font-medium"
        >
            <span>Cmd+Enter to save • Esc to cancel</span>
            <span class="bg-tertiary px-1.5 py-0.5 rounded"
                >Long Text</span
            >
        </div>
    </div>
    <div class="flex items-center justify-end gap-2 pt-1">
        <button
            type="button"
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-tertiary text-foreground hover:bg-muted transition-all active:scale-95 border border-border"
            onclick={onCancel}
        >
            <IconX class="size-3.5" />
            Cancel
        </button>
        <button
            type="button"
            class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-accent text-accent-foreground hover:bg-accent-hover transition-all active:scale-95 shadow-sm"
            onclick={commit}
        >
            <IconCheck class="size-3.5" />
            Save Changes
        </button>
    </div>
</div>

<style>
    .anim-pop {
        animation: pop 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    }
    @keyframes pop {
        from {
            transform: scale(0.95) translateY(4px);
            opacity: 0;
        }
        to {
            transform: scale(1) translateY(0);
            opacity: 1;
        }
    }
</style>
