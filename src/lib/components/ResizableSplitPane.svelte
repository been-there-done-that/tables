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
        leftVisible = true,
        rightVisible = true,
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
        leftVisible?: boolean;
        rightVisible?: boolean;
        left?: Snippet;
        right?: Snippet;
    } = $props();

    let container = $state<HTMLElement>();
    let isDragging = $state(false);
    let ratio = $state(defaultRatio);

    const isVertical = $derived(orientation === "vertical");
    const axis = $derived(isVertical ? "height" : "width");
    const cursor = $derived(isVertical ? "row-resize" : "col-resize");

    /* ---------- Derived layout ---------- */

    const firstSize = $derived.by(() => {
        if (!leftVisible) return "0px";
        if (!rightVisible) return "100%";
        return `${ratio * 100}%`;
    });

    const secondSize = $derived.by(() => {
        if (!rightVisible) return "0px";
        return "auto";
    });

    const dividerSize = $derived.by(() =>
        leftVisible && rightVisible ? `${gapSize}px` : "0px",
    );

    const dividerOpacity = $derived.by(() =>
        !leftVisible || !rightVisible ? "0" : "1",
    );

    const dividerPointerEvents = $derived.by(() =>
        !leftVisible || !rightVisible ? "none" : "auto",
    );

    /* ---------- Drag logic ---------- */

    function startDrag() {
        if (!resizable) return;
        isDragging = true;
        document.body.style.cursor = cursor;
        document.body.style.userSelect = "none";
        window.addEventListener("mousemove", onDrag);
        window.addEventListener("mouseup", stopDrag);
    }

    function onDrag(e: MouseEvent) {
        if (!container) return;

        const rect = container.getBoundingClientRect();
        const total = isVertical ? rect.height : rect.width;
        const offset = isVertical
            ? e.clientY - rect.top
            : e.clientX - rect.left;

        // Simple ratio clamping between 0 and 1
        ratio = Math.max(0, Math.min(offset / total, 1));
    }

    function stopDrag() {
        isDragging = false;
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
        window.removeEventListener("mousemove", onDrag);
        window.removeEventListener("mouseup", stopDrag);
    }

    $effect(() => {
        if (!resizable && resetOnDisable) {
            ratio = defaultRatio;
        }
    });
</script>

<div
    bind:this={container}
    class="flex h-full w-full overflow-hidden"
    class:flex-col={isVertical}
>
    <!-- First panel -->
    <div
        class="overflow-hidden transition-[width,height] duration-200 ease-out"
        class:transition-none={isDragging}
        style="{axis}: {firstSize}; min-{axis}: {leftVisible ? minLeft : '0px'}"
    >
        {@render left?.()}
    </div>

    <!-- Divider -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        class="relative flex-none transition-opacity duration-200"
        style="{axis}: {dividerSize}; opacity: {dividerOpacity}; pointer-events: {dividerPointerEvents}"
        onmousedown={startDrag}
    >
        <!-- visual line -->
        <div
            class="absolute inset-0 bg-(--theme-border-default)"
            class:w-px={!isVertical}
            class:h-px={isVertical}
        ></div>

        <!-- hit area -->
        <div
            class="absolute inset-0"
            style={isVertical ? "top:-4px;bottom:-4px" : "left:-4px;right:-4px"}
            style:cursor={resizable ? cursor : "default"}
        ></div>
    </div>

    <!-- Second panel -->
    <div
        class="flex-1 overflow-hidden"
        style="{axis}: {secondSize}; min-{axis}: {rightVisible
            ? minRight
            : '0px'}"
    >
        {@render right?.()}
    </div>
</div>
