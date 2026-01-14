<script lang="ts">
    import IconX from "@tabler/icons-svelte/icons/x";
    import IconBrandTesla from "@tabler/icons-svelte/icons/brand-tesla";
    import { windowState } from "$lib/stores/window.svelte";
    import bgImage from "$lib/svg/image.png";
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

    <!-- Background Pattern -->
    <div
        class="absolute inset-0 z-0 opacity-20 pointer-events-none text-foreground"
        style="background-image: url('{bgImage}'); background-repeat: repeat; background-size: 200px; background-position: 0 0;"
    ></div>

    <!-- Content -->
    <div
        class="flex-1 overflow-auto flex flex-col opacity-70 items-center justify-center p-6 text-center space-y-6 relative z-10"
    >
        <div class="space-y-2 max-w-xs">
            <h3 class="text-sm font-medium tracking-tight">
                Cooking Intelligence...
            </h3>
            <p class="text-xs text-muted-foreground">
                We are currently brewing the AI in the lab. Please check back
                later for advanced query assistance.
            </p>
        </div>

        <!-- Loading Indicator Style -->
        <div class="flex gap-1.5">
            <div class="h-2 w-2 animate-bounce rounded-full bg-red-500"></div>
            <div
                class="h-2 w-2 animate-bounce rounded-full bg-red-500 delay-150"
            ></div>
            <div
                class="h-2 w-2 animate-bounce rounded-full bg-red-500 delay-300"
            ></div>
        </div>
    </div>
</div>
