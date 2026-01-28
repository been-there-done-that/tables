<script lang="ts">
  import DataSourceSidebar from "$lib/components/datasource/DataSourceSidebar.svelte";
  import ConnectionForm from "$lib/components/datasource/ConnectionForm.svelte";
  import { type Driver, drivers } from "$lib/components/datasource/DriverList";
  import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
  import { connectionStore } from "$lib/commands/stores.svelte";
  import { onDestroy, onMount } from "svelte";
  import type { Connection } from "$lib/commands/types";

  let viewMode = $state<"list" | "add" | "edit">("list");
  let selectedConnection = $state<Connection | null>(null);

  const currentDriver = $derived(connectionForm.driver);

  onMount(() => {
    connectionStore.loadConnections();
  });

  function handleSelectConnection(connection: Connection) {
    viewMode = "edit";
    selectedConnection = connection;
    connectionForm.setFromConnection(connection);
  }

  function handleAddNew() {
    viewMode = "add";
    selectedConnection = null;
    connectionForm.reset();
  }

  function handleSelectDriver(driver: Driver) {
    if (driver.status !== "supported") return;
    connectionForm.setDriver(driver);
    // viewMode stays 'add' but now we have a driver
  }

  function handleCancel() {
    viewMode = "list";
    selectedConnection = null;
    connectionForm.reset();
  }

  function handleSave(id: string) {
    // Switch to edit mode for the newly saved connection
    viewMode = "edit";
    // Find the connection in store (it was just added/updated)
    const conn = connectionStore.connections.find((c) => c.id === id);
    if (conn) {
      selectedConnection = conn;
    }
  }

  function handleSaveSuccess() {
    viewMode = "list";
    selectedConnection = null;
    connectionForm.reset();
    connectionStore.loadConnections();
  }

  onDestroy(() => {
    connectionForm.reset();
  });
</script>

<div class="flex flex-col w-full h-full bg-background overflow-hidden">
  <div class="flex grow w-full overflow-hidden">
    <!-- Sidebar -->
    <div class="max-w-60 w-full shrink-0 h-full border-r border-border">
      <DataSourceSidebar
        onSelectConnection={handleSelectConnection}
        onAddNew={handleAddNew}
        {selectedConnection}
      />
    </div>

    <!-- Main Content -->
    <div class="grow h-full bg-background min-w-0">
      {#if currentDriver}
        <ConnectionForm
          driver={currentDriver}
          isEdit={viewMode === "edit"}
          onCancel={handleCancel}
          onSave={handleSave}
          onSaveSuccess={handleSaveSuccess}
        />
      {:else}
        <!-- Home / Driver Selection Grid -->
        <div class="p-8 h-full overflow-y-auto">
          <h2 class="text-xl font-semibold mb-6">Select Database Type</h2>
          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
            {#each drivers as driver}
              {@const IconComponent = driver.icon}
              <button
                class="flex flex-col items-center justify-center p-6 rounded-xl border border-border transition-all
                  {driver.status === 'supported'
                  ? 'hover:border-accent hover:bg-accent/5 cursor-pointer'
                  : 'opacity-50 grayscale cursor-not-allowed bg-muted/20'}"
                onclick={() => handleSelectDriver(driver)}
              >
                <div class="size-12 mb-3 flex items-center justify-center">
                  {#if typeof IconComponent === "function"}
                    <IconComponent />
                  {:else}
                    <img
                      src={IconComponent}
                      alt={driver.name}
                      class="size-full object-contain"
                    />
                  {/if}
                </div>
                <span class="text-sm font-medium">{driver.name}</span>
                {#if driver.status !== "supported"}
                  <span
                    class="mt-2 px-2 py-0.5 text-[10px] uppercase font-bold tracking-wider rounded-full
                        {driver.status === 'coming-soon'
                      ? 'bg-yellow-500/10 text-yellow-500'
                      : 'bg-gray-500/10 text-gray-500'}"
                  >
                    {driver.status.replace("-", " ")}
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
