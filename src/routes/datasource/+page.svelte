<script lang="ts">
  import DataSourceSidebar from "./DataSourceSidebar.svelte";
  import ConnectionForm from "./ConnectionForm.svelte";
  import Button from "$lib/components/Button.svelte";
  import { type Driver } from "./DriverList";

  let selectedDriver = $state<Driver | null>(null);

  function handleDriverSelect(driver: Driver | null) {
    selectedDriver = driver;
  }
</script>

<div class="flex flex-col w-full h-full bg-[#1e1f22] overflow-hidden">
  <div class="flex grow w-full overflow-hidden">
    <!-- Sidebar -->
    <div class="w-[300px] shrink-0 h-full border-r border-[#1e1f22]">
      <DataSourceSidebar onSelect={handleDriverSelect} {selectedDriver} />
    </div>

    <!-- Main Content -->
    <div class="grow h-full bg-[#1e1f22] min-w-0">
      <ConnectionForm driver={selectedDriver} />
    </div>
  </div>

  <!-- Footer Actions -->
  {#if selectedDriver}
    <div
      class="flex items-center justify-between p-4 bg-[#2b2d30] border-t border-[#323232] text-sm shrink-0"
    >
      <div class="flex items-center space-x-2">
        <button class="text-[#3574f0] hover:underline">Test Connection</button>
        {#if selectedDriver.id === "postgresql"}
          <span class="text-gray-500 text-xs">PostgreSQL 16.11</span>
        {/if}
      </div>

      <div class="flex space-x-3">
        <Button
          variant="outline"
          class="border-[#5e6060] text-[#a9b7c6] hover:bg-[#393b40]"
          >Cancel</Button
        >
        <Button
          variant="outline"
          class="border-[#5e6060] text-[#a9b7c6] hover:bg-[#393b40]"
          >Apply</Button
        >
        <Button
          variant="solid"
          class="bg-[#3574f0] text-white hover:bg-[#3369d6] border border-[#3574f0]"
          >OK</Button
        >
      </div>
    </div>
  {:else}
    <div class="h-12 bg-[#2b2d30] border-t border-[#323232] shrink-0"></div>
  {/if}
</div>
