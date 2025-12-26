<script lang="ts">
  import DraggableWindow from "./DraggableWindow.svelte";
  import { cn } from "$lib/utils";
  import Button from "./Button.svelte";
  import PostgresConfig from "./datasources/PostgresConfig.svelte";
  import ComingSoon from "./datasources/ComingSoon.svelte";
  import Database from "@tabler/icons-svelte/icons/database";

  const drivers = [
    { id: "postgresql", label: "PostgreSQL" },
    { id: "mysql", label: "MySQL" },
    { id: "sqlite", label: "SQLite" },
    { id: "custom", label: "Custom" },
  ];

  let selectedDriver = $state("sqlite");


  let {
    open = $bindable(false),
    onClose,
    onTest = undefined,
  } = $props();

  function defaultValuesFor(driver: string): Record<string, string> {
    if (driver === "postgresql") {
      return { host: "", port: "", database: "", user: "", password: "" };
    }
    return {} as Record<string, string>;
  }

  let formValues = $state<Record<string, string>>(defaultValuesFor(selectedDriver));

  function selectDriver(id: string) {
    selectedDriver = id;
    formValues = defaultValuesFor(id);
  }

  function handleSubmit() {
    const payload = { driver: selectedDriver, values: formValues };
    console.log("Datasource submit", payload);
    open = false;
  }

  function handleTest() {
    const payload = { driver: selectedDriver, values: formValues };
    onTest?.(payload);
  }

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
      <aside class="w-[20%] bg-(--theme-bg-secondary) p-4 space-y-3 overflow-y-auto">
        <div class="flex items-center gap-2 text-sm font-semibold text-(--theme-fg-secondary)">
          <span>Drivers</span>
          <div class="divide-y-2 h-1 w-full"></div>
        </div>
        <div class="space-y-1">
          {#each drivers as driver}
            <button
              class={cn(
                "w-full text-left px-3 py-2 rounded-lg border border-transparent hover:border-(--theme-border-default) hover:bg-(--theme-bg-primary)",
                selectedDriver === driver.id && "border-(--theme-border-default) bg-[color-mix(in_srgb,var(--theme-bg-primary)_90%,transparent)]",
              )}
              onclick={() => selectDriver(driver.id)}
            >
              <div class="flex items-center gap-2">
                <div class="h-6 w-6 rounded bg-(--theme-border-default) flex items-center justify-center text-xs text-(--theme-fg-secondary)">{driver.label[0]}</div>
                <p class="text-sm font-medium text-(--theme-fg-primary)">{driver.label}</p>
              </div>
            </button>
          {/each}
        </div>
      </aside>

      <section class="w-[80%] flex-1 p-6 space-y-6 overflow-y-auto">
        <header class="flex items-start justify-between">
          <div>
            <p class="text-xs uppercase tracking-wide text-(--theme-fg-secondary)">Selected driver</p>
            <h3 class="text-lg font-semibold text-(--theme-fg-primary)">{drivers.find((d) => d.id === selectedDriver)?.label}</h3>
          </div>
        </header>

        <div class="space-y-4">
          {#if selectedDriver === "postgresql"}
            <PostgresConfig bind:values={formValues} />
          {:else}
            <ComingSoon />
          {/if}
        </div>
      </section>
    </div>

    <footer class="flex items-center justify-between pt-4 border-t border-(--theme-border-default) px-6 pb-4">
      <div></div>
      <div class="flex gap-2">
        <Button
          variant="outline"
          onClick={() => {
            open = false;
            onClose?.();
          }}
        >
          Cancel
        </Button>

        <Button
          variant="outline"
          onClick={() => {
            onTest?.({ driver: selectedDriver, values: formValues });
          }}
        >
          Test connection
        </Button>

        <Button
          variant="solid"
          onClick={handleSubmit}
        >
          Save
        </Button>
      </div>
    </footer>
  </div>
</DraggableWindow>
