<script lang="ts">
    import { type Snippet, untrack } from "svelte";

    let {
        defaultRatio = 0.2,
        controlledRatio = $bindable(),
        onRatioChange,
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
        controlledRatio?: number;
        onRatioChange?: (ratio: number) => void;
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

    // Internal ratio state, syncs with controlled value if provided
    let internalRatio = $state(untrack(() => controlledRatio ?? defaultRatio));

    // Use controlled ratio if provided, otherwise use internal
    const ratio = $derived(controlledRatio ?? internalRatio);

    // Sync internal ratio when controlled ratio changes
    $effect(() => {
        if (controlledRatio !== undefined) {
            internalRatio = controlledRatio;
        }
    });

    const isVertical = $derived(orientation === "vertical");
    const axis = $derived(isVertical ? "height" : "width");
    const cursor = $derived(isVertical ? "row-resize" : "col-resize");

    /* ---------- Derived layout ---------- */

    const firstSize = $derived.by(() => {
        if (!leftVisible) return "0px";
        if (!rightVisible) return "100%";
        // Debugging: pure percentage width, no min/max constraints
        return `${ratio * 100}%`;
    });

    const secondSize = $derived.by(() => {
        return "auto";
    });

    const dividerSize = $derived.by(() =>
        leftVisible && rightVisible ? `${gapSize}px` : "0px",
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

    function onKeyDown(e: KeyboardEvent) {
        if (!resizable) return;
        const horizontalKeys = ["ArrowLeft", "ArrowRight"];
        const verticalKeys = ["ArrowUp", "ArrowDown"];

        if (
            (!isVertical && !horizontalKeys.includes(e.key)) ||
            (isVertical && !verticalKeys.includes(e.key))
        )
            return;

        e.preventDefault();
        const step = 0.02;
        const delta =
            e.key === "ArrowLeft" || e.key === "ArrowUp" ? -step : step;
        clampRatio(ratio + delta);
    }

    const clampRatio = (next: number) => {
        if (!container) return;
        const rect = container.getBoundingClientRect();
        const total = isVertical ? rect.height : rect.width;
        if (total <= 0) return;

        const parse = (v: string) =>
            v.endsWith("%") ? (parseFloat(v) / 100) * total : parseFloat(v);

        const minStart = parse(minLeft) / total;
        const minEnd = 1 - parse(minRight) / total;
        const clamped = Math.max(minStart, Math.min(next, minEnd));
        internalRatio = clamped;
        onRatioChange?.(clamped);
    };

    function onDrag(e: MouseEvent) {
        if (!container) return;

        const rect = container.getBoundingClientRect();
        const total = isVertical ? rect.height : rect.width;
        const offset = isVertical
            ? e.clientY - rect.top
            : e.clientX - rect.left;

        clampRatio(offset / total);
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
            internalRatio = defaultRatio;
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
        class="overflow-hidden transition-[width,height,flex-basis] duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] will-change-[width,height] min-w-0 min-h-0"
        class:transition-none={isDragging}
        style="{axis}: {firstSize};"
    >
        <!-- Inner wrapper: Restore min-{axis} to prevent squashing -->
        <div style="width: 100%; height: 100%; min-{axis}: {minLeft}">
            {@render left?.()}
        </div>
    </div>

    <!-- Divider -->
    <div
        role="slider"
        tabindex="0"
        aria-orientation={isVertical ? "vertical" : "horizontal"}
        aria-valuemin="0"
        aria-valuemax="1"
        aria-valuenow={ratio}
        aria-label="Resize panels"
        class="relative flex-none transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] outline-none"
        class:opacity-0={!leftVisible || !rightVisible}
        class:pointer-events-none={!leftVisible || !rightVisible}
        class:cursor-row-resize={resizable && isVertical}
        class:cursor-col-resize={resizable && !isVertical}
        style="{axis}: {dividerSize}; touch-action: none;"
        onmousedown={startDrag}
        onkeydown={onKeyDown}
    >
        <!-- visual line -->
        <div
            class="absolute inset-0 bg-(--theme-border-default)"
            class:w-px={!isVertical}
            class:h-px={isVertical}
            class:opacity-0={!leftVisible || !rightVisible}
        ></div>

        <!-- hit area -->
        <div
            class="absolute inset-0"
            style={isVertical ? "top:-4px;bottom:-4px" : "left:-4px;right:-4px"}
        ></div>
    </div>

    <!-- Second panel -->
    <div
        class="flex-1 overflow-hidden min-w-0 min-h-0"
        style="{axis}: {secondSize};"
    >
        <!-- Inner wrapper: Restore min-{axis} to prevent squashing -->
        <div style="width: 100%; height: 100%; min-{axis}: {minRight}">
            {@render right?.()}
        </div>
    </div>
</div>
