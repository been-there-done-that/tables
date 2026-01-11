<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { cn } from "$lib/utils";
    import { portal } from "$lib/actions/portal";
    import { focusTrap } from "$lib/actions/focus-trap";

    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

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
    let placement = $state<"left" | "right">("right");
    let arrowOffset = $state(0);

    const isMac =
        typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

    const originalString = $derived((value ?? "").toString());

    $effect(() => {
        inputValue = (value ?? "").toString();
    });

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
        placement = "right";

        const fitsRight = left + width + margin <= window.innerWidth;
        if (!fitsRight) {
            left = rect.left - width - margin;
            placement = "left";
        }
        left = Math.max(
            margin,
            Math.min(left, window.innerWidth - width - margin),
        );

        let top = rect.top + rect.height / 2 - overlayHeight / 2;
        const minTop = margin;
        const maxTop = window.innerHeight - overlayHeight - margin;
        top = Math.max(minTop, Math.min(top, maxTop));

        // Calculate arrow vertical offset with clamping to avoid corners
        const anchorCenterY = rect.top + rect.height / 2;
        const minArrow = 16;
        const maxArrow = overlayHeight - 16;
        arrowOffset = Math.max(
            minArrow,
            Math.min(anchorCenterY - top, maxArrow),
        );

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
    use:focusTrap
    bind:this={overlayEl}
    data-placement={placement}
    role="dialog"
    aria-label="Edit value"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "popover-editor fixed rounded-lg shadow-[0_10px_40px_-10px_rgba(0,0,0,0.5)] flex flex-col p-1",
        "bg-surface border border-accent/20 ring-1 ring-accent/10",
        isVisible ? "anim-pop opacity-100" : "opacity-0 pointer-events-none",
    )}
    style={`top:${position.top}px;left:${position.left}px;min-width:${position.width}px;max-width:400px;transform-origin:center;z-index:1000;--arrow-top:${arrowOffset}px`}
    aria-hidden={!isVisible}
>
    <div class="relative flex flex-col group">
        <textarea
            class="w-full rounded-md border border-accent/5 text-[13px] bg-background text-foreground min-h-[110px] resize-y p-1.5 pb-6 focus:ring-1 focus:ring-accent/10 focus:border-accent/10 focus:outline-none transition-all placeholder:text-foreground-muted/20"
            bind:value={inputValue}
            rows={4}
            placeholder="Edit text..."
        ></textarea>

        <div
            class="absolute bottom-1 left-0 right-0 flex items-center justify-center gap-2 pointer-events-none"
        >
            <button
                type="button"
                class="flex items-center gap-1.5 px-2 py-0.5 rounded border border-transparent hover:border-accent/10 hover:bg-muted text-foreground-muted transition-colors active:scale-95 group/btn pointer-events-auto"
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
                class="flex items-center gap-1.5 px-2 py-0.5 rounded text-accent border border-transparent hover:border-accent/10 hover:bg-accent/10 transition-colors active:scale-95 group/btn pointer-events-auto"
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
        </div>
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

    .popover-editor::before,
    .popover-editor::after {
        content: "";
        position: absolute;
        width: 10px;
        height: 10px;
        transform: rotate(45deg);
        top: var(--arrow-top);
        margin-top: -5px;
        pointer-events: none;
    }

    /* Border Layer (Match border-accent/20 ring-1 ring-accent/10) */
    .popover-editor::before {
        background: var(--theme-accent-primary);
        opacity: 0.25;
        z-index: 0;
    }

    /* Fill Layer (Match bg-surface / bg-secondary) */
    .popover-editor::after {
        background: var(--theme-bg-secondary);
        z-index: 1;
    }

    /* Right Side Placement (Arrow on left of popover) */
    .popover-editor[data-placement="right"]::before {
        left: -6px;
    }
    .popover-editor[data-placement="right"]::after {
        left: -5px;
    }

    /* Left Side Placement (Arrow on right of popover) */
    .popover-editor[data-placement="left"]::before {
        right: -6px;
    }
    .popover-editor[data-placement="left"]::after {
        right: -5px;
    }
</style>
