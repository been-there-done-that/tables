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
    import { windowState } from "$lib/stores/window.svelte";
    import { testConnectionParams } from "$lib/commands/client";
    import {
        IconCheck,
        IconAlertCircle,
        IconDatabase,
        IconTrash,
        IconLoader2,
    } from "@tabler/icons-svelte";

    interface Props {
        driver: Driver | null;
        isEdit?: boolean;
        onCancel: () => void;
        onSave: (id: string) => void;
        onSaveSuccess: () => void;
    }

    let {
        driver,
        isEdit = false,
        onCancel,
        onSave,
        onSaveSuccess,
    }: Props = $props();

    const formState = $derived(connectionForm.state);
    let isTesting = $state(false);
    let isSaving = $state(false);
    let isConfirmingDelete = $state(false);

    // Reset confirmation state when switching connections
    $effect(() => {
        formState.fields.id;
        isConfirmingDelete = false;
    });

    const isSessionActive = $derived(
        formState.fields.id
            ? !!windowState.sessions.find(
                  (s) => s.connectionId === formState.fields.id,
              )
            : false,
    );

    function handleChange(path: string, value: any) {
        connectionForm.updateField(path, value);
    }

    async function handleTest() {
        if (!driver) return;
        isTesting = true;
        connectionForm.setStatus({ type: "testing", message: "Connecting..." });

        try {
            const result = await testConnectionParams(
                driver.id,
                $state.snapshot(formState.fields),
            );
            if (result.success && result.data) {
                connectionForm.setTestResult(result.data);

                const details = [];
                if (result.data.version)
                    details.push(`v${result.data.version}`);
                if (result.data.response_time_ms)
                    details.push(`${result.data.response_time_ms}ms`);

                connectionForm.setStatus({
                    type: "success",
                    message: "Connection Successful",
                    details: details.join(" • "),
                });
            } else {
                connectionForm.setStatus({
                    type: "error",
                    message: result.error || "Connection test failed",
                });
            }
        } catch (e: any) {
            connectionForm.setStatus({
                type: "error",
                message: e.message || "An error occurred during testing",
            });
        } finally {
            isTesting = false;
        }
    }

    async function handleSave() {
        if (!driver) return;
        isSaving = true;
        connectionForm.setStatus({ type: "idle", message: "" });

        try {
            const fields: any = $state.snapshot(formState.fields);

            // Helper to extract common fields
            const getField = (path: string, fallback: any = null) => {
                const keys = path.split(".");
                let val: any = fields;
                for (const key of keys) {
                    val = val?.[key];
                }
                return val ?? fallback;
            };

            const now = Math.floor(Date.now() / 1000);
            const connectionData: any = {
                id: isEdit && fields.id ? fields.id : crypto.randomUUID(),
                name: fields.name || `My ${driver.name} Database`,
                engine: driver.id,
                config_json: JSON.stringify(fields),
                connection_params: fields,
                auth_type: "password",
                host: getField("db.host"),
                port: getField("db.port")
                    ? Number(getField("db.port"))
                    : driver.defaultPort,
                database: getField("db.database") || getField("db.uri") || "",
                username: getField("db.username") || "",
                uses_ssh: getField("transport.type") === "ssh",
                uses_tls: getField("tls.enabled", false),
                ssl_enabled: getField("tls.enabled", false),
                ssh_tunnel_enabled: getField("transport.type") === "ssh",
                is_favorite: fields.is_favorite || false,
                connection_count: fields.connection_count || 0,
                created_at: fields.created_at || now,
                updated_at: now,
            };

            const credentials = {
                password: fields.password || getField("db.password") || null,
            };

            if (isEdit && fields.id) {
                await connectionStore.updateConnection(
                    fields.id,
                    connectionData,
                    credentials,
                );
                onSave(fields.id);
            } else {
                const response = await connectionStore.createConnection(
                    connectionData,
                    credentials,
                );
                if (response?.success && response.data) {
                    // Update form to edit mode with new ID
                    connectionForm.updateField("id", response.data);
                    onSave(response.data);
                }
            }
            connectionForm.setStatus({
                type: "success",
                message: "Connection saved successfully",
            });
        } catch (e: any) {
            connectionForm.setStatus({
                type: "error",
                message: e.message || "Failed to save connection",
            });
        } finally {
            isSaving = false;
        }
    }

    async function handleOpenConnection() {
        const id = formState.fields.id;
        if (!id) return;

        const conn = connectionStore.connections.find((c) => c.id === id);
        if (conn) {
            windowState.startSession(conn);
        }
    }

    async function handleDelete() {
        if (!isEdit || !formState.fields.id) return;

        if (!isConfirmingDelete) {
            isConfirmingDelete = true;
            setTimeout(() => (isConfirmingDelete = false), 3000);
            return;
        }

        try {
            const id = formState.fields.id;

            // Close session if active
            const session = windowState.sessions.find(
                (s) => s.connectionId === id,
            );
            if (session) {
                windowState.closeSession(session.id);
            }

            await connectionStore.deleteConnection(id);
            onSaveSuccess();
        } catch (e: any) {
            connectionForm.setStatus({
                type: "error",
                message: e.message || "Failed to delete connection",
            });
            isConfirmingDelete = false;
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
                    value={formState.fields.name}
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
                        data={formState.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "mysql"}
                    <MysqlForm
                        data={formState.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "sqlite"}
                    <SqliteForm
                        data={formState.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "mongodb"}
                    <MongodbForm
                        data={formState.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "redis"}
                    <RedisForm
                        data={formState.fields as any}
                        onChange={handleChange}
                        hideFooter
                    />
                {:else if driver.id === "elasticsearch"}
                    <ElasticsearchForm
                        data={formState.fields as any}
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
            {#if formState.status.type === "error"}
                <div
                    class="p-3 bg-red-500/10 border border-red-500/20 rounded-md text-red-500 text-xs flex items-start gap-3"
                >
                    <IconAlertCircle size={16} class="shrink-0 mt-0.5" />
                    <div class="flex flex-col gap-1">
                        <span class="font-semibold">Error</span>
                        <span class="opacity-90"
                            >{formState.status.message}</span
                        >
                    </div>
                </div>
            {:else if formState.status.type === "success"}
                <div
                    class="p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-500 text-xs flex items-start gap-3"
                >
                    <IconCheck size={16} class="shrink-0 mt-0.5" />
                    <div class="flex flex-col gap-1">
                        <span class="font-semibold"
                            >{formState.status.message}</span
                        >
                        {#if formState.status.details}
                            <span class="opacity-90"
                                >{formState.status.details}</span
                            >
                        {/if}
                    </div>
                </div>
            {/if}

            <div class="flex items-center justify-between">
                <div class="flex items-center gap-6">
                    <button
                        onclick={handleTest}
                        disabled={isTesting}
                        class="text-sm font-medium underline underline-offset-4 hover:text-accent transition-colors disabled:opacity-50 cursor-pointer flex items-center gap-2"
                    >
                        {#if isTesting}
                            <IconLoader2 size={14} class="animate-spin" />
                        {/if}
                        {isTesting ? "Testing..." : "Test Connection"}
                    </button>

                    {#if isEdit}
                        <button
                            onclick={handleDelete}
                            class="text-sm font-medium transition-colors cursor-pointer flex items-center gap-1.5 {isConfirmingDelete
                                ? 'text-red-500 font-bold'
                                : 'text-red-500/70 hover:text-red-500'}"
                        >
                            {#if !isConfirmingDelete}
                                <IconTrash size={14} />
                            {/if}
                            {isConfirmingDelete
                                ? "Click here again to delete"
                                : "Delete"}
                        </button>
                    {/if}
                </div>

                <div class="flex items-center gap-3">
                    {#if formState.fields.id}
                        <Button variant="subtle" onClick={handleOpenConnection}>
                            {isSessionActive
                                ? "Focus Connection"
                                : "Open Connection"}
                        </Button>
                    {/if}
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
