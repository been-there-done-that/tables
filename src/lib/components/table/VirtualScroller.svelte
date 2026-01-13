<script lang="ts">
    import { onMount } from "svelte";

    interface Props {
        items: any[];
        itemHeight: number;
        class?: string;
        children: import("svelte").Snippet<[any, number]>;
        header: import("svelte").Snippet;
        onScroll?: (e: Event) => void;
    }

    let {
        items,
        itemHeight,
        class: className,
        children,
        header,
        onScroll,
    }: Props = $props();

    let container: HTMLDivElement;
    let scrollTop = $state(0);
    let containerHeight = $state(0);
    let isScrolling = $state(false);
    let scrollTimeout: ReturnType<typeof setTimeout> | null = null;

    // Add buffer to prevent blank states during fast scrolling
    const BUFFER_SIZE = 20;

    let totalHeight = $derived(items.length * itemHeight);
    let startIndex = $derived(
        Math.max(0, Math.floor(scrollTop / itemHeight) - BUFFER_SIZE),
    );
    let endIndex = $derived(
        Math.min(
            items.length,
            Math.ceil((scrollTop + containerHeight) / itemHeight) + BUFFER_SIZE,
        ),
    );
    let visibleItems = $derived(items.slice(startIndex, endIndex));
    let offsetY = $derived(startIndex * itemHeight);

    function handleScroll(e: Event) {
        const target = e.target as HTMLDivElement;
        scrollTop = target.scrollTop;

        // Set scrolling state to disable pointer events during scroll
        isScrolling = true;
        if (scrollTimeout) clearTimeout(scrollTimeout);
        scrollTimeout = setTimeout(() => {
            isScrolling = false;
        }, 150); // Reset after 150ms of no scrolling

        onScroll?.(e);
    }

    export function getScrollTop() {
        return scrollTop;
    }

    export function getContainer() {
        return container;
    }

    export function scrollToIndex(
        index: number,
        align: "start" | "end" | "center" | "auto" = "auto",
    ) {
        if (!container) return;

        const targetTop = index * itemHeight;
        const visibleHeight = container.clientHeight;

        if (align === "auto") {
            if (targetTop < scrollTop) {
                container.scrollTop = targetTop;
            } else if (targetTop + itemHeight > scrollTop + visibleHeight) {
                container.scrollTop = targetTop + itemHeight - visibleHeight;
            }
        } else if (align === "start") {
            container.scrollTop = targetTop;
        } else if (align === "end") {
            container.scrollTop = targetTop + itemHeight - visibleHeight;
        } else if (align === "center") {
            container.scrollTop =
                targetTop - visibleHeight / 2 + itemHeight / 2;
        }
    }

    export function scrollToLeft(left: number) {
        if (container) {
            container.scrollLeft = left;
        }
    }

    onMount(() => {
        if (container) {
            containerHeight = container.clientHeight;
            const resizeObserver = new ResizeObserver(() => {
                containerHeight = container.clientHeight;
            });
            resizeObserver.observe(container);
            return () => {
                resizeObserver.disconnect();
                if (scrollTimeout) clearTimeout(scrollTimeout);
            };
        }
    });

    $effect(() => {
        if (containerHeight === 0 && items.length > 0) {
            // console.warn(
            //     "[VirtualScroller] Container height is 0, items might be invisible",
            // );
        }
    });
</script>

<div
    bind:this={container}
    class={className}
    style="overflow: auto; height: 100%;"
    onscroll={handleScroll}
>
    {#if header}
        <div class="sticky top-0 z-10 w-fit min-w-full bg-surface">
            {@render header()}
        </div>
    {/if}
    <div
        class="virtual-scroll-content"
        style="height: {totalHeight}px; width: fit-content; min-width: 100%; position: relative;"
    >
        <!-- Use absolute top positioning instead of transform for smoother rendering -->
        <div
            class="virtual-scroll-viewport"
            class:scrolling={isScrolling}
            style="position: absolute; width: fit-content; min-width: 100%; top: {offsetY}px;"
        >
            {#each visibleItems as item, i (item._rowId ?? `row-${startIndex + i}`)}
                {@render children(item, i + startIndex)}
            {/each}
        </div>
    </div>
</div>

<style>
    .virtual-scroll-viewport {
        /* Optimize rendering with backface-visibility */
        backface-visibility: hidden;
    }

    /* Disable pointer events during scroll to prevent repaints from hover states */
    .virtual-scroll-viewport.scrolling {
        pointer-events: none;
    }

    .virtual-scroll-content {
        /* Prevent subpixel rendering issues */
        backface-visibility: hidden;
    }
</style>
