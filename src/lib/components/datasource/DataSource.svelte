<script lang="ts">
    import DataSourceSidebar from "$lib/components/datasource/DataSourceSidebar.svelte";
    import ConnectionForm from "$lib/components/datasource/ConnectionForm.svelte";
    import Button from "$lib/components/Button.svelte";
    import { type Driver, drivers } from "$lib/components/datasource/DriverList";
    import { getCurrentWindow } from '@tauri-apps/api/window';

    let selectedDriver = $state<Driver | null>(drivers[2]);

    function handleDriverSelect(driver: Driver | null) {
        selectedDriver = driver;
    }

    async function handleClose() {
        const window = getCurrentWindow();
        await window.close();
    }
    
</script>


<div
    class="flex flex-col w-full h-full bg-[--theme-bg-primary] overflow-hidden"
>
    <div class="flex grow w-full overflow-hidden">
        <!-- Sidebar -->
        <div
            class="max-w-60 w-full shrink-0 h-full border-r border-(--theme-border-default)"
        >
            <DataSourceSidebar onSelect={handleDriverSelect} {selectedDriver} />
        </div>

        <!-- Main Content -->
        <div class="grow h-full bg-[--theme-bg-primary] min-w-0">
            <ConnectionForm driver={selectedDriver} />
        </div>
    </div>

    <!-- Footer Actions -->
    {#if selectedDriver}
        <div
            class="flex items-center py-2 px-6 justify-between mb-3"
        >
            <div class="flex items-center space-x-2">
            </div>

            <div class="flex space-x-3">
                <Button onClick={handleClose}>Cancel</Button>
                <Button>Apply</Button>
                <Button>Test Connection</Button>
            </div>
        </div>
    {:else}
        <div
        ></div>
    {/if}
</div>
