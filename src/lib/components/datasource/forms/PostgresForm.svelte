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
    import {
        ENGINE_SCHEMAS,
        createEmptyConfig,
        isFieldVisible,
    } from "$lib/schema/connectionSchema";
    import type { PostgresConfig } from "$lib/schema/connectionSchema";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import ConnectionResultPopover from "../ConnectionResultPopover.svelte";
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

    interface Props {
        data: PostgresConfig;
        onChange: (field: string, value: any) => void;
        hideFooter?: boolean;
    }

    let { data, onChange, hideFooter = false }: Props = $props();
    let tab = $state<"general" | "ssh" | "advanced">("general");
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

    // Validation function using schema
    function validateForm(): Record<string, string> {
        const validationErrors: Record<string, string> = {};
        const schema = ENGINE_SCHEMAS.postgres.fields;

        // Manually validate Name since it's outside the engine schema
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

            // Check number ranges (for port fields)
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

    async function handleApply() {
        const errors = validateForm();
        if (Object.keys(errors).length > 0) {
            validationErrors = Object.values(errors);
            return;
        }

        isSaving = true;
        try {
            // 1. Construct RuntimeConnection (PostgresConfig)
            // We just use `data` but wrap it with engine tag
            const runtimeConfig = {
                engine: "postgres",
                ...data,
                db: {
                    ...data.db,
                    password: null,
                },
            };

            const now = Math.floor(Date.now() / 1000);

            // 2. Construct full Connection struct
            const connectionPayload = {
                id: crypto.randomUUID(),
                // @ts-ignore - name might be on data even if not in type
                name: data.name || "Untitled Postgres",
                engine: "postgres",
                host: data.db?.host || null,
                port: data.db?.port || 5432,
                database: data.db?.database || null,
                username: data.db?.username || null,
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
                password: data.db?.password || null,
                ssh_private_key: null, // TODO: Extract if file ref
                ssh_passphrase: null,
                ssl_certificate: null,
                ssl_private_key: null,
                ssl_ca_certificate: null, // data.tls.ca_ref?
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
                "postgres",
                $state.snapshot(data),
            );

            if (response.success && response.data) {
                connectionForm.setTestResult(response.data);
                showPopover = true;
            } else if (!response.success && response.error) {
                // Optional: show error in modal
            }
        } finally {
            isTesting = false;
        }
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
    <!-- Form Content - grows to fill available space -->
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <!-- Classic Raised Tab Style -->
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
                <div
                    role="tab"
                    tabindex="0"
                    class={`px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px ${
                        tab === "advanced"
                            ? "bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]"
                            : "bg-[--theme-bg-tertiary] text-[--theme-fg-secondary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-secondary] border border-transparent"
                    }`}
                    onclick={() => (tab = "advanced")}
                    onkeydown={(e) => e.key === "Enter" && (tab = "advanced")}
                >
                    Advanced
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
                                value={String(data.db?.port || 5432)}
                                placeholder="5432"
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

                <!-- Transport Type -->
                <label
                    for="transport-type-select"
                    class="text-[--theme-fg-secondary]">Connection:</label
                >
                <div class="flex space-x-2">
                    <div class="grow">
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
                    </div>
                </div>

                <!-- TLS Settings -->
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
                        <div class="w-40">
                            <Select
                                id="tls-sslmode-select"
                                value={data.tls?.sslmode || "prefer"}
                                onCommit={(value: string) =>
                                    updateField("tls.sslmode", value)}
                                options={[
                                    { value: "disable", label: "Disable" },
                                    { value: "allow", label: "Allow" },
                                    { value: "prefer", label: "Prefer" },
                                    { value: "require", label: "Require" },
                                    {
                                        value: "verify-ca",
                                        label: "Verify CA",
                                    },
                                    {
                                        value: "verify-full",
                                        label: "Verify Full",
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
                                placeholder="CA certificate reference"
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
                        placeholder="bastion.example.com"
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
                        placeholder="22"
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
                            for="transport.ssh.auth.key_ref">SSH Key Ref:</label
                        >
                        <FormInput
                            inputId="transport.ssh.auth.key_ref"
                            type="password"
                            value={data.transport.ssh.auth?.key_ref || ""}
                            placeholder="Key reference from secure store"
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
                            >Password Ref:</label
                        >
                        <FormInput
                            inputId="transport.ssh.auth.password_ref"
                            type="password"
                            value={data.transport.ssh.auth?.password_ref || ""}
                            placeholder="Password reference from secure store"
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
                    <p>
                        SSH tunnel is not enabled. Switch to "Direct Connection"
                        in the General tab to use SSH.
                    </p>
                </div>
            {/if}
        {:else if tab === "advanced"}
            <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
                <label
                    for="options.search_path"
                    class="text-[--theme-fg-secondary]">Search Path:</label
                >
                <FormInput
                    inputId="options.search_path"
                    value={data.options?.search_path || "public"}
                    oninput={(e: any) =>
                        updateField("options.search_path", e.target.value)}
                />

                <label
                    for="options.application_name"
                    class="text-[--theme-fg-secondary]">Application Name:</label
                >
                <FormInput
                    inputId="options.application_name"
                    value={data.options?.application_name || ""}
                    oninput={(e: any) =>
                        updateField("options.application_name", e.target.value)}
                />
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
                                driverName="PostgreSQL"
                                onClose={() => (showPopover = false)}
                            />
                        {/if}
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
