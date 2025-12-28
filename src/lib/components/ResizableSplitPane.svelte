<script lang="ts">
    import { onMount } from "svelte";

    /**
     * The default ratio of the left panel (0 to 1).
     * Default: 0.2 (20%)
     */
    export let defaultRatio = 0.2;

    /**
     * Minimum width of the left panel (CSS string, e.g., '200px', '10%').
     * Default: '100px'
     */
    export let minLeft = "100px";

    /**
     * Minimum width of the right panel (CSS string, e.g., '200px', '10%').
     * Default: '100px'
     */
    export let minRight = "100px";

    /**
     * Size of the visual gap between panels in pixels.
     * Default: 1
     */
    export let gapSize = 1;

    /**
     * Whether the pane is resizable.
     * Default: true
     */
    export let resizable = true;

    export let resetOnDisable = false;

    let container: HTMLElement;
    let isDragging = false;
    let leftWidthPercent = defaultRatio * 100;

    function startDrag(event: MouseEvent) {
        if (!resizable) return;
        isDragging = true;
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";

        // Attach listeners to window to handle drags outside component
        window.addEventListener("mousemove", onDrag);
        window.addEventListener("mouseup", stopDrag);
    }

    function onDrag(event: MouseEvent) {
        if (!isDragging || !container) return;

        const containerRect = container.getBoundingClientRect();
        const newLeftWidth = event.clientX - containerRect.left;

        // Convert to percentage
        let newRatio = newLeftWidth / containerRect.width;

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

        const minLeftPx = parseLength(minLeft, containerRect.width);
        const minRightPx = parseLength(minRight, containerRect.width);

        // Convert constraints to ratios
        const minRatio = minLeftPx / containerRect.width;
        const maxRatio = 1 - minRightPx / containerRect.width;

        // Apply clamping
        if (minRatio > maxRatio) {
            newRatio = 0.5; // Fallback if window too small
        } else {
            newRatio = Math.max(minRatio, Math.min(newRatio, maxRatio));
        }

        leftWidthPercent = newRatio * 100;
    }

    function stopDrag() {
        isDragging = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        window.removeEventListener("mousemove", onDrag);
        window.removeEventListener("mouseup", stopDrag);
    }

    $: if (!resizable && resetOnDisable) {
        leftWidthPercent = defaultRatio * 100;
    }
</script>

<div
    bind:this={container}
    class="flex flex-row w-full h-full overflow-hidden relative select-none"
    role="group"
>
    <!-- Left Panel -->
    <div
        class="flex-none h-full overflow-auto relative z-0 transition-[width] duration-0 ease-linear"
        class:transition-all={!isDragging}
        class:duration-300={!isDragging}
        style="width: {leftWidthPercent}%; min-width: {minLeft};"
    >
        <slot name="left">
            <div class="p-4 text-gray-400">Left Content</div>
        </slot>
    </div>

    <!-- Handle -->
    <div
        role="separator"
        tabindex="0"
        aria-valuenow={leftWidthPercent}
        aria-valuemin={0}
        aria-valuemax={100}
        class="relative h-full z-10 flex-non"
        style="width: {gapSize}px; cursor: {resizable
            ? 'col-resize'
            : 'default'};"
        on:mousedown={startDrag}
    >
        <!-- Visual Line -->
        <div class="absolute inset-y-0 w-px bg-(--theme-border-default)"></div>

        <!-- Hit Area (Invisible, wider for easier grabbing) -->
        <div
            class="absolute inset-y-0 -left-1 -right-1 z-20 bg-transparent"
        ></div>
    </div>

    <!-- Right Panel -->
    <div
        class="flex-1 h-full overflow-auto relative z-0"
        style="min-width: {minRight};"
    >
        <slot name="right">
            <div class="p-4 text-gray-400">Right Content</div>
        </slot>
    </div>
</div>
