<script lang="ts">
    import { type Driver } from "./DriverList";
    import PostgresForm from "./forms/PostgresForm.svelte";
    import MysqlForm from "./forms/MysqlForm.svelte";
    import SqliteForm from "./forms/SqliteForm.svelte";
    import MongodbForm from "./forms/MongodbForm.svelte";
    import RedisForm from "./forms/RedisForm.svelte";
    import ElasticsearchForm from "./forms/ElasticsearchForm.svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";

    interface Props {
        driver: Driver | null;
    }

    let { driver }: Props = $props();

    const state = $derived(connectionForm.state);

    $effect(() => {
        if (driver) {
            connectionForm.setDriver(driver);
        }
    });

    function handleChange(path: string, value: any) {
        connectionForm.updateField(path, value);
    }
</script>

{#if driver}
    <div class="flex flex-col h-full">
        <!-- Top Common Fields - fixed height -->
        <div class="shrink-0 p-6 pb-0">
            <div class="flex w-full gap-x-4 mb-6 items-center text-sm">
                <label for="name" class="text-right">Name:</label>
                <div class="flex items-center w-full">
                    <div class="grow">
                        <FormInput
                            inputId="name"
                            value={state.fields.name}
                            oninput={(e: any) =>
                                handleChange("name", e.target.value)}
                        />
                    </div>
                </div>
            </div>
        </div>

        <!-- Dynamic Form Content - takes remaining height -->
        <div class="grow px-6 min-h-0">
            {#if driver.id === "postgresql"}
                <PostgresForm
                    data={state.fields as any}
                    onChange={handleChange}
                />
            {:else if driver.id === "mysql"}
                <MysqlForm data={state.fields as any} onChange={handleChange} />
            {:else if driver.id === "sqlite"}
                <SqliteForm
                    data={state.fields as any}
                    onChange={handleChange}
                />
            {:else if driver.id === "mongodb"}
                <MongodbForm
                    data={state.fields as any}
                    onChange={handleChange}
                />
            {:else if driver.id === "redis"}
                <RedisForm data={state.fields as any} onChange={handleChange} />
            {:else if driver.id === "elasticsearch"}
                <ElasticsearchForm
                    data={state.fields as any}
                    onChange={handleChange}
                />
            {:else}
                <div class="text-center text-muted-foreground mt-10">
                    Configuration for {driver.name} is not yet implemented.
                </div>
            {/if}
        </div>
    </div>
{:else}
    <div class="flex items-center justify-center h-full text-muted-foreground">
        Select a driver to configure
    </div>
{/if}
