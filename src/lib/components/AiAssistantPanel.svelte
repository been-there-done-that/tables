<script lang="ts">
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconBrandTesla from "@tabler/icons-svelte/icons/brand-tesla";
    import { windowState } from "$lib/stores/window.svelte";
    import { settingsStore } from "$lib/stores/settings.svelte";
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
        <button
            type="button"
            class="h-6 w-6 flex items-center justify-center hover:bg-accent rounded text-muted-foreground transition-colors"
            onclick={() => windowState.closeRightPanel()}
        >
            <IconX class="size-4" />
        </button>
    </div>

    <!-- Content -->
    <div
        class="flex-1 overflow-auto flex flex-col items-center justify-center p-6 text-center space-y-6 relative z-10"
    >
        <div class="space-y-2 max-w-xs">
            <h3 class="text-sm font-medium tracking-tight">
                {settingsStore.aiAgentName} is sleeping...
            </h3>
            <p class="text-xs text-muted-foreground">
                We are currently refining the intelligence. Please check back
                later for query assistance.
            </p>
        </div>

        <!-- Loading Indicator Style -->
        <div class="flex gap-1.5 opacity-40">
            <div
                class="h-1.5 w-1.5 animate-bounce rounded-full bg-red-500"
            ></div>
            <div
                class="h-1.5 w-1.5 animate-bounce rounded-full bg-red-500 delay-150"
            ></div>
            <div
                class="h-1.5 w-1.5 animate-bounce rounded-full bg-red-500 delay-300"
            ></div>
        </div>
    </div>
</div>
