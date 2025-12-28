<script lang="ts">
    import { type Snippet } from "svelte";

    let {
        defaultRatio = 0.2,
        minLeft = "100px",
        minRight = "100px",
        gapSize = 1,
        resizable = true,
        orientation = "horizontal",
        resetOnDisable = false,
        left,
        right,
    }: {
        defaultRatio?: number;
        minLeft?: string;
        minRight?: string;
        gapSize?: number;
        resizable?: boolean;
        orientation?: "horizontal" | "vertical";
        resetOnDisable?: boolean;
        left?: Snippet;
        right?: Snippet;
    } = $props();

    let container = $state<HTMLElement>();
    let isDragging = $state(false);
    let currentRatio = $state(defaultRatio * 100);

    let isVertical = $derived(orientation === "vertical");

    function startDrag(event: MouseEvent) {
        if (!resizable) return;
        isDragging = true;
        document.body.style.cursor = isVertical ? "row-resize" : "col-resize";
        document.body.style.userSelect = "none";

        window.addEventListener("mousemove", onDrag);
        window.addEventListener("mouseup", stopDrag);
    }

    function onDrag(event: MouseEvent) {
        if (!isDragging || !container) return;

        const containerRect = container.getBoundingClientRect();

        let newSize = 0;
        let totalSize = 0;

        if (isVertical) {
            newSize = event.clientY - containerRect.top;
            totalSize = containerRect.height;
        } else {
            newSize = event.clientX - containerRect.left;
            totalSize = containerRect.width;
        }

        // Convert to percentage
        let newRatio = newSize / totalSize;

        // Parse constraints
        const parseLength = (val: string, total: number) => {
            if (val.endsWith("%")) {
                return (parseFloat(val) / 100) * total;
            }
            if (val.endsWith("px")) {
                return parseFloat(val);
            }
            return parseFloat(val) || 0;
        };

        const minStartPx = parseLength(minLeft, totalSize); // minLeft acts as minTop in vertical
        const minEndPx = parseLength(minRight, totalSize); // minRight acts as minBottom in vertical

        const minRatioVal = minStartPx / totalSize;
        const maxRatioVal = 1 - minEndPx / totalSize;

        // Apply clamping
        if (minRatioVal > maxRatioVal) {
            newRatio = 0.5;
        } else {
            newRatio = Math.max(minRatioVal, Math.min(newRatio, maxRatioVal));
        }

        currentRatio = newRatio * 100;
    }

    function stopDrag() {
        isDragging = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        window.removeEventListener("mousemove", onDrag);
        window.removeEventListener("mouseup", stopDrag);
    }

    // Reset logic effect
    $effect(() => {
        if (!resizable && resetOnDisable) {
            currentRatio = defaultRatio * 100;
        }
    });
</script>

<div
    bind:this={container}
    class="relative flex h-full w-full select-none overflow-hidden"
    class:flex-col={isVertical}
    class:flex-row={!isVertical}
    role="group"
>
    <!-- First Panel (Left/Top) -->
    <div
        class="relative z-0 flex-none overflow-auto transition-[flex-basis] duration-0 ease-linear"
        class:transition-all={!isDragging}
        class:duration-300={!isDragging}
        style="{isVertical ? 'height' : 'width'}: {currentRatio}%; {isVertical
            ? 'min-height'
            : 'min-width'}: {minLeft};"
    >
        {#if left}
            {@render left()}
        {:else}
            <div class="p-4 text-gray-400">Left Content</div>
        {/if}
    </div>

    <!-- Handle -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
        role="separator"
        tabindex="0"
        aria-valuenow={currentRatio}
        aria-valuemin={0}
        aria-valuemax={100}
        class="relative z-10 flex-none"
        class:w-full={isVertical}
        class:h-auto={isVertical}
        class:h-full={!isVertical}
        class:w-auto={!isVertical}
        style="{isVertical
            ? 'height'
            : 'width'}: {gapSize}px; cursor: {resizable
            ? isVertical
                ? 'row-resize'
                : 'col-resize'
            : 'default'};"
        onmousedown={startDrag}
    >
        <!-- Visual Line -->
        <div
            class="absolute inset-0 bg-(--theme-border-default)"
            class:h-px={isVertical}
            class:w-full={isVertical}
            class:w-px={!isVertical}
            class:h-full={!isVertical}
        ></div>

        <!-- Hit Area (Invisible, wider for easier grabbing) -->
        <div
            class="absolute z-20 bg-transparent"
            style={isVertical
                ? "top: -3px; bottom: -3px; left: 0; right: 0;"
                : "left: -3px; right: -3px; top: 0; bottom: 0;"}
        ></div>
    </div>

    <!-- Second Panel (Right/Bottom) -->
    <div
        class="relative z-0 flex-1 overflow-auto"
        style="{isVertical ? 'min-height' : 'min-width'}: {minRight};"
    >
        {#if right}
            {@render right()}
        {:else}
            <div class="p-4 text-gray-400">Right Content</div>
        {/if}
    </div>
</div>
