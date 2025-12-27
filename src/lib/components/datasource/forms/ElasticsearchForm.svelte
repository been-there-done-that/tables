<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { testConnectionParams } from "$lib/commands/client";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    const testResult = $derived(connectionForm.state.testResult);

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        notifications.success("Elasticsearch settings saved!");
        console.log("Applying Elasticsearch config", $state.snapshot(data));
    }

    async function handleTestConnection() {
        if (!data.db?.host && data.auth?.method !== "cloud_id") {
            notifications.error("Host is required");
            return;
        }
        notifications.info("Testing Elasticsearch connection...");

        const response = await testConnectionParams(
            "elasticsearch",
            $state.snapshot(data),
        );

        if (response.success && response.data) {
            connectionForm.setTestResult(response.data);
            if (response.data.connected) {
                notifications.success(
                    `Connection successful! ${response.data.version || ""}`,
                );
            } else {
                notifications.error(response.data.error || "Connection failed");
            }
        } else {
            notifications.error(response.error || "Test failed");
        }
    }
</script>

<div class="h-full flex flex-col">
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

    <div
        class="shrink-0 flex flex-col border-t border-[--theme-border-default]"
    >
        {#if testResult}
            <div
                class="px-6 py-2 text-[10px] uppercase tracking-wider font-mono flex items-center justify-between bg-[--theme-bg-secondary]/30"
            >
                <div class="flex items-center gap-4">
                    <span class="text-[--theme-fg-tertiary]"
                        >Driver: <span class="text-[--theme-fg-secondary]"
                            >Elasticsearch</span
                        ></span
                    >
                    {#if testResult.connected}
                        <span class="text-[--theme-fg-tertiary]"
                            >Version: <span class="text-[--theme-fg-secondary]"
                                >{testResult.version || "Unknown"}</span
                            ></span
                        >
                        <span class="text-[--theme-fg-tertiary]"
                            >Ping: <span class="text-[--theme-accent-primary]"
                                >{testResult.response_time_ms} ms</span
                            ></span
                        >
                    {:else}
                        <span class="text-red-500/80 font-bold"
                            >Connection Failed: {testResult.error ||
                                "Unknown Error"}</span
                        >
                    {/if}
                </div>
            </div>
        {/if}

        <div class="flex justify-center items-center py-4">
            <div class="flex space-x-3">
                <Button onClick={handleCancel}>Cancel</Button>
                <Button onClick={handleApply}>Apply</Button>
                <Button onClick={handleTestConnection}>Test Connection</Button>
            </div>
        </div>
    </div>
</div>
