<script lang="ts">
    import { IconChevronDown, IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import {
        testConnectionParams,
        createConnection,
    } from "$lib/commands/client";
    import { ENGINE_SCHEMAS } from "$lib/schema/connectionSchema";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import ConnectionResultPopover from "../ConnectionResultPopover.svelte";
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
        hideFooter?: boolean;
    }

    let { data, onChange, hideFooter = false }: Props = $props();
    let tab = $state<"general" | "ssh">("general");
    let showPopover = $state(false);
    let isTesting = $state(false);
    let isSaving = $state(false);
    let validationErrors = $state<string[]>([]);
    let copiedIndex = $state<number | null>(null);
    const testResult = $derived(connectionForm.state.testResult);

    async function handleCopy(text: string, index: number) {
        try {
            await navigator.clipboard.writeText(text);
            copiedIndex = index;
            setTimeout(() => {
                copiedIndex = null;
            }, 2000);
        } catch (err) {
            console.error("Failed to copy:", err);
        }
    }

    function validateForm(): Record<string, string> {
        const validationErrors: Record<string, string> = {};
        const schema = ENGINE_SCHEMAS.mongodb.fields;

        // @ts-ignore
        if (!data.name && !data.id) {
            // ...
        }

        // Iterate through all schema fields
        for (const [fieldPath, fieldDef] of Object.entries(schema)) {
            const def = fieldDef as any;

            // Skip if field has a condition and isn't visible
            if (def.condition && !def.condition(data)) {
                continue;
            }

            // Get the field value
            const value = getFieldValue(fieldPath);

            // Check required fields
            if (def.required) {
                if (
                    value === undefined ||
                    value === null ||
                    (typeof value === "string" && value.trim() === "")
                ) {
                    validationErrors[fieldPath] = `${def.label} is required`;
                }
            }
        }
        return validationErrors;
    }

    // Helper to get nested field value
    function getFieldValue(path: string) {
        const keys = path.split(".");
        let current: any = data;
        for (const key of keys) {
            current = current?.[key];
        }
        return current;
    }

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    async function handleTestConnection() {
        const errors = validateForm();
        if (Object.keys(errors).length > 0) {
            validationErrors = Object.values(errors);
            return;
        }

        showPopover = false;
        connectionForm.setTestResult(null);
        isTesting = true;

        try {
            const response = await testConnectionParams(
                "mongodb",
                $state.snapshot(data),
            );

            if (response.success && response.data) {
                connectionForm.setTestResult(response.data);
                showPopover = true;
            } else if (!response.success && response.error) {
                // Optional
            }
        } finally {
            isTesting = false;
        }
    }

    async function handleApply() {
        const errors = validateForm();
        if (Object.keys(errors).length > 0) {
            validationErrors = Object.values(errors);
            return;
        }

        isSaving = true;
        try {
            // 1. Construct RuntimeConnection (MongodbConfig)
            const runtimeConfig = {
                engine: "mongodb",
                ...data,
                db: {
                    ...data.db,
                    password: null,
                },
            };

            const now = Math.floor(Date.now() / 1000);
            const isUri = data.auth?.method === "uri";

            // 2. Construct full Connection struct
            const connectionPayload = {
                id: crypto.randomUUID(),
                // @ts-ignore
                name: data.name || "Untitled MongoDB",
                engine: "mongodb",
                host: isUri ? null : data.db?.host || null,
                port: isUri ? null : data.db?.port || 27017,
                database: isUri ? null : data.db?.database || null, // Ideally parse URI for specific DB if known, or leave null
                username: isUri ? null : data.db?.username || null,
                uses_ssh: data.transport?.type === "ssh",
                uses_tls: data.tls?.enabled || false,
                config_json: JSON.stringify(runtimeConfig),
                is_favorite: false,
                color_tag: null,
                created_at: now,
                updated_at: now,
                last_connected_at: null,
                connection_count: 0,
            };

            // 3. Construct Credentials
            const credentialsPayload = {
                password: isUri ? null : data.db?.password || null,
                ssh_private_key: null,
                ssh_passphrase: null,
                ssl_certificate: null,
                ssl_private_key: null,
                ssl_ca_certificate: null,
                api_token: null,
                aws_access_key_id: null,
                aws_secret_access_key: null,
                aws_session_token: null,
            };

            const payload = {
                connection: connectionPayload,
                credentials: credentialsPayload,
            };

            // @ts-ignore
            const response = await createConnection(payload);

            if (response.success) {
                console.log("Saved:", response.data);
                const window = getCurrentWindow();
                await window.close();
            } else {
                validationErrors = [
                    response.error || "Failed to save connection",
                ];
            }
        } catch (err) {
            console.error("Save failed:", err);
            validationErrors = [String(err)];
        } finally {
            isSaving = false;
        }
    }

    function updateField(path: string, value: any) {
        onChange(path, value);
    }
</script>

<div class="h-full flex flex-col">
    {#if validationErrors.length > 0}
        <DraggableWindow
            title="Validation Issue"
            modal={false}
            initialPosition="center"
            openShortcut={undefined}
            closeShortcut={undefined}
            headerActions={undefined}
            headerClass="bg-red-500/5 border-b border-red-500/10"
            titleClass="text-red-400 font-bold uppercase tracking-wider text-[11px]"
            class="w-[420px] shadow-2xl border-red-500/20"
            onClose={() => (validationErrors = [])}
        >
            <div class="px-5 py-4 flex flex-col gap-4">
                <div class="p-3">
                    <ul class="flex flex-col gap-2">
                        {#each validationErrors as error, i}
                            <li
                                class="flex items-start justify-between gap-3 text-xs text-[--theme-fg-secondary] group"
                            >
                                <div class="flex items-start gap-2.5 pt-0.5">
                                    <div
                                        class="mt-1.5 size-1 rounded-full bg-red-500 shrink-0 shadow-[0_0_8px_rgba(239,68,68,0.5)]"
                                    ></div>
                                    <span
                                        class="leading-relaxed select-text cursor-text"
                                        >{error}</span
                                    >
                                </div>
                                <button
                                    onclick={() => handleCopy(error, i)}
                                    class="opacity-0 group-hover:opacity-100 focus:opacity-100 p-1 text-[--theme-fg-tertiary] hover:text-[--theme-fg-primary] hover:bg-white/5 rounded transition-all shrink-0 cursor-pointer"
                                    title="Copy error"
                                >
                                    {#if copiedIndex === i}
                                        <IconCheck
                                            class="size-3.5 text-green-400"
                                        />
                                    {:else}
                                        <IconCopy class="size-3.5" />
                                    {/if}
                                </button>
                            </li>
                        {/each}
                    </ul>
                </div>

                <div class="flex justify-end pt-2">
                    <Button
                        variant="subtle"
                        height="8"
                        onClick={() => (validationErrors = [])}
                    >
                        Dismiss
                    </Button>
                </div>
            </div>
        </DraggableWindow>
    {/if}
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="flex justify-center">
            <div class="flex border-b border-[--theme-border-default]">
                <div
                    role="tab"
                    tabindex="0"
                    class={`px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px ${tab === "general" ? "bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]" : "bg-[--theme-bg-tertiary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] border border-transparent"}`}
                    onclick={() => (tab = "general")}
                    onkeydown={(e) => e.key === "Enter" && (tab = "general")}
                >
                    General
                </div>
                <div
                    role="tab"
                    tabindex="0"
                    class={`px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px ${tab === "ssh" ? "bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]" : "bg-[--theme-bg-tertiary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] border border-transparent"}`}
                    onclick={() => (tab = "ssh")}
                    onkeydown={(e) => e.key === "Enter" && (tab = "ssh")}
                >
                    SSH
                </div>
            </div>
        </div>

        {#if tab === "general"}
            <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
                <span class="text-[--theme-fg-secondary]">Method:</span>
                <Select
                    value={data.auth?.method || "standard"}
                    onCommit={(v: string) => updateField("auth.method", v)}
                    options={[
                        { value: "standard", label: "Standard Parameters" },
                        { value: "uri", label: "Connection URI" },
                    ]}
                />

                {#if data.auth?.method === "uri"}
                    <label for="db.uri" class="text-[--theme-fg-secondary]"
                        >URI:</label
                    >
                    <FormInput
                        inputId="db.uri"
                        value={data.db?.uri || ""}
                        placeholder="mongodb+srv://user:pass@cluster.mongodb.net/db"
                        oninput={(e: any) =>
                            updateField("db.uri", e.target.value)}
                    />
                {:else}
                    <label for="db.host" class="text-[--theme-fg-secondary]"
                        >Host/Port:</label
                    >
                    <div class="flex space-x-2">
                        <div class="grow">
                            <FormInput
                                inputId="db.host"
                                value={data.db?.host || ""}
                                placeholder="localhost"
                                oninput={(e: any) =>
                                    updateField("db.host", e.target.value)}
                            />
                        </div>
                        <div class="flex items-center space-x-2">
                            <label
                                for="db.port"
                                class="text-[--theme-fg-secondary]">Port:</label
                            >
                            <div class="w-32">
                                <FormInput
                                    inputId="db.port"
                                    type="number"
                                    value={String(data.db?.port || 27017)}
                                    oninput={(e: any) =>
                                        updateField(
                                            "db.port",
                                            parseInt(e.target.value),
                                        )}
                                />
                            </div>
                        </div>
                    </div>

                    <span class="text-[--theme-fg-secondary]">Options:</span>
                    <label class="flex items-center space-x-2">
                        <input
                            id="mongodb-srv"
                            type="checkbox"
                            class="rounded"
                            checked={data.db?.srv || false}
                            onchange={(e: any) =>
                                updateField("db.srv", e.target.checked)}
                        />
                        <span>Use SRV Record</span>
                    </label>
                {/if}

                <label for="db.database" class="text-[--theme-fg-secondary]"
                    >Database:</label
                >
                <FormInput
                    inputId="db.database"
                    value={data.db?.database || ""}
                    oninput={(e: any) =>
                        updateField("db.database", e.target.value)}
                />

                <label for="db.username" class="text-[--theme-fg-secondary]"
                    >Username:</label
                >
                <FormInput
                    inputId="db.username"
                    value={data.db?.username || ""}
                    oninput={(e: any) =>
                        updateField("db.username", e.target.value)}
                />

                <label for="db.password" class="text-[--theme-fg-secondary]"
                    >Password:</label
                >
                <FormInput
                    inputId="db.password"
                    type="password"
                    value={data.db?.password || ""}
                    oninput={(e: any) =>
                        updateField("db.password", e.target.value)}
                />

                <label for="db.authSource" class="text-[--theme-fg-secondary]"
                    >Auth DB:</label
                >
                <FormInput
                    inputId="db.authSource"
                    value={data.db?.authSource || "admin"}
                    placeholder="admin"
                    oninput={(e: any) =>
                        updateField("db.authSource", e.target.value)}
                />

                <span class="text-[--theme-fg-secondary]">TLS:</span>
                <label class="flex items-center space-x-2">
                    <input
                        id="mongodb-tls"
                        type="checkbox"
                        class="rounded"
                        checked={data.tls?.enabled || false}
                        onchange={(e: any) =>
                            updateField("tls.enabled", e.target.checked)}
                    />
                    <span>Enable TLS/SSL</span>
                </label>
            </div>
        {:else if tab === "ssh"}
            <div class="text-center py-8 text-[--theme-fg-tertiary]">
                <p>
                    SSH Tunnel configuration for MongoDB is identical to SQL
                    engines.
                </p>
                <p class="text-xs mt-2 italic">
                    (Implementation pending common SSH component refactor)
                </p>
            </div>
        {/if}
    </div>

    {#if !hideFooter}
        <div
            class="shrink-0 flex justify-center items-center py-4 border-t border-[--theme-border-default] relative"
        >
            <div class="flex items-center space-x-6">
                <div class="flex items-center gap-3">
                    <Button onClick={handleCancel}>Cancel</Button>
                    <Button onClick={handleApply}>Save</Button>
                </div>

                <div class="flex items-center gap-3 relative">
                    <div class="relative flex items-center gap-2">
                        <button
                            onclick={handleTestConnection}
                            class="text-sm flex items-center gap-2 underline underline-offset-4 hover:text-[--theme-accent-hover] transition-colors cursor-pointer"
                        >
                            Test Connection
                        </button>

                        {#if testResult && showPopover && !isTesting}
                            <ConnectionResultPopover
                                result={testResult}
                                driverName="MongoDB"
                                onClose={() => (showPopover = false)}
                            />
                        {/if}
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
