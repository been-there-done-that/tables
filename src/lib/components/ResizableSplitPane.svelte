<script lang="ts">
    import { type Snippet, untrack } from "svelte";

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

    // Fix: unwrap defaultRatio to avoid reactive dependency warning if intended to be initial only
    let ratio = $state(defaultRatio);

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

    function onDrag(e: MouseEvent) {
        if (!container) return;

        const rect = container.getBoundingClientRect();
        const total = isVertical ? rect.height : rect.width;
        const offset = isVertical
            ? e.clientY - rect.top
            : e.clientX - rect.left;

        // Parse constraints relative to total size
        const parse = (v: string) =>
            v.endsWith("%") ? (parseFloat(v) / 100) * total : parseFloat(v);

        const minStart = parse(minLeft) / total;
        const minEnd = 1 - parse(minRight) / total;

        // Clamp ratio between minStart and minEnd
        ratio = Math.max(minStart, Math.min(offset / total, minEnd));
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
    <!-- svelte-ignore a11y_separator_implicit_roles -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
        role="separator"
        tabindex="0"
        class="relative flex-none transition-all duration-300 ease-[cubic-bezier(0.25,0.1,0.25,1)] outline-none"
        class:opacity-0={!leftVisible || !rightVisible}
        class:pointer-events-none={!leftVisible || !rightVisible}
        class:cursor-row-resize={resizable && isVertical}
        class:cursor-col-resize={resizable && !isVertical}
        style="{axis}: {dividerSize}; touch-action: none;"
        onmousedown={startDrag}
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
