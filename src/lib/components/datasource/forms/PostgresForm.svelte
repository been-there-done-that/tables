<script lang="ts">
    import { IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";

    interface Props {
        data: any; // We'll type this properly later with Zod or similar
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
    let tab = $state<"general" | "ssh" | "advanced">("general");
</script>

<div class="space-y-4 text-sm">
    <div class="flex space-x-4 border-b border-(--theme-border-default) pb-2 text-xs font-medium text-(--theme-fg-secondary)">
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
    {:else if tab === "ssh"}
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center">
            <label class="text-[--theme-fg-secondary]" for="ssh_host">SSH Host:</label>
            <FormInput
                inputId="ssh_host"
                value={data.ssh_host}
                placeholder="bastion.example.com"
                oninput={(e: any) => onChange("ssh_host", e.target.value)}
            />

            <label class="text-[--theme-fg-secondary]" for="ssh_port">SSH Port:</label>
            <FormInput
                inputId="ssh_port"
                type="number"
                value={data.ssh_port}
                placeholder="22"
                oninput={(e: any) => onChange("ssh_port", parseInt(e.target.value))}
            />

            <label class="text-[--theme-fg-secondary]" for="ssh_user">SSH User:</label>
            <FormInput
                inputId="ssh_user"
                value={data.ssh_user}
                oninput={(e: any) => onChange("ssh_user", e.target.value)}
            />

            <label class="text-[--theme-fg-secondary]" for="ssh_key">SSH Private Key:</label>
            <FormInput
                inputId="ssh_key"
                type="password"
                value={data.ssh_key}
                placeholder="<hidden>"
                oninput={(e: any) => onChange("ssh_key", e.target.value)}
            />
        </div>
    {:else if tab === "advanced"}
        <div class="space-y-3 text-sm text-[--theme-fg-secondary]">
            <div class="text-[--theme-fg-tertiary]">Advanced settings placeholder</div>
            <FormInput
                inputId="search_path"
                label="Search path"
                value={data.search_path}
                oninput={(e: any) => onChange("search_path", e.target.value)}
            />
            <FormInput
                inputId="application_name"
                label="Application name"
                value={data.application_name}
                oninput={(e: any) => onChange("application_name", e.target.value)}
            />
        </div>
    {/if}
</div>
