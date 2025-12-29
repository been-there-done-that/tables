<script lang="ts">
    import { onMount } from "svelte";
    import { listConnections } from "$lib/commands/client";
    import type { Connection } from "$lib/commands/types";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import IconSearch from "@tabler/icons-svelte/icons/search";
    import { invoke } from "@tauri-apps/api/core";
    import { cn } from "$lib/utils";
    import ListSvg from "$lib/svg/List.svelte";
    import DiagonalArrowSvg from "$lib/svg/DiagonalArrow.svelte";
    import { resolveDriverIcon } from "./datasource/DriverList";

    let connections = $state<Connection[]>([]);
    let isOpen = $state(false);
    let rendered = $state(false);
    let selectedConnection = $state<Connection | null>(null);
    let popoverRef = $state<HTMLDivElement | null>(null);
    let panelRef = $state<HTMLDivElement | null>(null);
    const MIN_WIDTH = 220;
    const MIN_HEIGHT = 260;

    let width = $state(360);
    let height = $state(460);
    let isResizing = $state(false);

    const clamp = (val: number, min: number, max: number) =>
        Math.min(Math.max(val, min), max);

    const persistSize = (nextWidth: number, nextHeight: number) => {
        width = nextWidth;
        height = nextHeight;
        localStorage.setItem("connection-picker-width", width.toString());
        localStorage.setItem("connection-picker-height", height.toString());
    };

    // Load connections
    const loadConnectionsData = async () => {
        try {
            const response = await listConnections();
            if (response.success && response.data) {
                connections = response.data;
            } else {
                console.error("Failed to load connections:", response.error);
            }
        } catch (e) {
            console.error("Failed to load connections:", e);
        }
    };

    onMount(() => {
        loadConnectionsData();

        // Load saved dimensions
        const savedWidth = localStorage.getItem("connection-picker-width");
        const savedHeight = localStorage.getItem("connection-picker-height");
        const parsedWidth = savedWidth ? parseInt(savedWidth, 10) : NaN;
        const parsedHeight = savedHeight ? parseInt(savedHeight, 10) : NaN;

        if (!Number.isNaN(parsedWidth)) {
            width = clamp(
                parsedWidth,
                MIN_WIDTH,
                Math.max(MIN_WIDTH, window.innerWidth - 32),
            );
        }
        if (!Number.isNaN(parsedHeight)) {
            height = clamp(
                parsedHeight,
                MIN_HEIGHT,
                Math.max(MIN_HEIGHT, window.innerHeight - 80),
            );
        }

        // Click outside handler
        const handleClickOutside = (event: MouseEvent) => {
            if (isResizing) return; // Don't close while resizing

            if (
                isOpen &&
                popoverRef &&
                panelRef &&
                !popoverRef.contains(event.target as Node) &&
                !panelRef.contains(event.target as Node)
            ) {
                isOpen = false;
            }
        };
        document.addEventListener("mousedown", handleClickOutside);

        return () => {
            document.removeEventListener("mousedown", handleClickOutside);
        };
    });

    const startManualResize = (event: PointerEvent) => {
        event.preventDefault();
        event.stopPropagation();
        isResizing = true;

        const startX = event.clientX;
        const startY = event.clientY;
        const startWidth = width;
        const startHeight = height;

        const handleMove = (moveEvent: PointerEvent) => {
            const deltaX = moveEvent.clientX - startX;
            const deltaY = moveEvent.clientY - startY;
            const maxWidth = Math.max(MIN_WIDTH, window.innerWidth - 32);
            const maxHeight = Math.max(MIN_HEIGHT, window.innerHeight - 80);

            // For centered element, width increases by 2 * deltaX if we want right side to move?
            // Wait, this is `left: 50%; transform: translateX(-50%)`.
            // Resizing from bottom-right corner means:
            // The element's center stays fixed, so it grows on BOTH sides visually if width changes?
            // No, standard flow would grow to the right, but transform -50% shifts it back.
            // If we drag the right edge by X, width increases by X.
            // Since it's centered, increasing width by X makes it grow X/2 to left and X/2 to right.
            // This means the mouse cursor will drift from the edge if we just do width += deltaX.
            // To make the right edge follow the mouse:
            // Current Right Edge X = CenterX + Width/2
            // New Right Edge X = MouseX
            // New Width/2 = MouseX - CenterX
            // New Width = 2 * (MouseX - CenterX)

            // Let's calculate based on center position
            if (panelRef) {
                const rect = panelRef.getBoundingClientRect();
                const center = rect.left + rect.width / 2;
                const newHalfWidth = moveEvent.clientX - center;
                // Double it because we want symmetric growth or at least tracking right edge?
                // Actually, dragging the right edge on a centered element is tricky.
                // If we just increase width, it grows on both sides.
                // To keep the mouse on the corner, we need the width to increase by 2 * movement if we only move right edge.

                // Let's rely on simple delta for now, but multiplied by 2 might feel more "sticky" to cursor
                // if users expect to drag the edge.
                // However, simpliest UX for centered popover resizing is often symmetrical or just direct width mapping.

                // Let's try direct mapping 1:1 first (it will "slip" under cursor by half speed),
                // or 2:1 (cursor stays attached but left side moves too).
                // 2:1 is usually standard for center-aligned resizing.

                const nextWidth = clamp(
                    startWidth + deltaX,
                    MIN_WIDTH,
                    maxWidth,
                );
                const nextHeight = clamp(
                    startHeight + deltaY,
                    MIN_HEIGHT,
                    maxHeight,
                ); // Height is top-anchored usually?
                // Top is fixed at 36px. So height grows down. 1:1 is correct for height.

                persistSize(nextWidth, nextHeight);
            }
        };

        const handleUp = () => {
            isResizing = false;
            window.removeEventListener("pointermove", handleMove);
            window.removeEventListener("pointerup", handleUp);
        };

        window.addEventListener("pointermove", handleMove);
        window.addEventListener("pointerup", handleUp);
    };

    const selectConnection = (conn: Connection) => {
        selectedConnection = conn;
        isOpen = false;
        // TODO: Actually open connection / trigger global event
    };

    const openNewConnection = async () => {
        try {
            await invoke("open_datasource_window");
            isOpen = false;
        } catch (e) {
            console.error("Failed to open datasource window:", e);
        }
    };

    $effect(() => {
        if (isOpen) rendered = true;
    });

    const handleAnimationEnd = (event: AnimationEvent) => {
        if (!isOpen && event.target === panelRef) {
            rendered = false;
        }
    };
</script>

<div class="relative" bind:this={popoverRef}>
    <button
        class="flex items-center gap-2 p-px text-sm rounded-md transition-colors hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active) border border-transparent hover:border-(--theme-border-subtle)"
        onclick={() => (isOpen = !isOpen)}
    >
        <div
            class="flex items-center gap-1 border border-dashed border-(--theme-border-default) py-1 pl-3 pr-2 rounded-md justify-center shadow-sm bg-(--theme-bg-primary)"
        >
            {#if selectedConnection}
                <div class="flex items-center gap-2">
                    <div
                        class={cn(
                            "w-2 h-2 rounded-full",
                            selectedConnection.color_tag
                                ? `bg-[${selectedConnection.color_tag}]`
                                : "bg-green-500",
                        )}
                    ></div>
                    <span class="font-medium">{selectedConnection.name}</span>
                </div>
            {:else}
                <ListSvg />
                <span class="pl-2 text-xs font-semibold tracking-wider"
                    >Select Connection</span
                >
            {/if}
            <IconChevronDown class="size-4 " />
        </div>
    </button>

    {#if rendered}
        <div
            bind:this={panelRef}
            class="absolute top-full left-0 mt-1 bg-(--theme-bg-secondary) border border-(--theme-border-default) rounded-lg shadow-xl z-50 overflow-hidden flex flex-col min-w-[200px] anim-pop"
            style={`width: ${width}px; height: ${height}px;`}
            data-state={isOpen ? "open" : "closed"}
            onanimationend={handleAnimationEnd}
        >
            <!-- Header / Search -->
            <div
                class="p-2 border-b border-(--theme-border-default) bg-(--theme-bg-tertiary)/30"
            >
                <div class="relative">
                    <IconSearch
                        class="absolute left-2 top-1/2 -translate-y-1/2 size-3.5 opacity-50"
                    />
                    <input
                        type="text"
                        placeholder="Search connections..."
                        class="w-full bg-(--theme-bg-primary) border border-(--theme-border-default) rounded-md py-1 pl-8 pr-2 text-xs focus:outline-none focus:border-(--theme-accent-primary)"
                    />
                </div>
            </div>

            <!-- Actions -->
            <button
                class="flex items-center gap-2 px-3 py-2 text-xs hover:bg-(--theme-bg-tertiary) transition-colors border-b border-(--theme-border-default) text-(--theme-accent-primary)"
                onclick={openNewConnection}
            >
                <IconPlus class="size-3.5" />
                New Connection...
            </button>

            <!-- Connections List -->
            <div class="flex-1 overflow-y-auto p-1">
                {#if connections.length === 0}
                    <div class="p-4 text-center text-xs opacity-50">
                        No connections found
                    </div>
                {:else}
                    <div
                        class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider opacity-50 text-(--theme-fg-secondary)"
                    >
                        Recents
                    </div>
                    {#each connections as conn}
                        {@const DriverIcon =
                            resolveDriverIcon(conn.engine) || IconDatabase}
                        <button
                            class={cn(
                                "w-full flex items-center justify-between px-2 py-1.5 rounded-md text-sm transition-colors group",
                                selectedConnection?.id === conn.id
                                    ? "bg-(--theme-accent-primary)/10 text-(--theme-accent-primary)"
                                    : "hover:bg-(--theme-bg-tertiary)",
                            )}
                            onclick={() => selectConnection(conn)}
                        >
                            <div
                                class="flex items-center gap-2 overflow-hidden"
                            >
                                <DriverIcon
                                    class={cn(
                                        "size-4 shrink-0",
                                        selectedConnection?.id === conn.id
                                            ? "opacity-100"
                                            : "opacity-50 group-hover:opacity-75",
                                    )}
                                />
                                <div class="flex flex-col items-start truncate">
                                    <span class="truncate font-medium"
                                        >{conn.name}</span
                                    >
                                    <span
                                        class="text-[10px] opacity-50 truncate"
                                        >{conn.username || "root"}@{conn.host ||
                                            "localhost"}</span
                                    >
                                </div>
                            </div>
                        </button>
                    {/each}
                {/if}
            </div>

            <!-- Footer/Status -->
            <div
                class="px-2 py-1 text-[10px] opacity-50 bg-(--theme-bg-tertiary)/30 flex justify-center items-center select-none"
            >
                <span
                    >{connections.length} Connection{connections.length === 1
                        ? ""
                        : "s"} (0 connected)</span
                >
            </div>

            <!-- Corner resize handle for reliable dragging -->
            <button
                type="button"
                aria-label="Resize"
                class="absolute bottom-1 right-1 p-0.5 text-(--theme-fg-secondary) opacity-50 hover:opacity-100 cursor-se-resize transition bg-transparent border border-dashed border-(--theme-border-default) rounded-sm"
                onpointerdown={startManualResize}
                title="Drag to resize"
            >
                <DiagonalArrowSvg />
            </button>
        </div>
    {/if}
</div>
