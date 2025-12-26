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
        name: driver?.name || "",
        comment: "",
        host: "localhost",
        port: driver?.defaultPort,
        username: "",
        password: "",
        database: "",
        authType: "password",
        file: "",
    });

    // Reset form data when driver changes (optional, or keep some fields)
    $effect(() => {
        if (driver) {
            formData.name = driver.name; // Reset name to driver name default
            formData.port = driver.defaultPort;
        }
    });

    function handleChange(field: string, value: any) {
        // @ts-ignore
        formData[field] = value;
    }
</script>

{#if driver}
    <div class="flex flex-col h-full bg-[#1e1f22] text-[#bbbbbb]">
        <div class="flex-grow p-6 overflow-y-auto">
            <!-- Top Common Fields -->
            <div
                class="grid grid-cols-[100px_1fr] gap-y-3 gap-x-4 mb-6 items-center text-sm"
            >
                <label for="name" class="text-right">Name:</label>
                <div class="flex items-center space-x-2">
                    <div class="flex-grow">
                        value={formData.name}
                        oninput={(e: any) =>
                            handleChange("name", e.target.value)}
                        class="grow bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6]
                        focus:border-[#3574f0]" /> />
                    </div>
                    <button
                        class="text-[#3574f0] text-xs hover:underline whitespace-nowrap"
                        >Create DDL Mapping</button
                    >
                </div>

                <label for="comment" class="text-right">Comment:</label>
                <div class="relative">
                    <textarea
                        id="comment"
                        bind:value={formData.comment}
                        rows="1"
                        class="w-full bg-[#2b2d30] border border-[#5e6060] rounded-md px-2 py-1.5 text-[#a9b7c6] focus:border-[#3574f0] outline-none resize-none text-sm"
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
                <div class="grow"></div>
                <span class="text-[#3574f0] cursor-pointer">More Options</span>
            </div>

            <!-- Dynamic Form Content -->
            <div class="mt-4">
                {#if driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"}
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
    </div>
{:else}
    <div class="flex items-center justify-center h-full text-[#bbbbbb]">
        Select a driver to configure
    </div>
{/if}
