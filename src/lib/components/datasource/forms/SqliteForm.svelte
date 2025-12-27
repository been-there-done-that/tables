<script lang="ts">
    import { IconFolder, IconChevronDown } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { testConnectionParams } from "$lib/commands/client";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();

    async function browseFile() {
        // Mock browse for now
    }

    // Self-contained action handlers
    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        if (data.mode === "file" && !data.file) {
            notifications.error("Database file is required");
            return;
        }
        notifications.success("SQLite settings saved!");
        console.log("Apply SQLite connection", data);
    }

    async function handleTestConnection() {
        if (data.mode === "file" && !data.file) {
            notifications.error("File path is required for file mode");
            return;
        }

        notifications.info("Testing SQLite connection...");

        console.debug(
            "Testing SQLite connection with payload",
            $state.snapshot(data),
        );
        const response = await testConnectionParams(
            "sqlite",
            $state.snapshot(data),
        );

        if (response.success && response.data) {
            connectionForm.setTestResult(response.data);
        }
    }

    const testResult = $derived(connectionForm.state.testResult);
</script>

<div class="h-full flex flex-col">
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="flex justify-center">
            <div class="flex border-b border-[--theme-border-default]">
                <div
                    role="tab"
                    tabindex="0"
                    class="px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]"
                >
                    General
                </div>
            </div>
        </div>

        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center px-6">
            <label for="mode" class="text-[--theme-fg-secondary]">Mode:</label>
            <div class="w-48">
                <Select
                    id="mode"
                    value={data.mode || "file"}
                    onCommit={(v: string) => onChange("mode", v)}
                    options={[
                        { value: "file", label: "Local File" },
                        { value: "memory", label: "In-Memory" },
                    ]}
                />
            </div>

            {#if data.mode === "file" || !data.mode}
                <label for="file" class="text-[--theme-fg-secondary]"
                    >File:</label
                >
                <div class="flex space-x-2">
                    <div class="grow">
                        <FormInput
                            inputId="file"
                            value={data.file || ""}
                            placeholder="/path/to/database.sqlite"
                            oninput={(e: any) =>
                                onChange("file", e.target.value)}
                        />
                    </div>
                    <button
                        onclick={browseFile}
                        class="px-2 py-1 h-8 bg-[--theme-bg-tertiary] border border-[--theme-border-default] rounded-md hover:bg-[--theme-bg-hover] text-[--theme-fg-secondary] flex items-center justify-center transition-colors"
                    >
                        <IconFolder size={16} />
                    </button>
                </div>

                <span class="text-[--theme-fg-secondary]">Preview:</span>
                <div
                    class="flex items-center h-8 bg-[--theme-bg-secondary] border border-[--theme-border-default] rounded-md px-3 py-1.5 text-[--theme-fg-tertiary] italic cursor-not-allowed text-xs overflow-hidden"
                >
                    <span class="truncate">jdbc:sqlite:{data.file || ""}</span>
                </div>
            {/if}

            <span class="text-[--theme-fg-secondary]">Options:</span>
            <div class="space-y-2">
                <label class="flex items-center space-x-2">
                    <input
                        id="read-only"
                        type="checkbox"
                        class="rounded"
                        checked={data.options?.read_only || false}
                        onchange={(e: any) =>
                            onChange("options.read_only", e.target.checked)}
                    />
                    <span>Read Only</span>
                </label>
                <label class="flex items-center space-x-2">
                    <input
                        id="enable-foreign-keys"
                        type="checkbox"
                        class="rounded"
                        checked={data.options?.pragmas?.foreign_keys ?? true}
                        onchange={(e: any) =>
                            onChange(
                                "options.pragmas.foreign_keys",
                                e.target.checked,
                            )}
                    />
                    <span>Enable Foreign Keys</span>
                </label>
            </div>

            <label for="journal" class="text-[--theme-fg-secondary]"
                >Journal:</label
            >
            <div class="w-40">
                <Select
                    id="journal"
                    value={data.options?.pragmas?.journal_mode || "WAL"}
                    onCommit={(v: string) =>
                        onChange("options.pragmas.journal_mode", v)}
                    options={[
                        { value: "DELETE", label: "Delete" },
                        { value: "WAL", label: "WAL" },
                        { value: "MEMORY", label: "Memory" },
                        { value: "OFF", label: "Off" },
                    ]}
                />
            </div>
        </div>
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
                            >Sqlite</span
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
