<script lang="ts">
    import FormInput from "$lib/components/FormInput.svelte";
    import Select from "$lib/components/Select.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import {
        testConnectionParams,
        createConnection,
        updateConnection,
    } from "$lib/commands/client";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import ConnectionResultPopover from "../ConnectionResultPopover.svelte";
    import IconFolder from "@tabler/icons-svelte/icons/folder";
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";

    import { open } from "@tauri-apps/plugin-dialog";
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";

    import { ENGINE_SCHEMAS } from "$lib/schema/connectionSchema";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
        hideFooter?: boolean;
    }

    let { data, onChange, hideFooter = false }: Props = $props();
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

        // Manually validate Name since it's outside the engine schema
        if (!data.name || data.name.trim() === "") {
            validationErrors["name"] = "Connection Name is required";
        }

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

    async function handleApply() {
        const errors = validateForm();
        if (Object.keys(errors).length > 0) {
            validationErrors = Object.values(errors);
            return;
        }

        isSaving = true;
        try {
            // 1. Construct RuntimeConnection (SqliteConfig)
            const runtimeConfig = {
                engine: "sqlite",
                ...data,
            };

            const now = Math.floor(Date.now() / 1000);

            // Checks if we are updating an existing connection
            // @ts-ignore
            const existingId = data.id;

            // 2. Prepare common payload data
            const connectionData = {
                // @ts-ignore
                name: data.name || "Untitled SQLite",
                engine: "sqlite",
                host: null,
                port: null,
                database: data.file,
                username: null,
                uses_ssh: false,
                uses_tls: false,
                config_json: JSON.stringify(runtimeConfig),
                is_favorite: false,
                color_tag: null,
                updated_at: now,
                last_connected_at: null,
                connection_count: 0,
            };

            const credentialsPayload = {
                // SQLite has no creds usually, maybe password if encrypted, but here simplified.
                password: null,
                ssh_private_key: null,
                ssh_passphrase: null,
                ssl_certificate: null,
                ssl_private_key: null,
                ssl_ca_certificate: null,
                api_token: null,
                aws_access_key_id: null,
                aws_secret_access_key: null,
                aws_session_token: null,
            };

            let response;
            if (existingId) {
                // UPDATE
                const payload = {
                    id: existingId,
                    connection: connectionData,
                    credentials: credentialsPayload,
                };
                // @ts-ignore
                response = await updateConnection(payload);
            } else {
                // CREATE
                // @ts-ignore
                const payload = {
                    connection: {
                        ...connectionData,
                        id: crypto.randomUUID(), // If backend allows ID, otherwise remove
                        created_at: now,
                    },
                    credentials: credentialsPayload,
                };
                // @ts-ignore
                response = await createConnection(payload);
            }

            if (response.success) {
                console.log("Saved/Updated:", response.data);
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
            } else if (!response.success && response.error) {
                // Also show validation window for backend connection errors during test?
                // Or stick to the popover if it handles errors?
                // The popover logic currently requires a result object.
                // If API returns success: false, it usually populates error string.
                // I'll leave the popover flow as is since user didn't ask to change that part for Save.
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
                                driverName="SQLite3"
                                onClose={() => (showPopover = false)}
                            />
                        {/if}
                    </div>
                </div>
            </div>
        </div>
    {/if}
</div>
