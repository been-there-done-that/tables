<script lang="ts">
    import { onMount } from "svelte";
    import { listConnections } from "$lib/commands/client";
    import type { Connection } from "$lib/commands/types";
    import IconChevronDown from "@tabler/icons-svelte/icons/chevron-down";
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconDatabase from "@tabler/icons-svelte/icons/database";
    import { invoke } from "@tauri-apps/api/core";
    import { cn } from "$lib/utils";
    import ListSvg from "$lib/svg/List.svelte";
    import IconDataSource from "$lib/svg/IconDataSource.svelte";
    import { resolveDriverIcon } from "./datasource/DriverList";
    import * as Menu from "$lib/components/ui/dropdown-menu";

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
        if (isDisabled(conn)) return;
        isOpen = false;
        // Start a new session for this connection
        windowState.startSession(conn);
        await schemaStore.connect(conn);
    };

    const isBusy = (id: string) => schemaStore.isConnectionBusy(id);

    const isDisabled = (conn: Connection) => {
        if (!isBusy(conn.id)) return false;
        // Only disable SQLite if busy ( Postgres can be multi-window)
        return conn.engine === "sqlite";
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

    function handleKeydown(e: KeyboardEvent) {
        if (
            (e.metaKey || e.ctrlKey) &&
            e.shiftKey &&
            e.key.toLowerCase() === "c"
        ) {
            e.preventDefault();
            isOpen = !isOpen;
        }
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative flex justify-center">
    <Menu.Root bind:open={isOpen}>
        <Menu.Trigger
            class={cn(
                "group flex items-center gap-2 p-1 text-sm rounded-md transition-all duration-200",
                "hover:bg-(--theme-bg-hover) active:bg-(--theme-bg-active)",
                "border border-transparent focus:outline-none",
                isOpen ? "bg-(--theme-bg-active)" : "",
            )}
            aria-expanded={isOpen}
        >
            <div class="flex items-center gap-2 px-2">
                {#if schemaStore.activeConnection}
                    <div class="flex items-center gap-2">
                        <IconDataSource
                            class="w-4 h-4 text-(--theme-accent-primary)"
                        />
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
                            "size-4 opacity-50 transition-transform duration-200",
                            isOpen && "rotate-180",
                        )}
                    />
                {/if}
            </div>
        </Menu.Trigger>

        <Menu.Content
            class="mt-1 w-72 origin-top p-0 z-50 overflow-hidden rounded-md border border-(--theme-border-default) bg-(--theme-bg-secondary) shadow-lg"
            align="start"
            onCloseAutoFocus={(e) => e.preventDefault()}
            onkeydown={(e) => {
                if (
                    (e.metaKey || e.ctrlKey) &&
                    e.shiftKey &&
                    e.key.toLowerCase() === "c"
                ) {
                    e.preventDefault();
                    isOpen = false;
                }
            }}
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
                        {@const busy = isBusy(conn.id)}
                        {@const disabled = isDisabled(conn)}
                        <Menu.Item
                            class={cn(
                                "w-full flex items-center gap-3 px-3 py-1.5 text-left transition-colors",
                                schemaStore.activeConnection?.id === conn.id &&
                                    "bg-accent/5 ring-1 ring-inset ring-accent/10",
                                disabled && "opacity-50 cursor-not-allowed",
                            )}
                            {disabled}
                            onclick={() => selectConnection(conn)}
                        >
                            {@const isImageIcon =
                                typeof DriverIcon === "string"}
                            {#if isImageIcon}
                                <img
                                    src={DriverIcon}
                                    alt={conn.engine}
                                    class={cn(
                                        "size-10 shrink-0 transition-opacity object-contain",
                                        schemaStore.activeConnection?.id ===
                                            conn.id
                                            ? "opacity-100"
                                            : "opacity-60 grayscale-[0.5]",
                                    )}
                                />
                            {:else}
                                <DriverIcon
                                    class={cn(
                                        "size-4 shrink-0 transition-opacity",
                                        schemaStore.activeConnection?.id ===
                                            conn.id
                                            ? "opacity-100 text-(--theme-accent-primary)"
                                            : "opacity-60 grayscale-[0.5]",
                                    )}
                                />
                            {/if}
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

                            {#if busy}
                                <div
                                    class="ml-auto px-1.5 py-0.5 rounded-full bg-amber-500/10 text-amber-500 text-[9px] font-bold uppercase tracking-wider border border-amber-500/20"
                                >
                                    Busy
                                </div>
                            {/if}
                        </Menu.Item>
                    {/each}
                {/if}
            </div>

            <!-- Footer Actions -->
            <div
                class="p-1 border-t border-border bg-muted/30 flex flex-col gap-0.5"
            >
                <Menu.Item
                    class="flex items-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/10 rounded-md transition-colors cursor-pointer"
                    onclick={openNewConnection}
                >
                    <IconPlus class="size-3.5 opacity-60" />
                    New Connection
                </Menu.Item>

                {#if schemaStore.activeConnection}
                    <Menu.Item
                        class="flex items-center gap-2 w-full px-3 py-1.5 text-xs font-medium text-red-500/80 hover:text-red-600 hover:bg-red-500/10 rounded-md transition-colors cursor-pointer"
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
