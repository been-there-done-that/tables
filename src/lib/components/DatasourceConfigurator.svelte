<script lang="ts">
  import DraggableWindow from "./DraggableWindow.svelte";
  import { cn } from "$lib/utils";
  import Footer from "./Footer.svelte";
  import PostgresConfig from "./datasources/PostgresConfig.svelte";
  import ComingSoon from "./datasources/ComingSoon.svelte";

  const drivers = [
    "postgresql",
    "mysql",
    "sqlite",
    "custom",
  ];

  let selectedDriver = $state("sqlite");


  let {
    open = $bindable(false),
    onClose,
    onTest = undefined,
  } = $props();

  type PostgresHandle = {
    triggerSave: () => void;
    triggerTest: () => void;
    triggerCancel: () => void;
  };

  let postgresRef = $state<PostgresHandle | null>(null);

  function selectDriver(id: string) {
    selectedDriver = id;
  }

  const handleCancel = () => {
    open = false;
    onClose?.();
  };

  const handleSave = (values: Record<string, string>) => {
    console.log("Datasource submit", { driver: selectedDriver, values });
    open = false;
  };

  const handleTest = (values: Record<string, string>) => {
    onTest?.({ driver: selectedDriver, values });
  };
</script>

<DraggableWindow
  bind:open
  onClose={onClose}
  title="Configure datasource"
  modal
  class="max-w-5xl"
  contentClass="p-0"
  openShortcut={undefined}
  closeShortcut={undefined}
  headerActions={undefined}
>
  <div class="flex flex-col h-[660px]">
    <div class="flex flex-1 divide-x divide-(--theme-border-default) min-h-0">
      <aside class="w-[20%] bg-(--theme-bg-secondary) space-y-3 overflow-y-auto">
        <div class="flex w-full items-center gap-2 text-sm font-semibold text-(--theme-fg-secondary)">
          <div class="text-center w-full">Drivers</div>
        </div>
        <div class="space-y-">
          {#each drivers as driver}
            <button
              class={cn(
                "w-full text-left py-2 rounded-lg border border-transparent hover:border-(--theme-border-default) hover:bg-(--theme-bg-primary)",
                selectedDriver === driver && "border-(--theme-border-default) bg-[color-mix(in_srgb,var(--theme-bg-primary)_90%,transparent)]",
              )}
              onclick={() => selectDriver(driver)}
            >
              <div class="flex items-start">
                <div class="h-6 w-6 rounded bg-(--theme-border-default) flex items-center justify-center text-xs text-(--theme-fg-secondary)">{driver[0]}</div>
                <p class="text-sm font-medium text-(--theme-fg-primary)">{driver}</p>
              </div>
            </button>
          {/each}
        </div>
      </aside>

      <section class="w-[80%] flex-1 p-6 space-y-6 overflow-y-auto">

        <div class="space-y-4">
          {#if selectedDriver === "postgresql"}
            <PostgresConfig bind:this={postgresRef} onCancel={handleCancel} onSave={handleSave} onTest={handleTest} />
          {:else if selectedDriver === "mysql"}
            <ComingSoon />
          {:else if selectedDriver === "sqlite"}
            <ComingSoon />
          {:else if selectedDriver === "custom"}
            <ComingSoon />
          {:else}
            <ComingSoon />
          {/if}
        </div>
      </section>
    </div>

    <Footer
      onCancel={() => {
        if (selectedDriver === "postgresql") {
          postgresRef?.triggerCancel();
        } else {
          handleCancel();
        }
      }}
      onTest={() => {
        if (selectedDriver === "postgresql") {
          postgresRef?.triggerTest();
        } else {
          handleTest({});
        }
      }}
      onSave={() => {
        if (selectedDriver === "postgresql") {
          postgresRef?.triggerSave();
        } else {
          handleSave({});
        }
      }}
    />
  </div>
</DraggableWindow>
