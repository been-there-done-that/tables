<script lang="ts">
    import { onMount, tick } from "svelte";
    import { cubicOut } from "svelte/easing";
    import { listConnections } from "$lib/commands/client";
    import type { Connection } from "$lib/commands/types";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import { invoke } from "@tauri-apps/api/core";
    import { cn } from "$lib/utils";
    import ListSvg from "$lib/svg/List.svelte";
    import { resolveDriverIcon } from "./datasource/DriverList";

    let connections = $state<Connection[]>([]);
    let isOpen = $state(false);
    let rendered = $state(false);
    let selectedConnection = $state<Connection | null>(null);
    let popoverRef = $state<HTMLDivElement | null>(null);
    let listRef = $state<HTMLDivElement | null>(null);
    let newConnBtn = $state<HTMLButtonElement | null>(null);
    let focusedIndex = $state(0);

    // Custom transition for production-grade feel
    function flyAndScale(
        node: Element,
        {
            delay = 0,
            duration = 200,
            easing = cubicOut,
            y = -8,
            start = 0.95,
        } = {},
    ) {
        const style = getComputedStyle(node);
        const opacity = +style.opacity;
        const transform = style.transform === "none" ? "" : style.transform;

        return {
            delay,
            duration,
            easing,
            css: (t: number) => `
                transform: ${transform} translate3d(0, ${(1 - t) * y}px, 0) scale(${start + (1 - start) * t});
                opacity: ${t * opacity};
            `,
        };
    }

    // Load connections
    const loadConnectionsData = async () => {
        try {
            const response = await listConnections();
            if (response.success && response.data) {
                connections = response.data.sort((a, b) =>
                    a.name.localeCompare(b.name),
                );
            } else {
                console.error("Failed to load connections:", response.error);
            }
        } catch (e) {
            console.error("Failed to load connections:", e);
        }
    };

    onMount(() => {
        loadConnectionsData();

        // Click outside handler
        const handleClickOutside = (event: PointerEvent) => {
            if (
                isOpen &&
                popoverRef &&
                !event.composedPath().includes(popoverRef)
            ) {
                isOpen = false;
            }
        };
        const handleKeydown = (event: KeyboardEvent) => {
            if (!isOpen) return;

            if (event.key === "Escape") {
                isOpen = false;
                return;
            }

            if (event.key === "Tab") {
                event.preventDefault();
                // Simple focus trap between the two focusable areas
                if (document.activeElement === newConnBtn) {
                    listRef?.focus();
                } else {
                    newConnBtn?.focus();
                }
                return;
            }

            // If focus is on the New Connection button, let standard behavior work (e.g. Enter clicks it)
            // and don't hijack arrows for list navigation
            if (document.activeElement === newConnBtn) {
                return;
            }

            if (event.key === "ArrowDown") {
                event.preventDefault();
                focusedIndex = Math.min(
                    focusedIndex + 1,
                    connections.length - 1,
                );
                scrollToFocused();
            } else if (event.key === "ArrowUp") {
                event.preventDefault();
                focusedIndex = Math.max(focusedIndex - 1, 0);
                scrollToFocused();
            } else if (event.key === "Enter") {
                event.preventDefault();
                if (connections[focusedIndex]) {
                    selectConnection(connections[focusedIndex]);
                } else if (connections.length === 0) {
                    // Maybe open new connection if list is empty?
                    openNewConnection();
                }
            }
        };

        document.addEventListener("pointerdown", handleClickOutside);
        document.addEventListener("keydown", handleKeydown);

        return () => {
            document.removeEventListener("pointerdown", handleClickOutside);
            document.removeEventListener("keydown", handleKeydown);
        };
    });

    const scrollToFocused = async () => {
        await tick();
        const button = listRef?.children[focusedIndex] as HTMLElement;
        if (button) {
            // Simple scroll into view if needed, though simple list usually fits
            button.scrollIntoView({ block: "nearest", behavior: "smooth" });
        }
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
        if (isOpen) {
            focusedIndex = 0; // Reset focus to top on open
            tick().then(() => {
                if (listRef) listRef.focus();
            });
        }
    });
</script>

<div class="relative flex justify-center" bind:this={popoverRef}>
    <button
        class={cn(
            "flex items-center gap-2 p-1 text-sm rounded-md transition-all duration-200",
            "hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)",
            "border border-transparent focus:outline-none focus:ring-1 focus:ring-(--theme-border-active)",
            isOpen ? "bg-(--theme-bg-active)" : "",
        )}
        onclick={() => (isOpen = !isOpen)}
        aria-expanded={isOpen}
    >
        <div class="flex items-center gap-2 px-2">
            {#if selectedConnection}
                <div class="flex items-center gap-2">
                    <div
                        class={cn(
                            "w-2 h-2 rounded-full shadow-sm",
                            selectedConnection.color_tag
                                ? `bg-[${selectedConnection.color_tag}]`
                                : "bg-emerald-500",
                        )}
                    ></div>
                    <span class="font-medium text-(--theme-fg-primary)"
                        >{selectedConnection.name}</span
                    >
                </div>
            {:else}
                <span class="opacity-70 flex">
                    <ListSvg />
                </span>
                <span class="font-medium text-(--theme-fg-secondary)"
                    >Select Connection</span
                >
            {/if}
            <IconChevronDown
                class={cn(
                    "size-3 opacity-50 transition-transform duration-200",
                    isOpen && "rotate-180",
                )}
            />
        </div>
    </button>

    {#if isOpen}
        <div
            class={cn(
                "absolute top-full left-1/2 -translate-x-1/2 mt-1 w-72 origin-top",
                "bg-(--theme-bg-secondary) border border-(--theme-border-default)",
                "rounded-lg shadow-xl shadow-black/10 ring-1 ring-black/5",
                "flex flex-col overflow-hidden z-50",
            )}
            transition:flyAndScale={{ y: -8, duration: 400, start: 0.95 }}
        >
            <!-- Connections List -->
            <div
                class="flex-1 overflow-y-auto max-h-[320px] py-1 focus:outline-none space-y-0.5"
                bind:this={listRef}
                tabindex="-1"
            >
                {#if connections.length === 0}
                    <div class="p-6 text-center">
                        <p
                            class="text-xs text-(--theme-fg-secondary) opacity-60"
                        >
                            No connections yet
                        </p>
                    </div>
                {:else}
                    {#each connections as conn, i}
                        {@const DriverIcon =
                            resolveDriverIcon(conn.engine) || IconDatabase}
                        <button
                            class={cn(
                                "w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors",
                                "focus:outline-none",
                                i === focusedIndex
                                    ? "bg-(--theme-bg-tertiary) text-(--theme-fg-primary)"
                                    : "text-(--theme-fg-secondary) hover:bg-(--theme-bg-tertiary)/50",
                                selectedConnection?.id === conn.id &&
                                    !focusedIndex &&
                                    "bg-(--theme-bg-tertiary)",
                            )}
                            onclick={() => selectConnection(conn)}
                            onmouseenter={() => (focusedIndex = i)}
                            tabindex="-1"
                        >
                            <DriverIcon
                                class={cn(
                                    "size-4 shrink-0 transition-opacity",
                                    i === focusedIndex ||
                                        selectedConnection?.id === conn.id
                                        ? "opacity-100 text-(--theme-accent-primary)"
                                        : "opacity-60 grayscale-[0.5]",
                                )}
                            />
                            <div class="flex flex-col min-w-0">
                                <span
                                    class={cn(
                                        "text-sm font-medium truncate leading-tight",
                                        i === focusedIndex
                                            ? "text-(--theme-fg-primary)"
                                            : "",
                                    )}>{conn.name}</span
                                >

                                <span
                                    class="text-[10px] text-(--theme-fg-secondary) opacity-40 truncate font-mono leading-tight"
                                >
                                    {conn.username || "root"}@{conn.host ||
                                        "localhost"}
                                </span>
                            </div>

                            {#if selectedConnection?.id === conn.id}
                                <div
                                    class="ml-auto w-1.5 h-1.5 rounded-full bg-(--theme-accent-primary)"
                                ></div>
                            {/if}
                        </button>
                    {/each}
                {/if}
            </div>

            <!-- Footer Action -->
            <div
                class="p-1 border-t border-(--theme-border-default) bg-(--theme-bg-primary)"
            >
                <button
                    class="flex items-center justify-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-(--theme-fg-secondary) hover:text-(--theme-fg-primary) hover:bg-(--theme-bg-tertiary) rounded-md transition-colors focus:outline-none focus:ring-1 focus:ring-dashed focus:ring-(--theme-fg-secondary) focus:bg-(--theme-bg-tertiary)"
                    onclick={openNewConnection}
                    bind:this={newConnBtn}
                >
                    <IconPlus class="size-3.5 opacity-60" />
                    New Connection
                </button>
            </div>
        </div>
    {/if}
</div>
