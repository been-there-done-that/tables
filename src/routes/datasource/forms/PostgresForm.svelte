<script lang="ts">
    import { IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";

    interface Props {
        data: any; // We'll type this properly later with Zod or similar
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();


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
                />
            </div>

            <div class="flex items-center space-x-2">
                <label for="port" class="text-[--theme-fg-secondary]"
                    >Port:</label
                >
                <div class="w-32">
                    <FormInput
                        inputId="port"
                        type="number"
                        value={data.port}
                        placeholder="5432"
                        oninput={(e: any) =>
                            onChange("port", parseInt(e.target.value))}
                    />
                </div>
            </div>
        </div>

        <label for="user" class="text-[--theme-fg-secondary]">User:</label>
        <FormInput
            inputId="user"
            value={data.username}
            oninput={(e: any) => onChange("username", e.target.value)}
        />

        <label for="password" class="text-[--theme-fg-secondary]"
            >Password:</label
        >
        <div class="flex space-x-2">
            <div class="grow">
                <FormInput
                    inputId="password"
                    type="password"
                    value={data.password}
                    placeholder="<hidden>"
                    oninput={(e: any) => onChange("password", e.target.value)}
                />
            </div>
        </div>

        <label for="database" class="text-[--theme-fg-secondary]"
            >Database:</label
        >
        <FormInput
            inputId="database"
            value={data.database || "postgres"}
            oninput={(e: any) => onChange("database", e.target.value)}
        />
    </div>
</div>
