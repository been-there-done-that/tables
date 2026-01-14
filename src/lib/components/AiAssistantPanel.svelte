<script lang="ts">
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconBrandTesla from "@tabler/icons-svelte/icons/brand-tesla";
    import { windowState } from "$lib/stores/window.svelte";
    import aiLabSvg from "$lib/assets/ai_lab.svg?raw";
</script>

<div class="flex h-full w-full flex-col bg-background">
    <!-- Header -->
    <div
        class="h-8 flex items-center justify-between px-3 py-1 border-b border-border bg-muted/30"
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
        class="flex-1 overflow-auto flex flex-col items-center justify-center p-6 text-center space-y-6"
    >
        <div class="relative w-64 h-64 opacity-90">
            {@html aiLabSvg}
        </div>

        <div class="space-y-2 max-w-xs">
            <h3 class="text-lg font-medium tracking-tight">
                Cooking Intelligence...
            </h3>
            <p class="text-sm text-muted-foreground">
                We are currently brewing the AI in the lab. Please check back
                later for advanced query assistance.
            </p>
        </div>

        <!-- Loading Indicator Style -->
        <div class="flex gap-1.5">
            <div
                class="w-2 h-2 rounded-full bg-red-500 animate-[bounce_1s_infinite]"
            ></div>
            <div
                class="w-2 h-2 rounded-full bg-red-500 animate-[bounce_1s_infinite_0.2s]"
            ></div>
            <div
                class="w-2 h-2 rounded-full bg-red-500 animate-[bounce_1s_infinite_0.4s]"
            ></div>
        </div>
    </div>
</div>
