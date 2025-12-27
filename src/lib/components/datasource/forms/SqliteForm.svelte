<script lang="ts">
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { testConnectionParams } from "$lib/commands/client";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import ConnectionResultPopover from "../ConnectionResultPopover.svelte";
    import IconFolder from "@tabler/icons-svelte/icons/folder";
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";

    import { open } from "@tauri-apps/plugin-dialog";
    import IconX from "@tabler/icons-svelte/icons/x";

    import { ENGINE_SCHEMAS } from "$lib/schema/connectionSchema";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
    let showPopover = $state(false);
    let isTesting = $state(false);
    let validationErrors = $state<string[]>([]);
    const testResult = $derived(connectionForm.state.testResult);

    // Helper to get nested field value
    function getFieldValue(path: string) {
        const keys = path.split(".");
        let current: any = data;
        for (const key of keys) {
            current = current?.[key];
        }
        return current;
    }

    // Dynamic validation using schema
    function validateForm(): Record<string, string> {
        const validationErrors: Record<string, string> = {};
        const schema = ENGINE_SCHEMAS.sqlite.fields;

        for (const [fieldPath, fieldDef] of Object.entries(schema)) {
            const def = fieldDef as any;

            // Skip if field condition is not met
            if (def.condition && !def.condition(data)) {
                continue;
            }

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
        }
        return validationErrors;
    }

    async function browseFile() {
        try {
            const selected = await open({
                multiple: false,
                filters: [
                    {
                        name: "SQLite Database",
                        extensions: ["db", "sqlite", "sqlite3"],
                    },
                ],
            });

            if (selected) {
                onChange("file", selected as string);
            }
        } catch (err) {
            console.error("Failed to open file dialog:", err);
        }
    }

    // Self-contained action handlers
    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    function handleApply() {
        const errors = validateForm();
        if (Object.keys(errors).length > 0) {
            validationErrors = Object.values(errors);
            return;
        }
        console.log("Apply SQLite connection", data);
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

        console.debug(
            "Testing SQLite connection with payload",
            $state.snapshot(data),
        );

        try {
            const response = await testConnectionParams(
                "sqlite",
                $state.snapshot(data),
            );

            if (response.success && response.data) {
                connectionForm.setTestResult(response.data);
                showPopover = true;
            }
        } finally {
            isTesting = false;
        }
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
            headerClass="bg-red-500/10 border-b border-red-500/10"
            titleClass="text-red-400 font-bold uppercase tracking-wider text-[11px]"
            class="w-[420px] shadow-2xl border-red-500/20"
            onClose={() => (validationErrors = [])}
        >
            <div class="px-5 py-4 flex flex-col gap-4">
                <div class="p-3">
                    <ul class="flex flex-col gap-2">
                        {#each validationErrors as error}
                            <li
                                class="flex items-start gap-2.5 text-xs text-[--theme-fg-secondary]"
                            >
                                <div
                                    class="mt-1.5 size-1 rounded-full bg-red-500 shrink-0 shadow-[0_0_8px_rgba(239,68,68,0.5)]"
                                ></div>
                                <span class="leading-relaxed">{error}</span>
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
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="flex flex-col items-center mx-36">
            <div class="flex">
                <div
                    role="tab"
                    tabindex="0"
                    class="px-6 py-2.5 text-xs font-medium cursor-pointer transition-all duration-150 rounded-t-lg -mb-px bg-[--theme-bg-primary] text-[--theme-fg-primary] border border-[--theme-border-default] border-b-[--theme-bg-primary]"
                >
                    General
                </div>
            </div>
            <!-- Custom arched/faded line -->
            <div
                class="h-px w-full bg-linear-to-r from-transparent via-(--theme-fg-secondary) to-transparent opacity-30"
            ></div>
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
                        class="p-1 bg-(--theme-bg-tertiary) border border-[--theme-border-default] rounded-md hover:bg-(--theme-bg-hover) text-(--theme-fg-secondary) flex items-center justify-center transition-colors"
                    >
                        <IconFolder class="size-5" />
                    </button>
                </div>
            {/if}

            <span class="text-[--theme-fg-secondary]">Options:</span>
            <div class="space-y-2">
                <label class="flex items-center space-x-2 cursor-pointer">
                    <input
                        id="read-only"
                        type="checkbox"
                        class="rounded border-[--theme-border-default] bg-[--theme-bg-tertiary] w-5 h-5 text-[--theme-accent-primary] focus:ring-[--theme-accent-primary] focus:ring-offset-0 cursor-pointer"
                        checked={data.options?.read_only || false}
                        onchange={(e: any) =>
                            onChange("options.read_only", e.target.checked)}
                    />
                    <span class="text-[--theme-fg-primary]">Read Only</span>
                </label>
                <label class="flex items-center space-x-2 cursor-pointer">
                    <input
                        id="enable-foreign-keys"
                        type="checkbox"
                        class="rounded border-[--theme-border-default] bg-[--theme-bg-tertiary] w-5 h-5 text-[--theme-accent-primary] focus:ring-[--theme-accent-primary] focus:ring-offset-0 cursor-pointer"
                        checked={data.options?.pragmas?.foreign_keys ?? true}
                        onchange={(e: any) =>
                            onChange(
                                "options.pragmas.foreign_keys",
                                e.target.checked,
                            )}
                    />
                    <span class="text-[--theme-fg-primary]"
                        >Enable Foreign Keys</span
                    >
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
                            driverName="SQLite3"
                            onClose={() => (showPopover = false)}
                        />
                    {/if}
                </div>
            </div>
        </div>
    </div>
</div>
