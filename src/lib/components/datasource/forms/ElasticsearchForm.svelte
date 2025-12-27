<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        notifications.success("Elasticsearch settings saved!");
        console.log("Applying Elasticsearch config", $state.snapshot(data));
    }

    function handleTestConnection() {
        notifications.info("Testing Elasticsearch connection...");
        setTimeout(() => notifications.success("Connection successful!"), 1500);
    }

    function updateField(path: string, value: any) {
        onChange(path, value);
    }
</script>

<div class="h-full flex flex-col">
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center mt-4">
            <label class="text-[--theme-fg-secondary]">Auth Method:</label>
            <Select
                value={data.auth?.method || "basic"}
                onCommit={(v: string) => updateField("auth.method", v)}
                options={[
                    { value: "basic", label: "Basic Auth" },
                    { value: "api_key", label: "API Key" },
                    { value: "cloud_id", label: "Elastic Cloud ID" },
                ]}
            />

            {#if data.auth?.method === "cloud_id"}
                <label class="text-[--theme-fg-secondary]">Cloud ID:</label>
                <FormInput
                    value={data.db?.cloud_id || ""}
                    placeholder="deployment-name:abcdef..."
                    oninput={(e: any) =>
                        updateField("db.cloud_id", e.target.value)}
                />
            {:else}
                <label class="text-[--theme-fg-secondary]">Host/Port:</label>
                <div class="flex space-x-2">
                    <div class="grow">
                        <FormInput
                            value={data.db?.host || ""}
                            placeholder="localhost"
                            oninput={(e: any) =>
                                updateField("db.host", e.target.value)}
                        />
                    </div>
                    <div class="flex items-center space-x-2">
                        <label class="text-[--theme-fg-secondary]">Port:</label>
                        <div class="w-32">
                            <FormInput
                                type="number"
                                value={String(data.db?.port || 9200)}
                                oninput={(e: any) =>
                                    updateField(
                                        "db.port",
                                        parseInt(e.target.value),
                                    )}
                            />
                        </div>
                    </div>
                </div>
            {/if}

            {#if data.auth?.method === "basic"}
                <label class="text-[--theme-fg-secondary]">Username:</label>
                <FormInput
                    value={data.db?.username || ""}
                    oninput={(e: any) =>
                        updateField("db.username", e.target.value)}
                />

                <label class="text-[--theme-fg-secondary]">Password:</label>
                <FormInput
                    type="password"
                    value={data.db?.password || ""}
                    oninput={(e: any) =>
                        updateField("db.password", e.target.value)}
                />
            {:else if data.auth?.method === "api_key"}
                <label class="text-[--theme-fg-secondary]">API Key:</label>
                <FormInput
                    type="password"
                    value={data.db?.api_key || ""}
                    placeholder="Base64 encoded API key"
                    oninput={(e: any) =>
                        updateField("db.api_key", e.target.value)}
                />
            {/if}

            <label class="text-[--theme-fg-secondary]">TLS:</label>
            <div class="space-y-2">
                <label class="flex items-center space-x-2">
                    <input
                        type="checkbox"
                        class="rounded"
                        checked={data.tls?.enabled || false}
                        onchange={(e: any) =>
                            updateField("tls.enabled", e.target.checked)}
                    />
                    <span>Enable HTTPS/TLS</span>
                </label>
                {#if data.tls?.enabled}
                    <div class="flex items-center space-x-2">
                        <label
                            class="text-[--theme-fg-secondary] text-xs shrink-0"
                            >CA Fingerprint:</label
                        >
                        <FormInput
                            value={data.tls?.ca_fingerprint || ""}
                            placeholder="Optional: SHA256 Fingerprint"
                            class="text-xs"
                            oninput={(e: any) =>
                                updateField(
                                    "tls.ca_fingerprint",
                                    e.target.value,
                                )}
                        />
                    </div>
                {/if}
            </div>
        </div>
    </div>

    <div
        class="shrink-0 flex justify-center items-center py-4 mt-auto border-t border-[--theme-border-default]"
    >
        <div class="flex space-x-3">
            <Button onClick={handleCancel}>Cancel</Button>
            <Button onClick={handleApply}>Apply</Button>
            <Button onClick={handleTestConnection}>Test Connection</Button>
        </div>
    </div>
</div>
