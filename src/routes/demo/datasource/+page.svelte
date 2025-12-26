<script lang="ts">
    import DatasourceDialog from "$lib/components/datasource/DatasourceDialog.svelte";
    import Button from "$lib/components/Button.svelte";

    let showDialog = $state(false);
    let lastSaved = $state<{ engine: string; data: any } | null>(null);

    function handleSave(engine: string, data: any) {
        lastSaved = { engine, data };
        console.log("Saved:", engine, data);
    }
</script>

<div
    class="p-10 flex flex-col items-center justify-center min-h-screen bg-(--theme-bg-primary) text-(--theme-fg-primary)"
>
    <h1 class="text-2xl font-bold mb-8">Datasource UI Architect Demo</h1>

    <Button onClick={() => (showDialog = true)} height="10">
        Open Datasource Config
    </Button>

    {#if lastSaved}
        <div
            class="mt-8 p-4 bg-(--theme-bg-secondary) border border-(--theme-border-default) rounded-md max-w-lg w-full"
        >
            <h3
                class="text-xs font-semibold uppercase text-(--theme-fg-tertiary) mb-2"
            >
                Last Saved Config ({lastSaved.engine})
            </h3>
            <pre class="text-[10px] overflow-auto">{JSON.stringify(
                    lastSaved.data,
                    null,
                    2,
                )}</pre>
        </div>
    {/if}

    <DatasourceDialog bind:open={showDialog} onSave={handleSave} />
</div>
