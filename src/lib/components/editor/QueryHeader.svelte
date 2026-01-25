<script lang="ts">
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import IconSquare from "@tabler/icons-svelte/icons/square";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import { cn } from "$lib/utils";

    let {
        status = "idle",
        duration = 0,
        onRun,
        onStop,
    } = $props<{
        status: "idle" | "running" | "success" | "error";
        duration?: number;
        onRun: () => void;
        onStop: () => void;
    }>();

    function formatDuration(ms: number): string {
        if (ms < 1000) return `${Math.round(ms)}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    }
</script>

<div class="flex items-center gap-3 py-0.5 px-2 select-none">
    <!-- Action Button -->
    <button
        class={cn(
            "flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-semibold transition-all shadow-sm active:scale-95",
            status === "running"
                ? "bg-red-500/10 text-red-500 hover:bg-red-500/20"
                : "bg-emerald-500/10 text-emerald-500 hover:bg-emerald-500/20",
        )}
        onclick={status === "running" ? onStop : onRun}
    >
        {#if status === "running"}
            <IconSquare class="size-3 fill-current" />
            <span>Stop</span>
        {:else}
            <IconPlayerPlay class="size-3 fill-current" />
            <span>Run</span>
        {/if}
    </button>

    <!-- Status Info -->
    {#if status === "running"}
        <div
            class="flex items-center gap-1.5 text-xs text-muted-foreground animate-pulse"
        >
            <IconLoader2 class="size-3 animate-spin" />
            <span>Running...</span>
        </div>
    {:else if status === "success" || status === "error"}
        <div
            class={cn(
                "flex items-center gap-1.5 text-xs",
                status === "error" ? "text-red-400" : "text-emerald-400",
            )}
        >
            {#if status === "error"}
                <span>Failed</span>
            {:else}
                <span>Took {formatDuration(duration || 0)}</span>
            {/if}
        </div>
    {/if}
</div>

<style>
    /* Ensure the component takes up the space Monaco allocates */
    :global(.monaco-editor .query-header-widget) {
        z-index: 10;
        width: 100%;
    }
</style>
