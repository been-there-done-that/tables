<script lang="ts">
    import { IconAlertCircle } from "@tabler/icons-svelte";
    import FormInput from "$lib/components/FormInput.svelte";
    import Button from "$lib/components/Button.svelte";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import {
        testConnectionParams,
        createConnection,
    } from "$lib/commands/client";
    import { ENGINE_SCHEMAS } from "$lib/schema/connectionSchema";
    import { connectionForm } from "$lib/components/datasource/connectionStore.svelte";
    import ConnectionResultPopover from "../ConnectionResultPopover.svelte";
    import DraggableWindow from "$lib/components/DraggableWindow.svelte";
    import IconCopy from "@tabler/icons-svelte/icons/copy";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import IconX from "@tabler/icons-svelte/icons/x";

    interface Props {
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { data, onChange }: Props = $props();
    // Redis form might not have tabs or SSH tab if simplified, but let's keep it safe
    let tab = $state<"general">("general");
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

    function validateForm(): string[] {
        const errors: string[] = [];
        // Basic validation for Redis
        if (!data.db?.host) errors.push("Host is required");
        return errors;
    }

    async function handleCancel() {
        const window = getCurrentWindow();
        await window.close();
    }

    async function handleTestConnection() {
        const errors = validateForm();
        if (errors.length > 0) {
            validationErrors = errors;
            return;
        }

        showPopover = false;
        connectionForm.setTestResult(null);
        isTesting = true;

        try {
            const response = await testConnectionParams(
                "redis",
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

    async function handleApply() {
        const errors = validateForm();
        if (errors.length > 0) {
            validationErrors = errors;
            return;
        }

        isSaving = true;
        try {
            // 1. Construct RuntimeConnection (RedisConfig)
            const runtimeConfig = {
                engine: "redis",
                ...data, // Assuming data maps to RedisConfig structure
                db: {
                    ...data.db,
                    password: null,
                },
            };

            const now = Math.floor(Date.now() / 1000);

            // 2. Construct full Connection struct
            const connectionPayload = {
                id: crypto.randomUUID(),
                // @ts-ignore
                name: data.name || "Untitled Redis",
                engine: "redis",
                host: data.db?.host || null,
                port: data.db?.port || 6379,
                database: String(data.db?.database || 0), // Redis DB is an index, schema expects string typically? Or connection struct 'database' is Option<String>.
                username: data.db?.username || null,
                uses_ssh: false, // Redis form in previous view didn't have SSH tab? Or I missed it?
                // Looking at file content: It only had General inputs. No keys/ssh tab shown.
                // So uses_ssh = false.
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
    <div class="grow overflow-y-auto space-y-6 text-sm">
        <div class="grid grid-cols-[120px_1fr] gap-y-3 items-center mt-4">
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
                            onChange("db.host", e.target.value)}
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
                            value={String(data.db?.port || 6379)}
                            oninput={(e: any) =>
                                onChange("db.port", parseInt(e.target.value))}
                        />
                    </div>
                </div>
            </div>

            <label for="db.database" class="text-[--theme-fg-secondary]"
                >DB Index:</label
            >
            <div class="w-32">
                <FormInput
                    inputId="db.database"
                    type="number"
                    value={String(data.db?.database || 0)}
                    oninput={(e: any) =>
                        onChange("db.database", parseInt(e.target.value))}
                />
            </div>

            <label for="db.username" class="text-[--theme-fg-secondary]"
                >Username:</label
            >
            <FormInput
                inputId="db.username"
                value={data.db?.username || ""}
                placeholder="Optional (Redis 6+ ACL)"
                oninput={(e: any) => onChange("db.username", e.target.value)}
            />

            <label for="db.password" class="text-[--theme-fg-secondary]"
                >Password:</label
            >
            <FormInput
                inputId="db.password"
                type="password"
                value={data.db?.password || ""}
                placeholder="AUTH password"
                oninput={(e: any) => onChange("db.password", e.target.value)}
            />

            <span class="text-[--theme-fg-secondary]">TLS:</span>
            <label class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    class="rounded"
                    checked={data.tls?.enabled || false}
                    onchange={(e: any) =>
                        onChange("tls.enabled", e.target.checked)}
                />
                <span>Enable TLS (Rediss)</span>
            </label>
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
                            driverName="Redis"
                            onClose={() => (showPopover = false)}
                        />
                    {/if}
                </div>
            </div>
        </div>
    </div>
</div>
