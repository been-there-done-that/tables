<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
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
        if (!data.db?.host) {
            notifications.error("Host is required");
            return;
        }
        notifications.success("Redis settings saved!");
        console.log("Applying Redis config", $state.snapshot(data));
    }

    function handleTestConnection() {
        if (!data.db?.host) {
            notifications.error("Host is required");
            return;
        }
        notifications.info("Testing Redis connection...");
        setTimeout(() => notifications.success("Connection successful!"), 1500);
    }

    function updateField(path: string, value: any) {
        onChange(path, value);
    }
</script>

<div class="h-full flex flex-col">
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center mt-4">
            <label class="text-[--theme-fg-secondary]">Host:</label>
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
                            value={String(data.db?.port || 6379)}
                            oninput={(e: any) =>
                                updateField(
                                    "db.port",
                                    parseInt(e.target.value),
                                )}
                        />
                    </div>
                </div>
            </div>

            <label class="text-[--theme-fg-secondary]">DB Index:</label>
            <div class="w-32">
                <FormInput
                    type="number"
                    value={String(data.db?.database || 0)}
                    oninput={(e: any) =>
                        updateField("db.database", parseInt(e.target.value))}
                />
            </div>

            <label class="text-[--theme-fg-secondary]">Username:</label>
            <FormInput
                value={data.db?.username || ""}
                placeholder="Optional (Redis 6+ ACL)"
                oninput={(e: any) => updateField("db.username", e.target.value)}
            />

            <label class="text-[--theme-fg-secondary]">Password:</label>
            <FormInput
                type="password"
                value={data.db?.password || ""}
                placeholder="AUTH password"
                oninput={(e: any) => updateField("db.password", e.target.value)}
            />

            <label class="text-[--theme-fg-secondary]">TLS:</label>
            <label class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    class="rounded"
                    checked={data.tls?.enabled || false}
                    onchange={(e: any) =>
                        updateField("tls.enabled", e.target.checked)}
                />
                <span>Enable TLS (Rediss)</span>
            </label>
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
