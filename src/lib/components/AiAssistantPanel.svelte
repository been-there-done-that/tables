<script lang="ts">
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconBrandTesla from "@tabler/icons-svelte/icons/brand-tesla";
    import IconSettings from "@tabler/icons-svelte/icons/settings";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import AgentChat from "$lib/components/agent/AgentChat.svelte";
</script>

<div class="flex h-full w-full flex-col bg-background relative overflow-hidden">
    <!-- Header -->
    <div
        class="h-8 flex items-center justify-between px-3 py-1 border-b border-border bg-muted/30 relative z-20"
    >
        <div class="flex items-center gap-2">
            <IconBrandTesla class="size-4 text-red-500" />
            <h2 class="text-xs font-semibold text-muted-foreground">
                AI Assistant
            </h2>
        </div>
        <div class="flex items-center gap-1">
            <a
                href="/settings/ai"
                class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
                title="AI Settings"
            >
                <IconSettings class="size-3.5" />
            </a>
            <button
                type="button"
                class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
                onclick={() => windowState.closeRightPanel()}
            >
                <IconX class="size-4" />
            </button>
        </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-hidden relative z-10 flex flex-col">
        {#if settingsStore.aiAgentUrl}
            <AgentChat />
        {:else}
            <div
                class="flex-1 flex flex-col items-center justify-center p-6 text-center space-y-6"
            >
                <div class="space-y-2 max-w-xs">
                    <h3 class="text-sm font-medium tracking-tight">
                        Setup Required
                    </h3>
                    <p class="text-xs text-muted-foreground">
                        Please configure your AI provider settings to start
                        using the assistant.
                    </p>
                </div>
                <a
                    href="/settings/ai"
                    class="px-4 py-2 bg-primary text-primary-foreground text-xs font-medium rounded-md hover:bg-primary/90 transition-colors"
                >
                    Configure Settings
                </a>
            </div>
        {/if}
    </div>
</div>
