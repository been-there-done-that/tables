<script lang="ts">
    import IconPlus from "@tabler/icons-svelte/icons/plus";
    import IconHistory from "@tabler/icons-svelte/icons/history";
    import IconX from "@tabler/icons-svelte/icons/x";
    import { agentStore } from "$lib/agent/agent.svelte";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
    import AgentChat from "$lib/components/agent/AgentChat.svelte";
    import SessionHistoryPopover from "$lib/components/agent/SessionHistoryPopover.svelte";

    let showHistory = $state(false);
    let isEditingTitle = $state(false);
    let titleInput = $state("");

    function handleStartEdit() {
        if (!agentStore.currentSession) return;
        titleInput = agentStore.currentSession.title;
        isEditingTitle = true;
    }

    async function handleSaveTitle() {
        if (!agentStore.currentSession) return;
        if (
            titleInput.trim() &&
            titleInput !== agentStore.currentSession.title
        ) {
            await agentStore.renameSession(
                agentStore.currentSession.id,
                titleInput,
            );
        }
        isEditingTitle = false;
    }

    async function handleNewSession() {
        await agentStore.createSession();
    }
</script>

<div class="flex h-full w-full flex-col bg-background relative overflow-hidden">
    <!-- Header -->
    <div
        class="h-9 flex items-center justify-between px-3 bg-muted/30 relative z-20"
    >
        <div class="flex items-center gap-2 flex-1 min-w-0 mr-2">
            {#if isEditingTitle}
                <input
                    class="h-6 w-full text-xs font-medium bg-background border border-border rounded px-1.5 focus:outline-none focus:ring-1 focus:ring-ring"
                    bind:value={titleInput}
                    onblur={handleSaveTitle}
                    onkeydown={(e) => e.key === "Enter" && handleSaveTitle()}
                    autofocus
                />
            {:else}
                <button
                    class="text-xs font-semibold text-foreground truncate hover:text-accent-foreground/80 transition-colors text-left"
                    onclick={handleStartEdit}
                    title="Click to rename"
                >
                    {agentStore.currentSession?.title || "AI Assistant"}
                </button>
            {/if}
        </div>

        <div class="flex items-center gap-0.5">
            <button
                class="h-7 w-7 flex items-center justify-center hover:bg-accent rounded-sm text-muted-foreground transition-colors"
                onclick={handleNewSession}
                title="New Session"
            >
                <IconPlus class="size-4" />
            </button>

            <div class="relative">
                <button
                    class="h-7 w-7 flex items-center justify-center hover:bg-accent rounded-sm text-muted-foreground transition-colors"
                    class:bg-accent={showHistory}
                    onclick={() => (showHistory = !showHistory)}
                    title="History"
                >
                    <IconHistory class="size-4" />
                </button>
                {#if showHistory}
                    <SessionHistoryPopover
                        onClose={() => (showHistory = false)}
                    />
                {/if}
            </div>

            <button
                class="h-7 w-7 flex items-center justify-center hover:bg-accent rounded-sm text-muted-foreground transition-colors"
                onclick={() => windowState.closeRightPanel()}
                title="Close"
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
