<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";

    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

    type NumberType = "int" | "float";

    interface Props {
        value: any;
        kind: NumberType;
        anchorEl?: HTMLElement | null;
        onCommit: (newValue: any) => void;
        onCancel: () => void;
    }

    let { value, kind, anchorEl, onCommit, onCancel }: Props = $props();

    let overlayEl: HTMLElement | null = null;
    let position = $state({ top: 0, left: 0, width: 220 });
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

    function parseNumber(val: string) {
        if (kind === "int") {
            const n = parseInt(val, 10);
            return isNaN(n) ? null : n;
        }
        const n = parseFloat(val);
        return isNaN(n) ? null : n;
    }

    function handleKeydown(e: KeyboardEvent) {
        e.stopPropagation();
        if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
            return;
        }
        const isCmdEnter = (e.metaKey || e.ctrlKey) && e.key === "Enter";
        const isPlainEnter = e.key === "Enter";
        if (isPlainEnter || isCmdEnter) {
            e.preventDefault();
            commit();
        }
    }

    function commit() {
        const parsed = parseNumber(inputValue);
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
            const first = overlayEl?.querySelector("input");
            if (first instanceof HTMLInputElement) {
                first.focus();
                first.select();
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
    aria-label="Edit number value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "fixed bg-surface border border-accent/10 rounded-lg flex flex-col p-0.5",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:280px;transform-origin:center;z-index:1000`}
    aria-hidden={!isVisible}
>
    <div class="relative flex flex-col group">
        <input
            type="number"
            inputmode="decimal"
            class="w-full rounded-md border border-accent/5 px-2 pt-1 pb-6 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-accent/10 focus:border-accent/10 transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
            bind:value={inputValue}
            placeholder="0"
        />

        <div
            class="absolute bottom-1 left-2 right-1.5 flex items-center justify-between pointer-events-none"
        >
            <span
                class="text-[9px] text-foreground-muted opacity-30 font-medium tracking-tight"
            >
                ↵ Save · Esc Cancel
            </span>
            <div class="flex items-center gap-0.5 pointer-events-auto">
                <button
                    type="button"
                    class="p-0.5 rounded hover:bg-muted text-foreground-muted transition-colors active:scale-95"
                    onclick={onCancel}
                >
                    <IconX class="size-3" />
                </button>
                <button
                    type="button"
                    class="p-0.5 rounded text-accent hover:bg-accent/10 transition-colors active:scale-95"
                    onclick={commit}
                >
                    <IconCheck class="size-3" />
                </button>
            </div>
        </div>
    </div>
</div>
