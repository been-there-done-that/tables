<script lang="ts">
  import DataSourceSidebar from "./DataSourceSidebar.svelte";
  import ConnectionForm from "./ConnectionForm.svelte";
  import { type Driver } from "./DriverList";

  let selectedDriver = $state<Driver | null>(null);

  function handleDriverSelect(driver: Driver | null) {
    selectedDriver = driver;
  }
</script>

<div class="flex w-full h-full bg-[#1e1f22] overflow-hidden">
  <!-- Sidebar -->
  <div class="w-[300px] flex-shrink-0 h-full">
    <DataSourceSidebar onSelect={handleDriverSelect} {selectedDriver} />
  </div>

  <!-- Main Content -->
  <div class="flex-grow h-full bg-[#1e1f22]">
    <ConnectionForm driver={selectedDriver} />
  </div>
</div>
