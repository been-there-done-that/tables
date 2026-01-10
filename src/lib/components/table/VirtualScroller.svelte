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

    // Add buffer to prevent blank states during fast scrolling
    const BUFFER_SIZE = 5; // Render 5 extra items above and below

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
            return () => resizeObserver.disconnect();
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
        <div class="sticky top-0 z-10 w-fit min-w-full">
            {@render header()}
        </div>
    {/if}
    <div style="height: {totalHeight}px; width: 100%; position: relative;">
        <div style="position: absolute; top: {offsetY}px; width: 100%;">
            {#each visibleItems as item, i (item._rowId || i + startIndex)}
                {@render children(item, i + startIndex)}
            {/each}
        </div>
    </div>
</div>
