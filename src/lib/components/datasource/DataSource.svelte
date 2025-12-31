<script lang="ts">
  import DataSourceSidebar from "$lib/components/datasource/DataSourceSidebar.svelte";
  import ConnectionForm from "$lib/components/datasource/ConnectionForm.svelte";
  import { type Driver, drivers } from "$lib/components/datasource/DriverList";
  import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
  import { onDestroy } from "svelte";

  let selectedDriver = $state<Driver | null>(drivers[2]);

  function handleDriverSelect(driver: Driver | null) {
    selectedDriver = driver;
    connectionForm.setDriver(driver);
  }

  $effect(() => {
    connectionForm.setDriver(selectedDriver);
  });

  onDestroy(() => {
    connectionForm.reset();
  });
</script>

<div class="flex flex-col w-full h-full bg-background overflow-hidden">
  <div class="flex grow w-full overflow-hidden">
    <!-- Sidebar -->
    <div class="max-w-60 w-full shrink-0 h-full border-r border-border">
      <DataSourceSidebar onSelect={handleDriverSelect} {selectedDriver} />
    </div>

    <!-- Main Content -->
    <div class="grow h-full bg-background min-w-0">
      <ConnectionForm driver={selectedDriver} />
    </div>
  </div>
</div>
