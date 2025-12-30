<script lang="ts">
    import { onMount } from "svelte";

    interface Props {
        items: any[];
        itemHeight: number;
        class?: string;
        children: import("svelte").Snippet<[any, number]>;
        onScroll?: (e: Event) => void;
    }

    let {
        items,
        itemHeight,
        class: className,
        children,
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
    <div
        style="height: {totalHeight}px; position: relative; min-width: fit-content;"
    >
        <div
            style="transform: translateY({offsetY}px); position: absolute; top: 0; left: 0; right: 0;"
        >
            {#each visibleItems as item, i (item._rowId || startIndex + i)}
                {@render children(item, startIndex + i)}
            {/each}
        </div>
    </div>
</div>
