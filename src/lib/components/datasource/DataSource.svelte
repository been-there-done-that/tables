<script lang="ts">
  import DataSourceSidebar from "$lib/components/datasource/DataSourceSidebar.svelte";
  import ConnectionForm from "$lib/components/datasource/ConnectionForm.svelte";
  import { type Driver, drivers } from "$lib/components/datasource/DriverList";
  import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
  import { connectionStore } from "$lib/commands/stores.svelte";
  import { onDestroy, onMount } from "svelte";
  import type { Connection } from "$lib/commands/types";

  let viewMode = $state<"list" | "add" | "edit">("list");
  let selectedDriver = $state<Driver | null>(null);
  let selectedConnection = $state<Connection | null>(null);

  onMount(() => {
    connectionStore.loadConnections();
  });

  function handleSelectConnection(connection: Connection) {
    viewMode = "edit";
    selectedConnection = connection;
    selectedDriver = drivers.find((d) => d.id === connection.engine) || null;
    connectionForm.setFromConnection(connection);
  }

  function handleAddNew() {
    viewMode = "add";
    selectedConnection = null;
    selectedDriver = null;
    connectionForm.reset();
  }

  function handleSelectDriver(driver: Driver) {
    if (driver.status !== "supported") return;
    selectedDriver = driver;
    connectionForm.setDriver(driver);
    // viewMode stays 'add' but now we have a driver
  }

  function handleCancel() {
    viewMode = "list";
    selectedConnection = null;
    selectedDriver = null;
    connectionForm.reset();
  }

  function handleSaveSuccess() {
    viewMode = "list";
    selectedConnection = null;
    selectedDriver = null;
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
      {#if viewMode === "add" && !selectedDriver}
        <!-- Driver Selection Grid -->
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
      {:else if selectedDriver}
        <ConnectionForm
          driver={selectedDriver}
          isEdit={viewMode === "edit"}
          onCancel={handleCancel}
          onSaveSuccess={handleSaveSuccess}
        />
      {:else}
        <div
          class="flex flex-col items-center justify-center h-full text-muted-foreground"
        >
          <div class="size-16 mb-4 opacity-20 text-foreground">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125"
              />
            </svg>
          </div>
          <p>Select a connection or add a new one</p>
          <button
            class="mt-4 px-4 py-2 bg-accent text-accent-foreground rounded-md hover:bg-accent/90 transition-colors"
            onclick={handleAddNew}
          >
            Add New Connection
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>
