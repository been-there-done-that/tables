<script lang="ts">
    import { drivers, resolveDriverIcon } from "./DriverList";
    import {
        IconPlus,
        IconDatabase,
        IconSearch,
        IconRobot,
    } from "@tabler/icons-svelte";
    import { connectionStore } from "$lib/commands/stores.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import type { Connection } from "$lib/commands/types";

    interface Props {
        onSelectConnection: (connection: Connection) => void;
        onAddNew: () => void;
        selectedConnection: Connection | null;
    }

    let { onSelectConnection, onAddNew, selectedConnection }: Props = $props();

    let searchQuery = $state("");

    const filteredConnections = $derived(
        connectionStore.state.connections.filter((c) =>
            c.name.toLowerCase().includes(searchQuery.toLowerCase()),
        ),
    );
</script>

<div class="flex flex-col h-full bg-background">
    <!-- Header/Actions -->
    <div class="p-4 border-b border-border">
        <div class="flex items-center justify-between gap-2">
            <!-- Search -->
            <div class="relative group">
                <IconSearch
                    size={14}
                    class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground group-focus-within:text-accent transition-colors"
                />
                <input
                    type="text"
                    bind:value={searchQuery}
                    placeholder="Search connections..."
                    class="w-full bg-accent/5 border border-border rounded-md pl-9 pr-3 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-accent/30 focus:border-accent/40"
                />
            </div>

            <div class="flex items-center gap-1">
                <button
                    class="p-1.5 {windowState.agentConsoleOpen
                        ? 'bg-primary/20 text-primary border-primary/30'
                        : 'bg-accent/10 text-accent border-transparent'} hover:bg-primary/30 border rounded-md transition-all cursor-pointer"
                    title="Open AI Agent (⌘⇧A)"
                    onclick={() =>
                        (windowState.agentConsoleOpen =
                            !windowState.agentConsoleOpen)}
                >
                    <IconRobot size={16} />
                </button>

                <button
                    class="p-1.5 bg-accent/10 hover:bg-accent/20 text-accent rounded-md transition-colors cursor-pointer"
                    title="Add New Connection"
                    onclick={onAddNew}
                >
                    <IconPlus size={16} />
                </button>
            </div>
        </div>
    </div>

    <!-- Connections List -->
    <div class="grow overflow-y-auto py-2">
        {#if filteredConnections.length === 0}
            <div class="px-4 py-8 text-center">
                <p class="text-xs text-muted-foreground">
                    No connections found
                </p>
                {#if searchQuery}
                    <button
                        class="mt-2 text-[11px] text-accent hover:underline cursor-pointer"
                        onclick={() => (searchQuery = "")}
                    >
                        Clear search
                    </button>
                {/if}
            </div>
        {:else}
            <div class="space-y-0.5 px-2">
                {#each filteredConnections as connection}
                    {@const IconComponent = resolveDriverIcon(
                        connection.engine,
                    )}
                    <button
                        class="w-full text-left px-3 py-2 flex items-center space-x-3 rounded-md transition-all cursor-pointer
                        {selectedConnection?.id === connection.id
                            ? 'bg-accent/10 text-accent shadow-sm'
                            : 'text-muted-foreground hover:bg-accent/5 hover:text-foreground'}"
                        onclick={() => onSelectConnection(connection)}
                    >
                        <div
                            class="size-6 shrink-0 flex items-center justify-center grayscale-50 group-hover:grayscale-0"
                        >
                            {#if typeof IconComponent === "function"}
                                <IconComponent />
                            {:else if typeof IconComponent === "string"}
                                <img
                                    src={IconComponent}
                                    alt=""
                                    class="size-4 object-contain"
                                />
                            {:else}
                                <IconDatabase
                                    size={14}
                                    class="text-muted-foreground"
                                />
                            {/if}
                        </div>

                        <div class="flex flex-col min-w-0">
                            <span class="text-sm font-medium truncate"
                                >{connection.name}</span
                            >
                            <span
                                class="text-[10px] opacity-60 truncate capitalize"
                                >{connection.engine}</span
                            >
                        </div>
                    </button>
                {/each}
            </div>
        {/if}
    </div>

    <!-- Quick Feedback Stats -->
    <div class="p-3 border-t border-border bg-accent/5">
        <div
            class="flex items-center justify-between text-[10px] text-muted-foreground opacity-60"
        >
            <span>
                Total Connections:
                <span class="ml-1 text-2xl font-semibold">
                    {connectionStore.state.connections.length}
                </span>
            </span>
        </div>
    </div>
</div>
