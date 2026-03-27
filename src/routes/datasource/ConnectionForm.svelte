<script lang="ts">
    import { type Driver } from "./DriverList";
    import PostgresForm from "./forms/PostgresForm.svelte";
    import SqliteForm from "./forms/SqliteForm.svelte";
    import ProviderForm from "./forms/ProviderForm.svelte";
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
        port: undefined as number | undefined,
        username: "",
        password: "",
        database: "",
        authType: "password",
        file: "",
    });

    // Reset form data when driver changes (optional, or keep some fields)
    $effect(() => {
        if (driver) {
            formData.name = driver.name;
            formData.port = driver.defaultPort;
            // For provider drivers, clear host so the user pastes their URI
            if (driver.provider) {
                formData.host = "";
            } else {
                formData.host = "localhost";
            }
        } else {
            formData.name = "";
            formData.port = undefined;
            formData.host = "localhost";
        }
    });

    function handleChange(field: string, value: any) {
        // @ts-ignore
        formData[field] = value;
    }
</script>

{#if driver}
    <div class="flex flex-col h-full bg-[#1e1f22] text-[#bbbbbb]">
        <!-- Header -->
        <div
            class="flex items-center p-4 border-b border-[#1e1f22] bg-[#2b2d30]"
        >
            <div class="text-sm font-semibold">Data Sources and Drivers</div>
        </div>

        <div class="flex-grow p-6 overflow-y-auto">
            <!-- Top Common Fields -->
            <div
                class="grid grid-cols-[100px_1fr] gap-y-3 gap-x-4 mb-6 items-center text-sm"
            >
                <label for="name" class="text-right">Name:</label>
                <div class="flex items-center space-x-2">
                    <input
                        type="text"
                        id="name"
                        bind:value={formData.name}
                        class="flex-grow bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1.5 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
                    />
                    <button class="text-[#3574f0] text-xs hover:underline"
                        >Create DDL Mapping</button
                    >
                </div>

                <label for="comment" class="text-right">Comment:</label>
                <div class="relative">
                    <textarea
                        id="comment"
                        bind:value={formData.comment}
                        rows="1"
                        class="w-full bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1.5 text-[#a9b7c6] focus:border-[#3574f0] outline-none resize-none"
                    ></textarea>
                    <IconMaximize
                        size={12}
                        class="absolute right-2 top-2 text-gray-500 cursor-pointer"
                    />
                </div>
            </div>

            <!-- Tabs -->
            <div class="flex space-x-6 border-b border-[#323232] mb-4 text-sm">
                <button
                    class="pb-2 border-b-2 border-[#3574f0] text-[#a9b7c6] font-medium"
                    >General</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[#5e6060] text-[#bbbbbb]"
                    >Options</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[#5e6060] text-[#bbbbbb]"
                    >SSH/SSL</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[#5e6060] text-[#bbbbbb]"
                    >Schemas</button
                >
                <button
                    class="pb-2 border-b-2 border-transparent hover:border-[#5e6060] text-[#bbbbbb]"
                    >Advanced</button
                >
            </div>

            <!-- Driver Info Line -->
            <div
                class="flex items-center space-x-4 text-xs text-[#909090] mb-4"
            >
                <span
                    >Connection type: <span
                        class="text-[#3574f0] cursor-pointer">default</span
                    ></span
                >
                <span
                    >Driver: <span class="text-[#3574f0] cursor-pointer"
                        >{driver.name}</span
                    ></span
                >
                <div class="flex-grow"></div>
                <span class="text-[#3574f0] cursor-pointer">More Options</span>
            </div>

            <!-- Dynamic Form Content -->
            <div class="mt-4">
                {#if driver.provider}
                    <ProviderForm
                        providerId={driver.provider}
                        data={formData}
                        onChange={handleChange}
                    />
                {:else if driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"}
                    <PostgresForm data={formData} onChange={handleChange} />
                {:else if driver.id === "sqlite"}
                    <SqliteForm data={formData} onChange={handleChange} />
                {:else}
                    <div class="text-center text-gray-500 mt-10">
                        Configuration for {driver.name} is not yet implemented.
                    </div>
                {/if}
            </div>
        </div>

        <!-- Footer Actions -->
        <div
            class="flex items-center justify-between p-4 bg-[#2b2d30] border-t border-[#323232] text-sm"
        >
            <div class="flex items-center space-x-2">
                <button class="text-[#3574f0] hover:underline"
                    >Test Connection</button
                >
                {#if driver.id === "postgresql"}
                    <span class="text-gray-500 text-xs">PostgreSQL 16.11</span>
                {/if}
            </div>

            <div class="flex space-x-3">
                <button
                    class="px-4 py-1.5 rounded border border-[#5e6060] text-[#a9b7c6] hover:bg-[#393b40] transition-colors"
                    >Cancel</button
                >
                <button
                    class="px-4 py-1.5 rounded border border-[#5e6060] text-[#a9b7c6] hover:bg-[#393b40] transition-colors"
                    >Apply</button
                >
                <button
                    class="px-4 py-1.5 rounded bg-[#3574f0] text-white hover:bg-[#3369d6] border border-[#3574f0] transition-colors shadow-sm"
                    >OK</button
                >
            </div>
        </div>
    </div>
{:else}
    <div class="flex items-center justify-center h-full text-[#bbbbbb]">
        Select a driver to configure
    </div>
{/if}
