<script lang="ts">
    import { IconChevronDown, IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { testConnectionParams } from "$lib/commands/client";
    import { ENGINE_SCHEMAS } from "$lib/schema/connectionSchema";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
    let tab = $state<"general" | "ssh">("general");

    const testResult = $derived(connectionForm.state.testResult);

    function validateForm(): Record<string, string> {
        const validationErrors: Record<string, string> = {};
        const schema = ENGINE_SCHEMAS.mongodb.fields;

        for (const [fieldPath, fieldDef] of Object.entries(schema)) {
            const def = fieldDef as any;
            if (def.condition && !def.condition(data)) continue;

            const value = getFieldValue(fieldPath);
            if (
                def.required &&
                (!value || (typeof value === "string" && value.trim() === ""))
            ) {
                validationErrors[fieldPath] = `${def.label} is required`;
            }
        }
        return validationErrors;
    }

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        const validationErrors = validateForm();
        if (Object.keys(validationErrors).length > 0) {
            notifications.error(Object.values(validationErrors)[0]);
            return;
        }
        notifications.success("MongoDB settings saved!");
        console.log("Applying MongoDB config", $state.snapshot(data));
    }

    async function handleTestConnection() {
        const validationErrors = validateForm();
        if (Object.keys(validationErrors).length > 0) {
            notifications.error(Object.values(validationErrors)[0]);
            return;
        }
        notifications.info("Testing MongoDB connection...");

        const response = await testConnectionParams(
            "mongodb",
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

    function getFieldValue(path: string) {
        const keys = path.split(".");
        let current: any = data;
        for (const key of keys) current = current?.[key];
        return current;
    }

    function updateField(path: string, value: any) {
        onChange(path, value);
    }
</script>

<div class="h-full flex flex-col">
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
                            >MongoDB</span
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
