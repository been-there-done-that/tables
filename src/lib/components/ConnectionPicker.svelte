<script lang="ts">
    import { onMount, tick } from "svelte";
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
            if (event.key === "Escape" && isOpen) {
                isOpen = false;
            }
        };

        document.addEventListener("pointerdown", handleClickOutside);
        document.addEventListener("keydown", handleKeydown);

        return () => {
            document.removeEventListener("pointerdown", handleClickOutside);
            document.removeEventListener("keydown", handleKeydown);
        };
    });

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
            rendered = true;
            // Focus list for keyboard nav support (future)
            tick().then(() => {
                if (listRef) listRef.focus();
            });
        }
    });

    const handleAnimationEnd = (event: AnimationEvent) => {
        // Animation cleanup if needed, currently reusing logic from original but simpler
        if (!isOpen) {
            rendered = false;
        }
    };
</script>

<div class="relative" bind:this={popoverRef}>
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

    {#if rendered}
        <div
            class={cn(
                "absolute top-full left-0 mt-2 w-72 origin-top-left",
                "bg-(--theme-bg-secondary) border border-(--theme-border-default)",
                "rounded-lg shadow-xl shadow-black/10 ring-1 ring-black/5",
                "flex flex-col overflow-hidden z-50",
                "animate-in fade-in zoom-in-95 duration-100 ease-out", // Using Tailwind animate-in if available, else standard transition
            )}
            style:display={isOpen ? "flex" : "none"}
            style:visibility={isOpen ? "visible" : "hidden"}
        >
            <!-- Connections List -->
            <div
                class="flex-1 overflow-y-auto max-h-[320px] py-1 focus:outline-none"
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
                    {#each connections as conn}
                        {@const DriverIcon =
                            resolveDriverIcon(conn.engine) || IconDatabase}
                        <button
                            class={cn(
                                "w-full flex items-center gap-3 px-3 py-2 text-left transition-colors",
                                "hover:bg-(--theme-bg-tertiary) focus:bg-(--theme-bg-tertiary) focus:outline-none",
                                selectedConnection?.id === conn.id &&
                                    "bg-(--theme-bg-tertiary)",
                            )}
                            onclick={() => selectConnection(conn)}
                        >
                            <DriverIcon
                                class={cn(
                                    "size-4 shrink-0 opacity-70",
                                    selectedConnection?.id === conn.id
                                        ? "text-(--theme-accent-primary) opacity-100"
                                        : "",
                                )}
                            />
                            <div class="flex flex-col min-w-0">
                                <span
                                    class={cn(
                                        "text-sm font-medium truncate",
                                        selectedConnection?.id === conn.id
                                            ? "text-(--theme-fg-primary)"
                                            : "text-(--theme-fg-secondary)",
                                    )}>{conn.name}</span
                                >

                                <span
                                    class="text-[10px] text-(--theme-fg-secondary) opacity-40 truncate font-mono"
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
                    class="flex items-center justify-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-(--theme-fg-secondary) hover:text-(--theme-fg-primary) hover:bg-(--theme-bg-tertiary) rounded-md transition-colors"
                    onclick={openNewConnection}
                >
                    <IconPlus class="size-3.5 opacity-60" />
                    New Connection
                </button>
            </div>
        </div>
    {/if}
</div>
