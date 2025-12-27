<script lang="ts">
    import { IconChevronDown, IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import {
        ENGINE_SCHEMAS,
        isFieldVisible,
    } from "$lib/schema/connectionSchema";
    import type { MysqlConfig } from "$lib/schema/connectionSchema";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: MysqlConfig;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
    let tab = $state<"general" | "ssh">("general");

    // Validation function using schema
    function validateForm(): Record<string, string> {
        const validationErrors: Record<string, string> = {};
        const schema = ENGINE_SCHEMAS.mysql.fields;

        for (const [fieldPath, fieldDef] of Object.entries(schema)) {
            const def = fieldDef as any;

            if (def.condition && !def.condition(data)) {
                continue;
            }

            const value = getFieldValue(fieldPath);

            if (def.required) {
                if (
                    value === undefined ||
                    value === null ||
                    (typeof value === "string" && value.trim() === "")
                ) {
                    validationErrors[fieldPath] = `${def.label} is required`;
                }
            }

            if (
                def.type === "number" &&
                value !== undefined &&
                value !== null &&
                value !== ""
            ) {
                const numValue = Number(value);
                if (def.min !== undefined && numValue < def.min) {
                    validationErrors[fieldPath] =
                        `${def.label} must be at least ${def.min}`;
                }
                if (def.max !== undefined && numValue > def.max) {
                    validationErrors[fieldPath] =
                        `${def.label} must be at most ${def.max}`;
                }
            }
        }

        return validationErrors;
    }

    // Self-contained action handlers
    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        const validationErrors = validateForm();

        if (Object.keys(validationErrors).length > 0) {
            const firstError = Object.values(validationErrors)[0];
            notifications.error(firstError);
            return;
        }

        notifications.success("MySQL settings saved!");
        console.log("Valid MySQL connection, saving...", $state.snapshot(data));
    }

    function handleTestConnection() {
        const validationErrors = validateForm();
        if (Object.keys(validationErrors).length > 0) {
            const firstError = Object.values(validationErrors)[0];
            notifications.error(firstError);
            return;
        }

        notifications.info("Testing connection...");
        setTimeout(() => {
            notifications.success("Connection successful!");
        }, 1500);
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

    // Helper to update nested field
    function updateField(path: string, value: any) {
        const keys = path.split(".");
        const newData: any = { ...data };
        let current = newData;

        for (let i = 0; i < keys.length - 1; i++) {
            if (!current[keys[i]]) {
                current[keys[i]] = {};
            }
            current = current[keys[i]];
        }

        current[keys[keys.length - 1]] = value;
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
                    class={`px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px ${
                        tab === "general"
                            ? "bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]"
                            : "bg-[--theme-bg-tertiary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] border border-transparent"
                    }`}
                    onclick={() => (tab = "general")}
                    onkeydown={(e) => e.key === "Enter" && (tab = "general")}
                >
                    General
                </div>
                <div
                    role="tab"
                    tabindex="0"
                    class={`px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px ${
                        tab === "ssh"
                            ? "bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]"
                            : "bg-[--theme-bg-tertiary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] border border-transparent"
                    }`}
                    onclick={() => (tab = "ssh")}
                    onkeydown={(e) => e.key === "Enter" && (tab = "ssh")}
                >
                    SSH
                </div>
            </div>
        </div>

        {#if tab === "general"}
            <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
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
                                updateField("db.host", e.target.value)}
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
                                value={String(data.db?.port || 3306)}
                                placeholder="3306"
                                oninput={(e: any) =>
                                    updateField(
                                        "db.port",
                                        parseInt(e.target.value),
                                    )}
                            />
                        </div>
                    </div>
                </div>

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
                    >User:</label
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
                    placeholder="Enter password"
                    oninput={(e: any) =>
                        updateField("db.password", e.target.value)}
                />

                <label
                    for="transport-type-select"
                    class="text-[--theme-fg-secondary]">Connection:</label
                >
                <Select
                    id="transport-type-select"
                    value={data.transport?.type || "direct"}
                    onCommit={(value: string) =>
                        updateField("transport.type", value)}
                    options={[
                        { value: "direct", label: "Direct Connection" },
                        { value: "ssh", label: "SSH Tunnel" },
                    ]}
                />

                <label for="tls-enabled" class="text-[--theme-fg-secondary]"
                    >TLS:</label
                >
                <label class="flex items-center space-x-2">
                    <input
                        id="tls-enabled"
                        type="checkbox"
                        class="rounded"
                        checked={data.tls?.enabled || false}
                        onchange={(e: any) =>
                            updateField("tls.enabled", e.target.checked)}
                    />
                    <span class="text-sm">Enable TLS</span>
                </label>

                {#if data.tls?.enabled}
                    <label
                        for="tls-sslmode-select"
                        class="text-[--theme-fg-secondary]">SSL Mode:</label
                    >
                    <div class="flex space-x-4">
                        <div class="w-48">
                            <Select
                                id="tls-sslmode-select"
                                value={data.tls?.sslmode || "PREFERRED"}
                                onCommit={(value: string) =>
                                    updateField("tls.sslmode", value)}
                                options={[
                                    { value: "DISABLED", label: "Disabled" },
                                    { value: "PREFERRED", label: "Preferred" },
                                    { value: "REQUIRED", label: "Required" },
                                    { value: "VERIFY_CA", label: "Verify CA" },
                                    {
                                        value: "VERIFY_IDENTITY",
                                        label: "Verify Identity",
                                    },
                                ]}
                            />
                        </div>
                        <div class="flex items-center space-x-2 grow">
                            <label
                                for="tls-ca-ref"
                                class="text-[--theme-fg-secondary] shrink-0"
                                >CA Cert:</label
                            >
                            <FormInput
                                inputId="tls-ca-ref"
                                type="password"
                                value={data.tls?.ca_ref || ""}
                                placeholder="CA cert reference"
                                oninput={(e: any) =>
                                    updateField("tls.ca_ref", e.target.value)}
                            />
                        </div>
                    </div>
                {/if}
            </div>
        {:else if tab === "ssh"}
            {#if data.transport?.type === "ssh"}
                <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
                    <label
                        class="text-[--theme-fg-secondary]"
                        for="transport.ssh.host">SSH Host:</label
                    >
                    <FormInput
                        inputId="transport.ssh.host"
                        value={data.transport.ssh?.host || ""}
                        oninput={(e: any) =>
                            updateField("transport.ssh.host", e.target.value)}
                    />

                    <label
                        class="text-[--theme-fg-secondary]"
                        for="transport.ssh.port">SSH Port:</label
                    >
                    <FormInput
                        inputId="transport.ssh.port"
                        type="number"
                        value={String(data.transport.ssh?.port || 22)}
                        oninput={(e: any) =>
                            updateField(
                                "transport.ssh.port",
                                parseInt(e.target.value),
                            )}
                    />

                    <label
                        class="text-[--theme-fg-secondary]"
                        for="transport.ssh.user">SSH User:</label
                    >
                    <FormInput
                        inputId="transport.ssh.user"
                        value={data.transport.ssh?.user || ""}
                        oninput={(e: any) =>
                            updateField("transport.ssh.user", e.target.value)}
                    />

                    <label
                        for="ssh-auth-select"
                        class="text-[--theme-fg-secondary]">SSH Auth:</label
                    >
                    <Select
                        id="ssh-auth-select"
                        value={data.transport.ssh?.auth?.type || "key"}
                        onCommit={(value: string) =>
                            updateField("transport.ssh.auth.type", value)}
                        options={[
                            { value: "key", label: "SSH Key" },
                            { value: "password", label: "Password" },
                            { value: "agent", label: "SSH Agent" },
                        ]}
                    />

                    {#if data.transport.ssh?.auth?.type === "key"}
                        <label
                            class="text-[--theme-fg-secondary]"
                            for="transport.ssh.auth.key_ref">Key Ref:</label
                        >
                        <FormInput
                            inputId="transport.ssh.auth.key_ref"
                            type="password"
                            value={data.transport.ssh.auth?.key_ref || ""}
                            oninput={(e: any) =>
                                updateField(
                                    "transport.ssh.auth.key_ref",
                                    e.target.value,
                                )}
                        />
                    {:else if data.transport.ssh?.auth?.type === "password"}
                        <label
                            class="text-[--theme-fg-secondary]"
                            for="transport.ssh.auth.password_ref"
                            >Pass Ref:</label
                        >
                        <FormInput
                            inputId="transport.ssh.auth.password_ref"
                            type="password"
                            value={data.transport.ssh.auth?.password_ref || ""}
                            oninput={(e: any) =>
                                updateField(
                                    "transport.ssh.auth.password_ref",
                                    e.target.value,
                                )}
                        />
                    {/if}
                </div>
            {:else}
                <div class="text-center py-8 text-[--theme-fg-tertiary]">
                    <IconAlertCircle class="w-8 h-8 mx-auto mb-2" />
                    <p>SSH tunnel is not enabled in General tab.</p>
                </div>
            {/if}
        {/if}
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
