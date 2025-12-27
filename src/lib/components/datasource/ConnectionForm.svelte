<script lang="ts">
    import { type Driver } from "./DriverList";
    import PostgresForm from "./forms/PostgresForm.svelte";
    import SqliteForm from "./forms/SqliteForm.svelte";
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

    function handleChange(field: string, value: any) {
        connectionForm.updateField(field as keyof typeof state.fields, value);
    }
</script>

{#if driver}
    <div class="flex flex-col">
        <div class="grow p-6 overflow-y-auto">
            <!-- Top Common Fields -->
            <div
                class="flex w-full gap-x-4 mb-6 items-center text-sm"
            >
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

            <!-- Dynamic Form Content -->
            <div class="mt-4">
                {#if driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"}
                    <PostgresForm data={state.fields} onChange={handleChange} />
                {:else if driver.id === "sqlite"}
                    <SqliteForm data={state.fields} onChange={handleChange} />
                {:else}
                    <div class="text-center text-[--theme-fg-tertiary] mt-10">
                        Configuration for {driver.name} is not yet implemented.
                    </div>
                {/if}
            </div>
        </div>

        <!-- Footer Actions -->
    </div>
{:else}
    <div class="flex items-center justify-center h-full text-[--theme-fg-tertiary]">
      Select a driver to configure
    </div>
{/if}
