<script lang="ts">
    import { drivers, type Driver } from "./DriverList";
    import { IconDatabase } from "@tabler/icons-svelte";

    interface Props {
        onSelect: (driver: Driver | null) => void;
        selectedDriver: Driver | null;
    }

    let { onSelect, selectedDriver }: Props = $props();

    function selectDriver(driver: Driver) {
        onSelect(driver);
    }
</script>

<div class="flex flex-col h-full bg-background border-r border-border">
    <!-- Toolbar -->
    <div class="text-center py-1 space-x-1 border-b border-border">Drivers</div>

    <!-- Tree/List Area -->
    <div class="grow overflow-y-auto p-3">
        <div class="space-y-0.5">
            {#each drivers as driver}
                {@const IconComponent = driver.icon}
                <button
                    class="w-full text-left px-3 py-1.5 flex items-center space-x-2 text-sm rounded-sm outline-none transition-colors
                    {selectedDriver?.id === driver.id
                        ? 'bg-accent/10 text-foreground'
                        : 'text-muted-foreground hover:bg-accent/10 hover:text-foreground'}"
                    onclick={() => selectDriver(driver)}
                >
                    {#if typeof IconComponent === "function"}
                        <IconComponent />
                    {:else}
                        <IconDatabase
                            size={14}
                            class="text-(--theme-fg-tertiary)"
                        />
                    {/if}

                    <span class="truncate">{driver.name}</span>
                </button>
            {/each}
        </div>
    </div>
</div>
