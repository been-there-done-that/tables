<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";
    import { portal } from "$lib/actions/portal";
    import { focusTrap } from "$lib/actions/focus-trap";

    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconChevronUp from "@tabler/icons-svelte/icons/chevron-up";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";

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
    let inputValue = $state("");
    let placement = $state<"left" | "right">("right");
    let arrowOffset = $state(0);

    $effect(() => {
        inputValue = (value ?? "").toString();
    });

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    const originalString = $derived((value ?? "").toString());

    // Get table container reference for boundary detection
    const containerGetter = getContext<
        (() => HTMLElement | null | undefined) | undefined
    >("table-container");

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }
        const rect = anchorEl.getBoundingClientRect();
        const width = Math.max(rect.width + 60, position.width);
        const overlayHeight = overlayEl?.offsetHeight ?? 120;
        const margin = 8;
        const headerHeight = 36;

        const container = containerGetter?.();
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
    use:focusTrap
    bind:this={overlayEl}
    data-placement={placement}
    role="dialog"
    aria-label="Edit number value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "popover-editor fixed bg-surface border border-accent/20 rounded-lg flex flex-col p-1 shadow-[0_10px_40px_-10px_rgba(0,0,0,0.5)]",
        "ring-1 ring-accent/10",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:280px;transform-origin:center;z-index:1000;--arrow-top:${arrowOffset}px`}
    aria-hidden={!isVisible}
>
    <div class="flex flex-col gap-1">
        <input
            type="number"
            inputmode="decimal"
            class="w-full rounded-md border border-accent/10 px-2 py-1.5 text-sm bg-background text-foreground focus:outline-none focus:ring-1 focus:ring-accent/10 focus:border-accent/10 transition-all [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
            bind:value={inputValue}
            placeholder="0"
        />

        <div class="flex items-center justify-center gap-2 px-1 pb-0.5">
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn"
                onclick={onCancel}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-black/5 dark:bg-white/5 border border-black/5 dark:border-white/5 text-foreground-muted/60"
                    >Esc</span
                >
                <IconX
                    class="size-3.5 opacity-60 group-hover/btn:opacity-100"
                />
            </button>

            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn"
                onclick={commit}
            >
                <span
                    class="text-[9px] font-medium px-1 rounded bg-accent/10 border border-accent/20 text-accent/80"
                    >{isMac ? "⌘↵" : "Ctrl↵"}</span
                >
                <IconCheck
                    class="size-3.5 opacity-80 group-hover/btn:opacity-100"
                />
            </button>

            <div class="flex items-center gap-1">
                <button
                    type="button"
                    class="p-1 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted/60 hover:text-foreground-muted transition-colors active:scale-90"
                    onclick={() => {
                        const val = parseFloat(inputValue || "0");
                        inputValue = (val - 1).toString();
                    }}
                    title="Decrease"
                >
                    <IconChevronDown class="size-3.5" />
                </button>
                <button
                    type="button"
                    class="p-1 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted/60 hover:text-foreground-muted transition-colors active:scale-90"
                    onclick={() => {
                        const val = parseFloat(inputValue || "0");
                        inputValue = (val + 1).toString();
                    }}
                    title="Increase"
                >
                    <IconChevronUp class="size-3.5" />
                </button>
            </div>
        </div>
    </div>
</div>
