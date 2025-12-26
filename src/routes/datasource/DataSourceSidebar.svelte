<script lang="ts">
    import { drivers, type Driver } from "./DriverList";
    import {
        IconSearch,
        IconPlus,
        IconMinus,
        IconCopy,
        IconDatabase,
    } from "@tabler/icons-svelte";

    interface Props {
        onSelect: (driver: Driver | null) => void;
        selectedDriver: Driver | null;
    }

    let { onSelect, selectedDriver }: Props = $props();

    let searchQuery = $state("");

    let filteredDrivers = $derived(
        drivers.filter((d) =>
            d.name.toLowerCase().includes(searchQuery.toLowerCase()),
        ),
    );

    function selectDriver(driver: Driver) {
        onSelect(driver);
    }
</script>

<div class="flex flex-col h-full bg-[#2b2d30] border-r border-[#1e1f22]">
    <!-- Toolbar -->
    <div class="flex items-center p-2 space-x-1 border-b border-[#1e1f22]">
        <button
            class="p-1 hover:bg-[#393b40] rounded text-gray-400 hover:text-gray-200 transition-colors"
            title="Add"
        >
            <IconPlus size={16} />
        </button>
        <button
            class="p-1 hover:bg-[#393b40] rounded text-gray-400 hover:text-gray-200 transition-colors"
            title="Remove"
        >
            <IconMinus size={16} />
        </button>
        <button
            class="p-1 hover:bg-[#393b40] rounded text-gray-400 hover:text-gray-200 transition-colors"
            title="Duplicate"
        >
            <IconCopy size={16} />
        </button>
        <div class="grow"></div>
        <button
            class="p-1 hover:bg-[#393b40] rounded text-gray-400 hover:text-gray-200 transition-colors"
            title="Data Sources"
        >
            <IconDatabase size={16} />
        </button>
    </div>

    <!-- Tree/List Area -->
    <div class="grow overflow-y-auto">
        <div
            class="px-2 py-1 text-xs font-bold text-gray-500 uppercase tracking-wider mt-2"
        >
            Complete Support
        </div>

        <div class="space-y-0.5">
            {#each filteredDrivers as driver}
                <button
                    class="w-full text-left px-3 py-1.5 flex items-center space-x-2 text-sm
                    {selectedDriver?.id === driver.id
                        ? 'bg-[#2d4263] text-white'
                        : 'text-[#bbbbbb] hover:bg-[#393b40]'}"
                    onclick={() => selectDriver(driver)}
                >
                    <!-- <svelte:component this={getIcon(driver.icon)} size={14} class="text-gray-400" /> -->
                    <!-- Placeholder Icon -->
                    <IconDatabase size={14} class="text-gray-400" />

                    <span class="truncate">{driver.name}</span>
                </button>
            {/each}
        </div>
    </div>
</div>
