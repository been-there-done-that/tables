<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import {
        testConnectionParams,
        createConnection,
        updateConnection,
    } from "$lib/commands/client";
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

    function validateForm(): string[] {
        const errors: string[] = [];
        const method = data.auth?.method || "basic";

        if (method === "cloud_id") {
            if (!data.db?.cloud_id) errors.push("Cloud ID is required");
        } else {
            if (!data.db?.host) errors.push("Host is required");
        }

        if (method === "basic") {
            // Username/Password optional? Usually required.
            if (!data.db?.username) errors.push("Username is required");
        } else if (method === "api_key") {
            if (!data.db?.api_key) errors.push("API Key is required");
        }

        return errors;
    }

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    async function handleApply() {
        const errors = validateForm();
        if (errors.length > 0) {
            validationErrors = errors;
            return;
        }

        isSaving = true;
        try {
            // 1. Construct RuntimeConnection (ElasticsearchConfig)
            const runtimeConfig = {
                engine: "elasticsearch",
                ...data,
            };

            const now = Math.floor(Date.now() / 1000);
            const isCloud = data.auth?.method === "cloud_id";

            // 2. Construct full Connection struct
            const connectionPayload = {
                id: crypto.randomUUID(),
                // @ts-ignore
                name: data.name || "Untitled Elasticsearch",
                engine: "elasticsearch",
                host: isCloud ? null : data.db?.host || null,
                port: isCloud ? null : data.db?.port || 9200,
                database: null, // ES doesn't really have a "database" in standard JDBC sense, maybe index? Usually null.
                username:
                    isCloud || data.auth?.method === "api_key"
                        ? null
                        : data.db?.username || null,
                uses_ssh: false,
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
                password:
                    data.auth?.method === "basic"
                        ? data.db?.password || null
                        : null,
                ssh_private_key: null,
                ssh_passphrase: null,
                ssl_certificate: null,
                ssl_private_key: null,
                ssl_ca_certificate: null, // Maybe mapped from ca_fingerprint? Unlikely.
                api_token:
                    data.auth?.method === "api_key"
                        ? data.db?.api_key || null
                        : null,
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

    async function handleTestConnection() {
        const errors = validateForm();
        if (errors.length > 0) {
            validationErrors = errors;
            return;
        }

        showPopover = false;
        connectionForm.setTestResult(null);
        isTesting = true;

        try {
            const response = await testConnectionParams(
                "elasticsearch",
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
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center mt-4">
            <label for="auth.method" class="text-[--theme-fg-secondary]"
                >Auth Method:</label
            >
            <Select
                id="auth.method"
                value={data.auth?.method || "basic"}
                onCommit={(v: string) => onChange("auth.method", v)}
                options={[
                    { value: "basic", label: "Basic Auth" },
                    { value: "api_key", label: "API Key" },
                    { value: "cloud_id", label: "Elastic Cloud ID" },
                ]}
            />

            {#if data.auth?.method === "cloud_id"}
                <label for="db.cloud_id" class="text-[--theme-fg-secondary]"
                    >Cloud ID:</label
                >
                <FormInput
                    inputId="db.cloud_id"
                    value={data.db?.cloud_id || ""}
                    placeholder="deployment-name:abcdef..."
                    oninput={(e: any) =>
                        onChange("db.cloud_id", e.target.value)}
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
                                onChange("db.host", e.target.value)}
                        />
                    </div>
                    <div class="flex items-center space-x-2">
                        <label for="db.port" class="text-[--theme-fg-secondary]"
                            >Port:</label
                        >
                        <div class="w-32">
                            <FormInput
                                inputId="db.port"
                                type="number"
                                value={String(data.db?.port || 9200)}
                                oninput={(e: any) =>
                                    onChange(
                                        "db.port",
                                        parseInt(e.target.value),
                                    )}
                            />
                        </div>
                    </div>
                </div>
            {/if}

            {#if data.auth?.method === "basic"}
                <label for="db.username" class="text-[--theme-fg-secondary]"
                    >Username:</label
                >
                <FormInput
                    inputId="db.username"
                    value={data.db?.username || ""}
                    oninput={(e: any) =>
                        onChange("db.username", e.target.value)}
                />

                <label for="db.password" class="text-[--theme-fg-secondary]"
                    >Password:</label
                >
                <FormInput
                    inputId="db.password"
                    type="password"
                    value={data.db?.password || ""}
                    oninput={(e: any) =>
                        onChange("db.password", e.target.value)}
                />
            {:else if data.auth?.method === "api_key"}
                <label for="db.api_key" class="text-[--theme-fg-secondary]"
                    >API Key:</label
                >
                <FormInput
                    inputId="db.api_key"
                    type="password"
                    value={data.db?.api_key || ""}
                    placeholder="Base64 encoded API key"
                    oninput={(e: any) => onChange("db.api_key", e.target.value)}
                />
            {/if}

            <span class="text-[--theme-fg-secondary]">TLS:</span>
            <div class="space-y-2">
                <label class="flex items-center space-x-2">
                    <input
                        id="elasticsearch-tls"
                        type="checkbox"
                        class="rounded"
                        checked={data.tls?.enabled || false}
                        onchange={(e: any) =>
                            onChange("tls.enabled", e.target.checked)}
                    />
                    <span>Enable HTTPS/TLS</span>
                </label>
                {#if data.tls?.enabled}
                    <div class="flex items-center space-x-2">
                        <label
                            for="tls.ca_fingerprint"
                            class="text-[--theme-fg-secondary] text-xs shrink-0"
                            >CA Fingerprint:</label
                        >
                        <FormInput
                            inputId="tls.ca_fingerprint"
                            value={data.tls?.ca_fingerprint || ""}
                            placeholder="Optional: SHA256 Fingerprint"
                            class="text-xs"
                            oninput={(e: any) =>
                                onChange("tls.ca_fingerprint", e.target.value)}
                        />
                    </div>
                {/if}
            </div>
        </div>
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
                                driverName="Elasticsearch"
                                onClose={() => (showPopover = false)}
                            />
                        {/if}
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
