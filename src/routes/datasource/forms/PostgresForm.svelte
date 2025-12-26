<script lang="ts">
    import { IconChevronDown } from "@tabler/icons-svelte";

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
            <input
                type="text"
                id="host"
                value={data.host || "localhost"}
                oninput={(e) => onChange("host", e.currentTarget.value)}
                class="flex-grow bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
            />

            <div class="flex items-center space-x-2">
                <label for="port" class="text-[#bbbbbb]">Port:</label>
                <input
                    type="number"
                    id="port"
                    value={data.port || 5432}
                    oninput={(e) =>
                        onChange("port", parseInt(e.currentTarget.value))}
                    class="w-20 bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
                />
            </div>
        </div>

        <label for="auth" class="text-[#bbbbbb]">Authentication:</label>
        <div class="relative">
            <select
                id="auth"
                class="w-full bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] appearance-none focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
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
        <input
            type="text"
            id="user"
            value={data.username || ""}
            oninput={(e) => onChange("username", e.currentTarget.value)}
            class="w-full bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
        />

        <label for="password" class="text-[#bbbbbb]">Password:</label>
        <div class="flex space-x-2">
            <input
                type="password"
                id="password"
                value={data.password || ""}
                oninput={(e) => onChange("password", e.currentTarget.value)}
                placeholder="<hidden>"
                class="flex-grow bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none placeholder-gray-500"
            />
            <div class="flex items-center space-x-1 w-auto min-w-[120px]">
                <span class="text-[#bbbbbb] whitespace-nowrap">Save:</span>
                <div class="relative w-full">
                    <select
                        class="w-full bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] appearance-none text-xs focus:border-[#3574f0] outline-none"
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
        <input
            type="text"
            id="database"
            value={data.database || "postgres"}
            oninput={(e) => onChange("database", e.currentTarget.value)}
            class="w-full bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#a9b7c6] focus:border-[#3574f0] focus:ring-1 focus:ring-[#3574f0] outline-none"
        />

        <label for="url" class="text-[#bbbbbb]">URL:</label>
        <div
            class="flex items-center bg-[#2b2d30] border border-[#5e6060] rounded px-2 py-1 text-[#808080] italic cursor-not-allowed"
        >
            <span class="truncate"
                >jdbc:postgresql://{data.host || "localhost"}:{data.port ||
                    5432}/{data.database || "postgres"}</span
            >
        </div>
    </div>
</div>
