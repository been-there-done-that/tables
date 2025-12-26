<script lang="ts">
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";
    import Button from "$lib/components/Button.svelte";
    import Select from "$lib/components/Select.svelte";
    import SqlitePanel from "./panels/SqlitePanel.svelte";
    import ConfigField from "./fields/ConfigField.svelte";
    import { cn } from "$lib/utils";

    let { open = $bindable(false), onSave = () => {} } = $props();

    const engines = [
        { value: "sqlite", label: "SQLite" },
        { value: "postgres", label: "PostgreSQL" },
        { value: "mysql", label: "MySQL" },
        { value: "duckdb", label: "DuckDB" },
    ];

    let selectedEngine = $state("sqlite");

    // Isolated state per engine as requested
    let sqliteConfig = $state({
        name: "",
        path: "",
        readOnly: false,
        mustExist: false,
        busyTimeout: 5000,
        foreignKeys: true,
    });

    // Placeholder for future engines
    let postgresConfig = $state({ host: "localhost", port: 5432 });

    const handleSave = () => {
        // Collect data based on engine
        const data =
            selectedEngine === "sqlite" ? sqliteConfig : postgresConfig;
        onSave(selectedEngine, data);
        open = false;
    };

    const handleCancel = () => {
        open = false;
    };

    const handleTest = () => {
        console.log("Testing connection for", selectedEngine);
    };
</script>

<DraggableWindow
    title="Datasource Configuration"
    bind:open
    modal={true}
    class="w-[480px] h-auto"
    contentClass="p-0 flex flex-col"
>
    <!-- Header / Engine Selector -->
    <div
        class="px-5 py-4 border-b border-(--theme-border-default) bg-(--theme-bg-primary)"
    >
        <ConfigField label="Driver Type" labelWidth="w-24">
            <Select
                value={selectedEngine}
                options={engines}
                onCommit={(val) => (selectedEngine = val)}
                class="w-full"
            />
        </ConfigField>
    </div>

    <!-- Main Config Area -->
    <div class="px-5 py-6 flex-1 overflow-y-auto max-h-[400px]">
        {#if selectedEngine === "sqlite"}
            <SqlitePanel bind:config={sqliteConfig} />
        {:else}
            <div
                class="flex flex-col items-center justify-center py-12 text-(--theme-fg-tertiary)"
            >
                <p class="text-xs italic">
                    Configuration for {selectedEngine} is coming soon.
                </p>
            </div>
        {/if}
    </div>

    <!-- Footer Actions -->
    <div
        class="px-5 py-3 border-t border-(--theme-border-default) bg-(--theme-bg-primary) flex items-center justify-between"
    >
        <Button variant="outline" height="8" onClick={handleTest}>
            Test Connection
        </Button>

        <div class="flex items-center gap-2">
            <Button variant="ghost" height="8" onClick={handleCancel}>
                Cancel
            </Button>
            <Button variant="solid" height="8" onClick={handleSave}>
                Save
            </Button>
        </div>
    </div>
</DraggableWindow>
