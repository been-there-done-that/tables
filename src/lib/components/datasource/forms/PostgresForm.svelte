<script lang="ts">
    import { IconChevronDown, IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import {
        ENGINE_SCHEMAS,
        createEmptyConfig,
        isFieldVisible,
    } from "$lib/schema/connectionSchema";
    import type { PostgresConfig } from "$lib/schema/connectionSchema";

    interface Props {
        data: PostgresConfig;
        onChange: (field: string, value: any) => void;
        errors?: Record<string, string>;
    }

    let { data, onChange, errors = {} }: Props = $props();
    let tab = $state<"general" | "ssh" | "advanced">("general");
    let showAllErrors = $state(false);

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

    // Validation helpers
    function getFieldError(path: string) {
        return errors[path];
    }

    function hasErrors() {
        return Object.keys(errors).length > 0;
    }

    function getVisibleErrors() {
        if (showAllErrors) return errors;

        // Only show errors for visible fields
        const visibleErrors: Record<string, string> = {};
        for (const [field, error] of Object.entries(errors)) {
            if (isFieldVisible("postgres", field, data)) {
                visibleErrors[field] = error;
            }
        }
        return visibleErrors;
    }
</script>

<!-- Error Summary Popup -->
{#if hasErrors()}
    <div class="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg">
        <div class="flex items-center justify-between">
            <div class="flex items-center space-x-2">
                <IconAlertCircle class="w-4 h-4 text-red-500" />
                <span class="text-sm font-medium text-red-800">
                    {Object.keys(getVisibleErrors()).length} validation error(s)
                </span>
            </div>
            <button
                class="text-xs text-red-600 hover:text-red-800"
                onclick={() => (showAllErrors = !showAllErrors)}
            >
                {showAllErrors ? "Show less" : "Show all"}
            </button>
        </div>

        {#if showAllErrors}
            <div class="mt-2 space-y-1">
                {#each Object.entries(getVisibleErrors()) as [field, error]}
                    <div class="text-xs text-red-700">
                        <span class="font-medium">{field}:</span>
                        {error}
                    </div>
                {/each}
            </div>
        {/if}
    </div>
{/if}

<div class="space-y-4 text-sm">
    <div
        class="flex space-x-4 border-b border-(--theme-border-default) pb-2 text-xs font-medium text-(--theme-fg-secondary)"
    >
        <button
            class={`pb-1 border-b-2 ${tab === "general" ? "border-(--theme-accent-primary) text-(--theme-fg-primary)" : "border-transparent hover:border-(--theme-border-default)"}`}
            onclick={() => (tab = "general")}
        >
            General
        </button>
        <button
            class={`pb-1 border-b-2 ${tab === "ssh" ? "border-(--theme-accent-primary) text-(--theme-fg-primary)" : "border-transparent hover:border-(--theme-border-default)"}`}
            onclick={() => (tab = "ssh")}
        >
            SSH
        </button>
        <button
            class={`pb-1 border-b-2 ${tab === "advanced" ? "border-(--theme-accent-primary) text-(--theme-fg-primary)" : "border-transparent hover:border-(--theme-border-default)"}`}
            onclick={() => (tab = "advanced")}
        >
            Advanced
        </button>
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
                    {#if getFieldError("db.host")}
                        <div class="text-xs text-red-600 mt-1">
                            {getFieldError("db.host")}
                        </div>
                    {/if}
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
                        {#if getFieldError("db.port")}
                            <div class="text-xs text-red-600 mt-1">
                                {getFieldError("db.port")}
                            </div>
                        {/if}
                    </div>
                </div>
            </div>

            <label for="db.database" class="text-[--theme-fg-secondary]"
                >Database:</label
            >
            <FormInput
                inputId="db.database"
                value={data.db?.database || ""}
                oninput={(e: any) => updateField("db.database", e.target.value)}
            />
            {#if getFieldError("db.database")}
                <div class="text-xs text-red-600 mt-1">
                    {getFieldError("db.database")}
                </div>
            {/if}

            <label for="db.username" class="text-[--theme-fg-secondary]"
                >User:</label
            >
            <FormInput
                inputId="db.username"
                value={data.db?.username || ""}
                oninput={(e: any) => updateField("db.username", e.target.value)}
            />
            {#if getFieldError("db.username")}
                <div class="text-xs text-red-600 mt-1">
                    {getFieldError("db.username")}
                </div>
            {/if}

            <!-- Transport Type -->
            <label for="transport-type-select" class="text-[--theme-fg-secondary]"
                >Connection:</label
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
            {#if getFieldError("transport.type")}
                <div class="text-xs text-red-600 mt-1">
                    {getFieldError("transport.type")}
                </div>
            {/if}

            <!-- TLS Settings -->
            <label for="tls-enabled" class="text-[--theme-fg-secondary]">TLS:</label>
            <div class="space-y-2">
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
                    <div class="pl-6 space-y-2">
                        <div>
                            <label
                                for="tls-sslmode-select"
                                class="text-[--theme-fg-secondary]"
                                >SSL Mode:</label
                            >
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
                            {#if getFieldError("tls.sslmode")}
                                <div class="text-xs text-red-600 mt-1">
                                    {getFieldError("tls.sslmode")}
                                </div>
                            {/if}
                        </div>
                    </div>
                {/if}
            </div>
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
                {#if getFieldError("transport.ssh.host")}
                    <div class="text-xs text-red-600 mt-1">
                        {getFieldError("transport.ssh.host")}
                    </div>
                {/if}

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
                {#if getFieldError("transport.ssh.port")}
                    <div class="text-xs text-red-600 mt-1">
                        {getFieldError("transport.ssh.port")}
                    </div>
                {/if}

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
                {#if getFieldError("transport.ssh.user")}
                    <div class="text-xs text-red-600 mt-1">
                        {getFieldError("transport.ssh.user")}
                    </div>
                {/if}

                <div>
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
                    {#if getFieldError("transport.ssh.auth.type")}
                        <div class="text-xs text-red-600 mt-1">
                            {getFieldError("transport.ssh.auth.type")}
                        </div>
                    {/if}
                </div>

                {#if data.transport.ssh?.auth?.type === "key"}
                    <div>
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
                        {#if getFieldError("transport.ssh.auth.key_ref")}
                            <div class="text-xs text-red-600 mt-1">
                                {getFieldError("transport.ssh.auth.key_ref")}
                            </div>
                        {/if}
                    </div>
                {:else if data.transport.ssh?.auth?.type === "password"}
                    <div>
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
                        {#if getFieldError("transport.ssh.auth.password_ref")}
                            <div class="text-xs text-red-600 mt-1">
                                {getFieldError(
                                    "transport.ssh.auth.password_ref",
                                )}
                            </div>
                        {/if}
                    </div>
                {/if}
            </div>
        {:else}
            <div class="text-center py-8 text-[--theme-fg-tertiary]">
                <IconAlertCircle class="w-8 h-8 mx-auto mb-2" />
                <p>
                    SSH tunnel is not enabled. Switch to "Direct Connection" in
                    the General tab to use SSH.
                </p>
            </div>
        {/if}
    {:else if tab === "advanced"}
        <div class="space-y-4">
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
                {#if getFieldError("options.search_path")}
                    <div class="text-xs text-red-600 mt-1">
                        {getFieldError("options.search_path")}
                    </div>
                {/if}

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
                {#if getFieldError("options.application_name")}
                    <div class="text-xs text-red-600 mt-1">
                        {getFieldError("options.application_name")}
                    </div>
                {/if}
            </div>
        </div>
    {/if}
</div>
