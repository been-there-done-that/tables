<script lang="ts">
    import { IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";

    interface Props {
        data: any; // We'll type this properly later with Zod or similar
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    const authOptions = [
        { value: "password", label: "User & Password" },
        { value: "pg_pass", label: "PgPass" },
        { value: "os_credentials", label: "OS Credentials" },
    ];

    const saveOptions = [
        { value: "forever", label: "Forever" },
        { value: "session", label: "For Session" },
        { value: "never", label: "Never" },
    ];
</script>

<div class="space-y-4 text-sm">
    <!-- General Settings -->
    <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
        <label for="host" class="text-[--theme-fg-secondary]">Host:</label>
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="host"
                    value={data.host}
                    placeholder="localhost"
                    oninput={(e: any) => onChange("host", e.target.value)}
                    class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary]"
                />
            </div>

            <div class="flex items-center space-x-2">
                <label for="port" class="text-[--theme-fg-secondary]">Port:</label>
                <div class="w-20">
                    <FormInput
                        inputId="port"
                        type="number"
                        value={data.port}
                        placeholder="5432"
                        oninput={(e: any) =>
                            onChange("port", parseInt(e.target.value))}
                        class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary]"
                    />
                </div>
            </div>
        </div>

        <label for="auth" class="text-[--theme-fg-secondary]">Authentication:</label>
        <div class="relative">
            <Select
                options={authOptions}
                value={data.authType || "password"}
                onCommit={(value: any) => onChange("authType", value)}
                class="w-full bg-[--theme-bg-secondary]"
            />
        </div>

        <label for="user" class="text-[--theme-fg-secondary]">User:</label>
        <FormInput
            inputId="user"
            value={data.username}
            oninput={(e: any) => onChange("username", e.target.value)}
            class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary]"
        />

        <label for="password" class="text-[--theme-fg-secondary]">Password:</label>
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="password"
                    type="password"
                    value={data.password}
                    placeholder="<hidden>"
                    oninput={(e: any) => onChange("password", e.target.value)}
                    class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary] placeholder-[--theme-fg-tertiary]"
                />
            </div>
            <div class="flex items-center space-x-1 w-auto min-w-[120px]">
                <span class="text-[--theme-fg-secondary] whitespace-nowrap">Save:</span>
                <div class="relative w-full">
                    <Select
                        options={saveOptions}
                        value={data.saveMode || "forever"}
                        onCommit={(value: any) => onChange("saveMode", value)}
                        class="w-full bg-[--theme-bg-secondary]"
                    />
                </div>
            </div>
        </div>

        <label for="database" class="text-[--theme-fg-secondary]">Database:</label>
        <FormInput
            inputId="database"
            value={data.database || "postgres"}
            oninput={(e: any) => onChange("database", e.target.value)}
            class="bg-[--theme-bg-secondary] border-[--theme-border-default] text-[--theme-fg-secondary] focus:border-[--theme-accent-primary]"
        />

        <label for="url" class="text-[--theme-fg-secondary]">URL:</label>
        <div
            class="flex items-center h-8 bg-[--theme-bg-secondary] border border-[--theme-border-default] rounded-md px-3 py-1.5 text-[--theme-fg-tertiary] italic cursor-not-allowed text-sm"
        >
            <span class="truncate"
                >jdbc:postgresql://{data.host || "localhost"}:{data.port ||
                    5432}/{data.database || "postgres"}</span
            >
        </div>
    </div>
</div>
