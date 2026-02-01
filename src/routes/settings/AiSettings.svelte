<script lang="ts">
    import { settingsStore } from "$lib/stores/settings.svelte";
    import IconDeviceFloppy from "@tabler/icons-svelte/icons/device-floppy";
    import IconRotate from "@tabler/icons-svelte/icons/rotate";
    import IconReload from "@tabler/icons-svelte/icons/reload";
    import IconCheck from "@tabler/icons-svelte/icons/check";
    import { emit } from "@tauri-apps/api/event";
    import { cn } from "$lib/utils";
    import { toast } from "svelte-sonner";

    import { fetchModels as fetchModelsCommand } from "$lib/commands/client";

    let aiName = $state(settingsStore.aiAgentName);
    let aiUrl = $state(settingsStore.aiAgentUrl);
    let aiApiKey = $state(settingsStore.aiAgentApiKey);
    let aiBasePath = $state(settingsStore.aiAgentBasePath);
    let aiModel = $state(settingsStore.aiAgentModel);
    let availableModels = $state([...settingsStore.aiAgentAvailableModels]);
    let isFetching = $state(false);

    async function fetchModels() {
        if (!aiUrl) {
            toast.error("Base URL is required to fetch models");
            return;
        }

        isFetching = true;
        try {
            const result = await fetchModelsCommand(
                aiUrl,
                aiApiKey || undefined,
            );

            if (result.success && result.data) {
                availableModels = result.data;
                toast.success(`Fetched ${result.data.length} models`);
            } else {
                throw new Error(result.error || "Failed to fetch models");
            }
        } catch (error: any) {
            console.error("Fetch models error:", error);
            toast.error(error.message || "Failed to fetch models");
        } finally {
            isFetching = false;
        }
    }

    async function saveChanges() {
        settingsStore.aiAgentName = aiName;
        settingsStore.aiAgentUrl = aiUrl;
        settingsStore.aiAgentApiKey = aiApiKey;
        settingsStore.aiAgentBasePath = aiBasePath;
        settingsStore.aiAgentModel = aiModel;
        settingsStore.aiAgentAvailableModels = availableModels;

        await emit("settings-changed", ["ai_agent_name", aiName]);
        await emit("settings-changed", ["ai_agent_url", aiUrl]);
        await emit("settings-changed", ["ai_agent_api_key", aiApiKey]);
        await emit("settings-changed", ["ai_agent_base_path", aiBasePath]);
        await emit("settings-changed", ["ai_agent_model", aiModel]);
        await emit("settings-changed", [
            "ai_agent_available_models",
            availableModels.join(","),
        ]);

        toast.success("AI Settings saved");
    }

    function resetChanges() {
        aiName = settingsStore.aiAgentName;
        aiUrl = settingsStore.aiAgentUrl;
        aiApiKey = settingsStore.aiAgentApiKey;
        aiBasePath = settingsStore.aiAgentBasePath;
        aiModel = settingsStore.aiAgentModel;
        availableModels = [...settingsStore.aiAgentAvailableModels];
    }

    const hasChanges = $derived(
        aiName !== settingsStore.aiAgentName ||
            aiUrl !== settingsStore.aiAgentUrl ||
            aiApiKey !== settingsStore.aiAgentApiKey ||
            aiBasePath !== settingsStore.aiAgentBasePath ||
            aiModel !== settingsStore.aiAgentModel ||
            JSON.stringify(availableModels) !==
                JSON.stringify(settingsStore.aiAgentAvailableModels),
    );
</script>

<div
    class="flex flex-col h-full w-full bg-background overflow-hidden p-6 max-w-2xl mx-auto space-y-8"
>
    <div class="space-y-1">
        <h2 class="text-xl font-semibold tracking-tight">
            AI Assistant Settings
        </h2>
        <p class="text-sm text-muted-foreground">
            Configure your AI assistant connection and behavior.
        </p>
    </div>

    <div class="space-y-6">
        <!-- Assistant Name -->
        <div class="space-y-2">
            <label for="ai-name" class="text-sm font-medium"
                >Assistant Name</label
            >
            <input
                id="ai-name"
                bind:value={aiName}
                placeholder="Assistant"
                class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent"
            />
        </div>

        <div class="grid grid-cols-2 gap-4">
            <!-- API URL -->
            <div class="space-y-2">
                <label for="ai-url" class="text-sm font-medium">Base URL</label>
                <input
                    id="ai-url"
                    bind:value={aiUrl}
                    placeholder="http://127.0.0.1:1234/v1"
                    class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent font-mono"
                />
            </div>

            <!-- Base Path -->
            <div class="space-y-2">
                <label for="ai-base" class="text-sm font-medium"
                    >Extra Path / Context ID</label
                >
                <input
                    id="ai-base"
                    bind:value={aiBasePath}
                    placeholder="/v1/chat/completions"
                    class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent font-mono"
                />
            </div>
        </div>

        <!-- API Key -->
        <div class="space-y-2">
            <label for="ai-key" class="text-sm font-medium"
                >API Key / Secret</label
            >
            <input
                id="ai-key"
                type="password"
                bind:value={aiApiKey}
                placeholder="sk-..."
                class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent font-mono"
            />
        </div>

        <!-- Model Selection (Chips) -->
        <div class="space-y-3">
            <div class="flex items-center justify-between">
                <label class="text-sm font-medium">Available Models</label>
                <button
                    onclick={fetchModels}
                    disabled={isFetching || !aiUrl}
                    class="flex items-center gap-1.5 px-2 py-1 text-[10px] font-medium text-accent hover:bg-accent/10 rounded-md transition-colors disabled:opacity-40 uppercase tracking-wider"
                    title="Fetch models from the API"
                >
                    <IconReload
                        class={cn("size-3", isFetching && "animate-spin")}
                    />
                    {isFetching ? "FETCHING..." : "FETCH MODELS"}
                </button>
            </div>

            <div
                class="flex flex-wrap gap-2 min-h-[80px] p-4 border-2 border-dashed border-border/60 rounded-md bg-transparent overflow-y-auto max-h-[300px]"
            >
                {#each availableModels as model}
                    <button
                        onclick={() => (aiModel = model)}
                        class={cn(
                            "group flex items-center gap-1.5 px-2.5 py-1 rounded text-[10px] font-medium transition-all border font-mono",
                            aiModel === model
                                ? "bg-primary text-primary-foreground border-primary"
                                : "bg-transparent text-muted-foreground border-border hover:border-primary/50 hover:text-foreground",
                        )}
                    >
                        {#if aiModel === model}
                            <IconCheck class="size-3" />
                        {/if}
                        {model}
                    </button>
                {:else}
                    <div
                        class="w-full h-full flex flex-col items-center justify-center text-center opacity-40 py-4 gap-2"
                    >
                        <IconReload class="size-4" />
                        <div class="space-y-0.5">
                            <p
                                class="text-[10px] uppercase tracking-wider font-medium"
                            >
                                No models loaded
                            </p>
                            <p class="text-[9px]">Click fetch to load</p>
                        </div>
                    </div>
                {/each}
            </div>
            {#if availableModels.length > 0}
                <div class="flex justify-end">
                    <p class="text-[10px] text-muted-foreground italic">
                        Active: <span class="font-mono text-foreground"
                            >{aiModel || "none"}</span
                        >
                    </p>
                </div>
            {/if}
        </div>
    </div>

    <!-- Footer Actions -->
    <div
        class="pt-4 flex items-center justify-end gap-3 border-t border-border"
    >
        <button
            class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-muted-foreground hover:text-foreground hover:bg-muted rounded-md transition-colors"
            onclick={resetChanges}
            disabled={!hasChanges}
            class:opacity-50={!hasChanges}
        >
            <IconRotate class="size-4" />
            Reset
        </button>

        <button
            class="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-(--theme-accent-primary) text-white hover:opacity-90 rounded-md transition-colors shadow-sm"
            onclick={saveChanges}
            disabled={!hasChanges}
            class:opacity-50={!hasChanges}
        >
            <IconDeviceFloppy class="size-4" />
            Save AI Settings
        </button>
    </div>
</div>
