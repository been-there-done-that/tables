<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { testConnectionParams } from "$lib/commands/client";

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
            return;
        }
        console.log("Applying Redis config", $state.snapshot(data));
    }

    async function handleTestConnection() {
        if (!data.db?.host) {
            return;
        }

        const response = await testConnectionParams(
            "redis",
            $state.snapshot(data),
        );
    }
</script>

<div class="h-full flex flex-col">
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center mt-4">
            <label for="db.host" class="text-[--theme-fg-secondary]"
                >Host:</label
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
                            value={String(data.db?.port || 6379)}
                            oninput={(e: any) =>
                                onChange("db.port", parseInt(e.target.value))}
                        />
                    </div>
                </div>
            </div>

            <label for="db.database" class="text-[--theme-fg-secondary]"
                >DB Index:</label
            >
            <div class="w-32">
                <FormInput
                    inputId="db.database"
                    type="number"
                    value={String(data.db?.database || 0)}
                    oninput={(e: any) =>
                        onChange("db.database", parseInt(e.target.value))}
                />
            </div>

            <label for="db.username" class="text-[--theme-fg-secondary]"
                >Username:</label
            >
            <FormInput
                inputId="db.username"
                value={data.db?.username || ""}
                placeholder="Optional (Redis 6+ ACL)"
                oninput={(e: any) => onChange("db.username", e.target.value)}
            />

            <label for="db.password" class="text-[--theme-fg-secondary]"
                >Password:</label
            >
            <FormInput
                inputId="db.password"
                type="password"
                value={data.db?.password || ""}
                placeholder="AUTH password"
                oninput={(e: any) => onChange("db.password", e.target.value)}
            />

            <span class="text-[--theme-fg-secondary]">TLS:</span>
            <label class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    class="rounded"
                    checked={data.tls?.enabled || false}
                    onchange={(e: any) =>
                        onChange("tls.enabled", e.target.checked)}
                />
                <span>Enable TLS (Rediss)</span>
            </label>
        </div>
    </div>

    <div
        class="shrink-0 flex justify-center items-center py-4 border-t border-[--theme-border-default]"
    >
        <div class="flex space-x-3">
            <Button onClick={handleCancel}>Cancel</Button>
            <Button onClick={handleApply}>Apply</Button>
            <Button onClick={handleTestConnection}>Test Connection</Button>
        </div>
    </div>
</div>
