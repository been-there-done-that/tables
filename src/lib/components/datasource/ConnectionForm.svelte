<script lang="ts">
    import { type Driver } from "./DriverList";
    import PostgresForm from "./forms/PostgresForm.svelte";
    import MysqlForm from "./forms/MysqlForm.svelte";
    import SqliteForm from "./forms/SqliteForm.svelte";
    import MongodbForm from "./forms/MongodbForm.svelte";
    import RedisForm from "./forms/RedisForm.svelte";
    import ElasticsearchForm from "./forms/ElasticsearchForm.svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Button from "$lib/components/Button.svelte";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import { connectionStore } from "$lib/commands/stores.svelte";
    import { testConnectionParams } from "$lib/commands/client";
    import {
        IconCheck,
        IconAlertCircle,
        IconDatabase,
    } from "@tabler/icons-svelte";

    interface Props {
        driver: Driver | null;
        isEdit?: boolean;
        onCancel: () => void;
        onSaveSuccess: () => void;
    }

    let { driver, isEdit = false, onCancel, onSaveSuccess }: Props = $props();

    const state = $derived(connectionForm.state);
    let isTesting = $state(false);
    let isSaving = $state(false);
    let error = $state<string | null>(null);
    let successMsg = $state<{ title: string; details?: string } | null>(null);

    function handleChange(path: string, value: any) {
        connectionForm.updateField(path, value);
    }

    async function handleTest() {
        if (!driver) return;
        isTesting = true;
        error = null;
        successMsg = null;

        try {
            const result = await testConnectionParams(
                driver.id,
                $state.snapshot(state.fields),
            );
            if (result.success && result.data) {
                connectionForm.setTestResult(result.data);

                const details = [];
                if (result.data.version)
                    details.push(`v${result.data.version}`);
                if (result.data.response_time_ms)
                    details.push(`${result.data.response_time_ms}ms`);

                successMsg = {
                    title: "Connection Successful",
                    details: details.join(" • "),
                };
            } else {
                error = result.error || "Connection test failed";
            }
        } catch (e: any) {
            error = e.message || "An error occurred during testing";
        } finally {
            isTesting = false;
        }
    }

    async function handleSave() {
        if (!driver) return;
        isSaving = true;
        error = null;
        successMsg = null;

        try {
            const fields = $state.snapshot(state.fields);
            const connectionData = {
                name: fields.name,
                engine: driver.id,
                config_json: JSON.stringify(fields),
                auth_type: "password", // Default
            };

            const credentials = {
                password: fields.password || null,
            };

            if (isEdit && fields.id) {
                await connectionStore.updateConnection(
                    fields.id,
                    connectionData as any,
                    credentials,
                );
            } else {
                await connectionStore.createConnection(
                    connectionData as any,
                    credentials,
                );
            }
            onSaveSuccess();
        } catch (e: any) {
            error = e.message || "Failed to save connection";
        } finally {
            isSaving = false;
        }
    }
</script>

{#if driver}
    <div class="flex flex-col h-full bg-background/50">
        <!-- Top Common Fields -->
        <div class="shrink-0 p-8 pb-4">
            <div class="flex flex-col gap-1 mb-6">
                <h1 class="text-2xl font-bold tracking-tight">
                    {isEdit ? "Edit" : "New"}
                    {driver.name} Connection
                </h1>
                <p class="text-sm text-muted-foreground">
                    Configure your database connection settings below.
                </p>
            </div>

            <div
                class="grid grid-cols-[120px_1fr] gap-4 items-center max-w-2xl"
            >
                <label
                    for="name"
                    class="text-sm font-medium text-muted-foreground"
                    >Connection Name</label
                >
                <FormInput
                    inputId="name"
                    value={state.fields.name}
                    placeholder={`My ${driver.name} Database`}
                    oninput={(e: any) => handleChange("name", e.target.value)}
                />
            </div>
        </div>

        <!-- Dynamic Form Content -->
        <div class="grow px-8 overflow-y-auto min-h-0">
            <div class="max-w-2xl">
                {#if driver.id === "postgres"}
                    <PostgresForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "mysql"}
                    <MysqlForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "sqlite"}
                    <SqliteForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "mongodb"}
                    <MongodbForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "redis"}
                    <RedisForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "elasticsearch"}
                    <ElasticsearchForm
                        data={state.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else}
                    <div
                        class="text-center text-muted-foreground mt-10 p-8 border border-dashed rounded-lg"
                    >
                        Configuration for {driver.name} is not yet implemented.
                    </div>
                {/if}
            </div>
        </div>

        <!-- Footer Actions -->
        <div class="shrink-0 p-8 pt-4 flex flex-col gap-4">
            {#if error}
                <div
                    class="p-3 bg-red-500/10 border border-red-500/20 rounded-md text-red-500 text-xs flex items-start gap-3"
                >
                    <IconAlertCircle size={16} class="shrink-0 mt-0.5" />
                    <div class="flex flex-col gap-1">
                        <span class="font-semibold">Error</span>
                        <span class="opacity-90">{error}</span>
                    </div>
                </div>
            {/if}

            {#if successMsg}
                <div
                    class="p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-500 text-xs flex items-start gap-3"
                >
                    <IconCheck size={16} class="shrink-0 mt-0.5" />
                    <div class="flex flex-col gap-1">
                        <span class="font-semibold">{successMsg.title}</span>
                        {#if successMsg.details}
                            <span class="opacity-90">{successMsg.details}</span>
                        {/if}
                    </div>
                </div>
            {/if}

            <div class="flex items-center justify-between">
                <button
                    onclick={handleTest}
                    disabled={isTesting}
                    class="text-sm font-medium underline underline-offset-4 hover:text-accent transition-colors disabled:opacity-50 cursor-pointer"
                >
                    {isTesting ? "Testing..." : "Test Connection"}
                </button>

                <div class="flex items-center gap-3">
                    <Button variant="ghost" onClick={onCancel}>Cancel</Button>
                    <Button
                        onClick={handleSave}
                        disabled={isSaving}
                        class="min-w-[80px]"
                    >
                        {isSaving ? "Saving..." : "Save"}
                    </Button>
                </div>
            </div>
        </div>
    </div>
{:else}
    <div
        class="flex flex-col items-center justify-center h-full text-muted-foreground p-12"
    >
        <div class="size-20 mb-6 opacity-10">
            <IconDatabase size={80} />
        </div>
        <h3 class="text-lg font-medium text-foreground mb-2">
            No Driver Selected
        </h3>
        <p class="max-w-xs text-center text-sm">
            Please pick a database type from the list to start configuring your
            connection.
        </p>
    </div>
{/if}
