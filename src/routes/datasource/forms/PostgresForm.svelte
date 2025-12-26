<script lang="ts">
    import { IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";

    interface Props {
        data: any; // We'll type this properly later with Zod or similar
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
</script>

<div class="space-y-4 text-sm">
    <!-- General Settings -->
    <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
        <label for="host" class="text-[#bbbbbb]">Host:</label>
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="host"
                    value={data.host}
                    placeholder="localhost"
                    oninput={(e: any) => onChange("host", e.target.value)}
                    class="bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6] focus:border-[#3574f0]"
                />
            </div>

            <div class="flex items-center space-x-2">
                <label for="port" class="text-[#bbbbbb]">Port:</label>
                <div class="w-20">
                    <FormInput
                        inputId="port"
                        type="number"
                        value={data.port}
                        placeholder="5432"
                        oninput={(e: any) =>
                            onChange("port", parseInt(e.target.value))}
                        class="bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6] focus:border-[#3574f0]"
                    />
                </div>
            </div>
        </div>

        <label for="auth" class="text-[#bbbbbb]">Authentication:</label>
        <div class="relative">
            <select
                id="auth"
                class="w-full h-8 rounded-md border border-[#5e6060] bg-[#2b2d30] px-3 py-1.5 text-sm text-[#a9b7c6] appearance-none focus:border-[#3574f0] focus:ring-0 outline-none"
                value={data.authType || "password"}
                onchange={(e) => onChange("authType", e.currentTarget.value)}
            >
                <option value="password">User & Password</option>
                <option value="pg_pass">PgPass</option>
                <option value="os_credentials">OS Credentials</option>
            </select>
            <IconChevronDown
                size={14}
                class="absolute right-2 top-1/2 transform -translate-y-1/2 text-gray-500 pointer-events-none"
            />
        </div>

        <label for="user" class="text-[#bbbbbb]">User:</label>
        <FormInput
            inputId="user"
            value={data.username}
            oninput={(e: any) => onChange("username", e.target.value)}
            class="bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6] focus:border-[#3574f0]"
        />

        <label for="password" class="text-[#bbbbbb]">Password:</label>
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="password"
                    type="password"
                    value={data.password}
                    placeholder="<hidden>"
                    oninput={(e: any) => onChange("password", e.target.value)}
                    class="bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6] focus:border-[#3574f0] placeholder-gray-500"
                />
            </div>
            <div class="flex items-center space-x-1 w-auto min-w-[120px]">
                <span class="text-[#bbbbbb] whitespace-nowrap">Save:</span>
                <div class="relative w-full">
                    <select
                        class="w-full h-8 rounded-md border border-[#5e6060] bg-[#2b2d30] px-2 py-1.5 text-xs text-[#a9b7c6] appearance-none focus:border-[#3574f0] outline-none"
                    >
                        <option>Forever</option>
                        <option>For Session</option>
                        <option>Never</option>
                    </select>
                    <IconChevronDown
                        size={12}
                        class="absolute right-1 top-1/2 transform -translate-y-1/2 text-gray-500 pointer-events-none"
                    />
                </div>
            </div>
        </div>

        <label for="database" class="text-[#bbbbbb]">Database:</label>
        <FormInput
            inputId="database"
            value={data.database || "postgres"}
            oninput={(e: any) => onChange("database", e.target.value)}
            class="bg-[#2b2d30] border-[#5e6060] text-[#a9b7c6] focus:border-[#3574f0]"
        />

        <label for="url" class="text-[#bbbbbb]">URL:</label>
        <div
            class="flex items-center h-8 bg-[#2b2d30] border border-[#5e6060] rounded-md px-3 py-1.5 text-[#808080] italic cursor-not-allowed text-sm"
        >
            <span class="truncate"
                >jdbc:postgresql://{data.host || "localhost"}:{data.port ||
                    5432}/{data.database || "postgres"}</span
            >
        </div>
    </div>
</div>
