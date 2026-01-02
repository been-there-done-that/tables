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
    import * as Menu from "$lib/components/ui/context-menu";

    import { schemaStore } from "$lib/stores/schema.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import IconLogout from "@tabler/icons-svelte/icons/logout";

    let connections = $state<Connection[]>([]);
    let isOpen = $state(false);

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
    });

    const selectConnection = async (conn: Connection) => {
        isOpen = false;
        // Start a new session for this connection
        windowState.startSession(conn);
        await schemaStore.connect(conn);
    };

    const disconnectConnection = async () => {
        isOpen = false;
        await schemaStore.disconnect();
    };

    const openNewConnection = async () => {
        try {
            await invoke("open_datasource_window");
            isOpen = false;
        } catch (e) {
            console.error("Failed to open datasource window:", e);
        }
    };
</script>

<div class="relative flex justify-center">
    <Menu.Root bind:open={isOpen}>
        <Menu.DropdownTrigger>
            <button
                class={cn(
                    "group flex items-center gap-2 p-1 text-sm rounded-md transition-all duration-200",
                    "hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)",
                    "border border-transparent focus:outline-none focus:ring-1 focus:ring-(--theme-border-active)",
                    isOpen ? "bg-(--theme-bg-active)" : "",
                )}
                aria-expanded={isOpen}
            >
                <div class="flex items-center gap-2 px-2">
                    {#if schemaStore.activeConnection}
                        <div class="flex items-center gap-2">
                            <div
                                class={cn(
                                    "w-2 h-2 rounded-full shadow-sm",
                                    schemaStore.activeConnection.color_tag
                                        ? `bg-[${schemaStore.activeConnection.color_tag}]`
                                        : "bg-emerald-500",
                                )}
                            ></div>
                            <span class="font-medium text-(--theme-fg-primary)"
                                >{schemaStore.activeConnection.name}</span
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
                    {#if schemaStore.status === "connecting" || schemaStore.status === "refreshing"}
                        <IconLoader2
                            class="animate-spin size-3 opacity-50 transition-transform duration-200"
                        />
                    {:else}
                        <IconChevronDown
                            class={cn(
                                "size-3 opacity-50 transition-transform duration-200",
                                isOpen && "rotate-180",
                            )}
                        />
                    {/if}
                </div>
            </button>
        </Menu.DropdownTrigger>

        <Menu.Content
            class="-translate-x-1/2 mt-1 w-72 origin-top p-0 z-50 overflow-hidden"
        >
            <!-- Connections List -->
            <div class="flex-1 overflow-y-auto max-h-[320px] py-1 space-y-0.5">
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
                        <Menu.Item
                            class={cn(
                                "w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors",
                                schemaStore.activeConnection?.id === conn.id &&
                                    "bg-(--theme-bg-tertiary)",
                            )}
                            onclick={() => selectConnection(conn)}
                        >
                            <DriverIcon
                                class={cn(
                                    "size-4 shrink-0 transition-opacity",
                                    schemaStore.activeConnection?.id === conn.id
                                        ? "opacity-100 text-(--theme-accent-primary)"
                                        : "opacity-60 grayscale-[0.5]",
                                )}
                            />
                            <div class="flex flex-col min-w-0">
                                <span
                                    class="text-sm font-medium truncate leading-tight"
                                    >{conn.name}</span
                                >

                                <span
                                    class="text-[10px] text-(--theme-fg-secondary) opacity-40 truncate font-mono leading-tight"
                                >
                                    {conn.username || "root"}@{conn.host ||
                                        "localhost"}
                                </span>
                            </div>

                            {#if schemaStore.activeConnection?.id === conn.id}
                                <div
                                    class="ml-auto w-1.5 h-1.5 rounded-full bg-(--theme-accent-primary)"
                                ></div>
                            {/if}
                        </Menu.Item>
                    {/each}
                {/if}
            </div>

            <!-- Footer Actions -->
            <div
                class="p-1 border-t border-(--theme-border-default) bg-(--theme-bg-primary) flex flex-col gap-0.5"
            >
                <Menu.Item
                    class="flex items-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-(--theme-fg-secondary) hover:text-(--theme-fg-primary) rounded-md transition-colors"
                    onclick={openNewConnection}
                >
                    <IconPlus class="size-3.5 opacity-60" />
                    New Connection
                </Menu.Item>

                {#if schemaStore.activeConnection}
                    <Menu.Item
                        class="flex items-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-red-500/80 hover:text-red-500 hover:bg-red-500/5 rounded-md transition-colors"
                        onclick={disconnectConnection}
                    >
                        <IconLogout class="size-3.5 opacity-80" />
                        Disconnect
                    </Menu.Item>
                {/if}
            </div>
        </Menu.Content>
    </Menu.Root>
</div>
