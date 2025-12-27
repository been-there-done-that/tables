<script lang="ts">
    import { drivers, type Driver } from "./DriverList";
    import {
        IconDatabase,
    } from "@tabler/icons-svelte";

    interface Props {
        onSelect: (driver: Driver | null) => void;
        selectedDriver: Driver | null;
    }

    let { onSelect, selectedDriver }: Props = $props();

    function selectDriver(driver: Driver) {
        onSelect(driver);
    }
</script>

<div class="flex flex-col h-full bg-(--theme-bg-) border-r border-(--theme-border-default)">
    <!-- Toolbar -->
    <div class="text-center py-1 space-x-1 border-b border-(--theme-border-default)">
        Drivers
    </div>

    <!-- Tree/List Area -->
    <div class="grow overflow-y-auto p-3">
        <div class="space-y-0.5">
            {#each drivers as driver}
                {@const IconComponent = driver.icon}
                <button
                    class="w-full text-left px-3 py-1.5 flex items-center space-x-2 text-sm rounded-md
                    {selectedDriver?.id === driver.id
                        ? 'bg-(--theme-accent-primary) text-white [text-shadow:0_1px_2px_rgba(0,0,0,0.45)] hover:bg-[color-mix(in_srgb,var(--theme-accent-primary)_78%,black_22%)] focus-visible:ring-offset-2 focus-visible:ring-offset-(--theme-bg-primary)'
                        : 'text-(--theme-fg-secondary) hover:bg-(--theme-bg-hover)'}"
                    onclick={() => selectDriver(driver)}
                >
                    {#if typeof IconComponent === 'function'}
                        <IconComponent />
                    {:else}
                        <IconDatabase size={14} class="text-(--theme-fg-tertiary)" />
                    {/if}

                    <span class="truncate">{driver.name}</span>
                </button>
            {/each}
        </div>
    </div>
</div>
