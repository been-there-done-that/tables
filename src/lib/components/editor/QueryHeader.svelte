<script lang="ts">
    import IconPlayerPlay from "@tabler/icons-svelte/icons/player-play";
    import IconSquare from "@tabler/icons-svelte/icons/square";
    import IconLoader2 from "@tabler/icons-svelte/icons/loader-2";
    import { cn } from "$lib/utils";

    let {
        status = "idle",
        duration = 0,
        errorMessage = "",
        onRun,
        onStop,
    } = $props<{
        status: "idle" | "running" | "success" | "error";
        duration?: number;
        errorMessage?: string;
        onRun: () => void;
        onStop: () => void;
    }>();

    function formatDuration(ms: number): string {
        if (ms < 1000) return `${Math.round(ms)}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    }
</script>

<div
    class="flex flex-row items-center gap-2 py-1 select-none whitespace-nowrap w-max"
>
    <!-- Action Button -->
    <button
        class={cn(
            "flex items-center gap-1.5 px-2 py-0.5 rounded border transition-all shadow-sm active:scale-95 group/btn",
            status === "running"
                ? "bg-red-500/10 text-red-500 border-red-500/20 hover:bg-red-500/20"
                : "bg-primary/10 text-primary border-primary/20 hover:bg-primary/20",
        )}
        onclick={() => {
            if (status === "running") onStop();
            else onRun();
        }}
    >
        {#if status === "running"}
            <IconSquare class="size-3 fill-current" />
            <span class="text-xs font-semibold tracking-tight">Stop</span>
        {:else}
            <IconPlayerPlay class="size-3 fill-current" />
            <span class="text-xs font-semibold tracking-tight">Run</span>
        {/if}
    </button>

    <!-- Status Info -->
    {#if status === "running"}
        <div
            class="flex items-center gap-1.5 text-xs text-muted-foreground/70 animate-pulse"
        >
            <IconLoader2 class="size-3 animate-spin" />
            <span>Running...</span>
        </div>
    {:else if status === "success" || status === "error"}
        <div
            class={cn(
                "flex items-center gap-1.5 text-xs overflow-hidden",
                status === "error"
                    ? "text-red-400"
                    : "text-muted-foreground/60",
            )}
            title={status === "error"
                ? errorMessage
                : `Execution time: ${duration}ms`}
        >
            {#if status === "error"}
                <span class="font-bold uppercase text-[10px] tracking-tight"
                    >Failed</span
                >
                <span
                    class="truncate max-w-[200px] font-mono text-[11px] opacity-80"
                    >{errorMessage || "Unknown error"}</span
                >
            {:else}
                <span class="font-medium">Finished in</span>
                <span class="font-mono text-[11px]"
                    >{formatDuration(duration || 0)}</span
                >
            {/if}
        </div>
    {/if}
</div>

<style>
    /* Ensure the component takes up the space Monaco allocates and is clickable */
    :global(.monaco-editor .query-header-widget) {
        z-index: 50 !important;
        pointer-events: auto !important;
        width: 100%;
    }
</style>
