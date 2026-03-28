<script lang="ts">
    import { getContext, onMount, type Snippet } from "svelte";
    import { cn } from "$lib/utils";
    import { portal } from "$lib/actions/portal";
    import { focusTrap } from "$lib/actions/focus-trap";

    interface Props {
        anchorEl: HTMLElement | null | undefined;
        onCancel: () => void;
        children: Snippet;
        width?: number;
        minWidth?: number;
        maxWidth?: number;
        minHeight?: number;
        maxHeight?: number;
        className?: string;
    }

    let {
        anchorEl,
        onCancel,
        children,
        width,
        minWidth = 280,
        maxWidth = 340,
        minHeight = 40,
        maxHeight,
        className,
    }: Props = $props();

    let overlayEl = $state<HTMLElement | null>(null);
    // svelte-ignore state_referenced_locally
    let position = $state({ top: 0, left: 0, width: width ?? minWidth });
    let isVisible = $state(false);
    let placement = $state<"left" | "right">("right");
    let arrowOffset = $state(0);

    const containerGetter = getContext<
        (() => HTMLElement | null | undefined) | undefined
    >("table-container");

    function updatePosition() {
        if (!anchorEl || !anchorEl.isConnected) {
            onCancel();
            return;
        }

        const rect = anchorEl.getBoundingClientRect();
        const currentWidth = width ?? minWidth;
        const overlayHeight = overlayEl?.offsetHeight ?? 200;
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

        const fitsRight = left + currentWidth <= safeRight;
        if (!fitsRight) {
            left = rect.left - currentWidth - margin;
            placement = "left";
        }

        // Final horizontal clamp
        left = Math.max(safeLeft, Math.min(left, safeRight - currentWidth));

        let top = rect.top + rect.height / 2 - overlayHeight / 2;

        // Constrain top to be within safe area
        top = Math.max(safeTop, Math.min(top, safeBottom - overlayHeight));

        // Calculate arrow vertical offset
        const anchorCenterY = rect.top + rect.height / 2;
        const minArrow = 12;
        const maxArrow = overlayHeight - 12;
        arrowOffset = Math.max(
            minArrow,
            Math.min(anchorCenterY - top, maxArrow),
        );

        position = { top, left, width: currentWidth };
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onCancel();
        }
    }

    function handleClickOutside(event: MouseEvent) {
        const target = event.target as Node;
        if (overlayEl?.contains(target)) return;
        if (anchorEl?.contains(target)) return;
        // Don't close if clicking monaco overlays or portal-rendered components that aren't this popover
        if ((target as HTMLElement).closest?.(".monaco-aria-container")) return;
        onCancel();
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
            isVisible = true;
            // Focus internal elements if any
            const firstInput = overlayEl?.querySelector(
                "input, select, textarea, button",
            ) as HTMLElement | null;
            firstInput?.focus();
        });

        return () => {
            window.removeEventListener("resize", handleUpdate);
            window.removeEventListener("scroll", handleUpdate, true);
            containerEl?.removeEventListener("scroll", handleUpdate);
            document.removeEventListener("mousedown", handleClickOutside);
        };
    });
</script>

<div
    use:portal
    use:focusTrap
    bind:this={overlayEl}
    data-placement={placement}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onkeydown={handleKeydown}
    class={cn(
        "popover-editor fixed rounded-lg shadow-[0_12px_48px_-12px_rgba(0,0,0,0.6),0_0_0_1px_rgba(0,0,0,0.05)] flex flex-col p-1",
        "bg-surface border border-accent/40 ring-1 ring-accent/30 transition-opacity duration-150",
        isVisible ? "opacity-100 anim-pop" : "opacity-0 pointer-events-none",
        className,
    )}
    style:top="{position.top}px"
    style:left="{position.left}px"
    style:min-width="{minWidth}px"
    style:max-width="{maxWidth}px"
    style:min-height="{minHeight}px"
    style:max-height="{maxHeight}px"
    style:width={width ? width + "px" : "auto"}
    style:transform-origin="center"
    style:z-index="1000"
    style="--arrow-top: {arrowOffset}px"
>
    {@render children()}
</div>
