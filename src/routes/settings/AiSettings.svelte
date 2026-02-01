<script lang="ts">
    import { settingsStore } from "$lib/stores/settings.svelte";
    import IconDeviceFloppy from "@tabler/icons-svelte/icons/device-floppy";
    import IconRotate from "@tabler/icons-svelte/icons/rotate";
    import { emit } from "@tauri-apps/api/event";
    import { cn } from "$lib/utils";

    let aiName = $state(settingsStore.aiAgentName);
    let aiUrl = $state(settingsStore.aiAgentUrl);
    let aiApiKey = $state(settingsStore.aiAgentApiKey);
    let aiBasePath = $state(settingsStore.aiAgentBasePath);
    let aiModel = $state(settingsStore.aiAgentModel);

    const models = [
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-4-turbo",
        "claude-3-5-sonnet-latest",
        "claude-3-opus-latest",
        "gemini-1.5-pro",
        "gemini-1.5-flash",
        "deepseek-chat",
        "deepseek-coder",
    ];

    async function saveChanges() {
        settingsStore.aiAgentName = aiName;
        settingsStore.aiAgentUrl = aiUrl;
        settingsStore.aiAgentApiKey = aiApiKey;
        settingsStore.aiAgentBasePath = aiBasePath;
        settingsStore.aiAgentModel = aiModel;

        await emit("settings-changed", ["ai_agent_name", aiName]);
        await emit("settings-changed", ["ai_agent_url", aiUrl]);
        await emit("settings-changed", ["ai_agent_api_key", aiApiKey]);
        await emit("settings-changed", ["ai_agent_base_path", aiBasePath]);
        await emit("settings-changed", ["ai_agent_model", aiModel]);
    }

    function resetChanges() {
        aiName = settingsStore.aiAgentName;
        aiUrl = settingsStore.aiAgentUrl;
        aiApiKey = settingsStore.aiAgentApiKey;
        aiBasePath = settingsStore.aiAgentBasePath;
        aiModel = settingsStore.aiAgentModel;
    }

    const hasChanges = $derived(
        aiName !== settingsStore.aiAgentName ||
            aiUrl !== settingsStore.aiAgentUrl ||
            aiApiKey !== settingsStore.aiAgentApiKey ||
            aiBasePath !== settingsStore.aiAgentBasePath ||
            aiModel !== settingsStore.aiAgentModel,
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
                placeholder="e.g. Tables AI"
                class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent"
            />
        </div>

        <!-- API URL -->
        <div class="space-y-2">
            <label for="ai-url" class="text-sm font-medium">Base URL</label>
            <input
                id="ai-url"
                bind:value={aiUrl}
                placeholder="https://api.openai.com/v1"
                class="w-full bg-accent/5 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent font-mono"
            />
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

        <!-- Model Picker -->
        <div class="space-y-2">
            <label for="ai-model" class="text-sm font-medium">Model</label>
            <select
                id="ai-model"
                bind:value={aiModel}
                class="w-full bg-accent/10 border border-border rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-accent"
            >
                {#each models as model}
                    <option value={model}>{model}</option>
                {/each}
            </select>
        </div>
    </div>

    <!-- Footer Actions -->
    <div class="pt-8 flex items-center justify-end gap-3">
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
