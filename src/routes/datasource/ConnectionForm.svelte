<script lang="ts">
    import { type Driver } from "../DriverList";
    import PostgresForm from "./forms/PostgresForm.svelte";
    import SqliteForm from "./forms/SqliteForm.svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import { IconMaximize, IconCircleCheck } from "@tabler/icons-svelte";

    interface Props {
        driver: Driver | null;
    }

    let { driver }: Props = $props();

    // Consolidated form data state
    let formData = $state({
        name: "",
        comment: "",
        host: "localhost",
        port: "",
        username: "",
        password: "",
        database: "",
        authType: "password",
        file: "",
    });

    // Reset form data when driver changes
    $effect(() => {
        if (driver) {
            formData.name = driver?.name || "";
            formData.port = driver?.defaultPort || "";
        }
    });

    function handleChange(field: string, value: any) {
        // @ts-ignore
        formData[field] = value;
    }
</script>

{#if driver}
    <div class="flex flex-col h-full bg-[--theme-bg-primary] text-[--theme-fg-secondary]">
        <div class="grow p-6 overflow-y-auto">
            <!-- Top Common Fields -->
            <div
                class="grid grid-cols-[100px_1fr] gap-y-3 gap-x-4 mb-6 items-center text-sm"
            >
                <label for="name" class="text-right">Name:</label>
                <div class="flex items-center space-x-2">
                    <div class="grow">
                        <FormInput
                            inputId="name"
                            value={formData.name}
                            oninput={(e: any) =>
                                handleChange("name", e.target.value)}
                        />
                    </div>
                </div>

            </div>

            <!-- Tabs -->
            <div class="flex space-x-6 border-b border-[--theme-border-default] mb-4 text-sm">
                <button
                    class="pb-2 border-b-2 border-[--theme-accent-primary] text-[--theme-fg-secondary] font-medium"
                    >General</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[--theme-border-default] text-[--theme-fg-secondary]"
                    >Options</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[--theme-border-default] text-[--theme-fg-secondary]"
                    >SSH/SSL</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[--theme-border-default] text-[--theme-fg-secondary]"
                    >Schemas</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[--theme-border-default] text-[--theme-fg-secondary]"
                    >Advanced</button
                >
            </div>

            <!-- Driver Info Line -->
            <div
                class="flex items-center space-x-4 text-xs text-[--theme-fg-tertiary] mb-4"
            >
                <span
                    >Connection type: <span
                        class="text-[--theme-accent-primary] cursor-pointer">default</span
                    ></span
                >
                <span
                    >Driver: <span class="text-[--theme-accent-primary] cursor-pointer"
                        >{driver.name}</span
                    ></span
                >
                <div class="grow"></div>
                <span class="text-[--theme-accent-primary] cursor-pointer">More Options</span>
            </div>

            <!-- Dynamic Form Content -->
            <div class="mt-4">
                {#if driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"}
                    <PostgresForm data={formData} onChange={handleChange} />
                {:else if driver.id === "sqlite"}
                    <SqliteForm data={formData} onChange={handleChange} />
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
    <div class="flex items-center justify-center h-full text-[--theme-fg-secondary]">
        Select a driver to configure
    </div>
{/if}
