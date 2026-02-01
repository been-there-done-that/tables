<script lang="ts">
    import { Loader2, Check, X, Wrench } from "lucide-svelte";
    import * as Tooltip from "$lib/components/ui/tooltip";

    let { toolName, status, result, error } = $props<{
        toolName: string;
        status: "pending" | "success" | "error";
        result?: string;
        error?: string;
    }>();

    // Format the tool name for display
    function formatToolName(name: string): string {
        return name.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
    }
</script>

<Tooltip.Root>
    <Tooltip.Trigger>
        <div
            class="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-muted/50 border text-xs transition-colors {status ===
            'pending'
                ? 'border-primary/30'
                : status === 'success'
                  ? 'border-green-500/30'
                  : 'border-destructive/30'}"
        >
            <Wrench class="size-3 opacity-50" />
            <span class="font-medium">{formatToolName(toolName)}</span>
            {#if status === "pending"}
                <Loader2 class="size-3 animate-spin text-primary" />
            {:else if status === "success"}
                <Check class="size-3 text-green-500" />
            {:else}
                <X class="size-3 text-destructive" />
            {/if}
        </div>
    </Tooltip.Trigger>
    <Tooltip.Content class="max-w-md" side="top">
        {#if status === "pending"}
            <p class="text-xs text-muted-foreground">Executing tool...</p>
        {:else if status === "success" && result}
            <div class="space-y-1">
                <p class="text-xs font-semibold text-green-600">Success</p>
                <pre
                    class="text-xs max-h-48 overflow-auto bg-muted/50 p-2 rounded whitespace-pre-wrap">{(
                        result ?? ""
                    ).slice(0, 500)}{(result ?? "").length > 500
                        ? "..."
                        : ""}</pre>
            </div>
        {:else if status === "error" && error}
            <div class="space-y-1">
                <p class="text-xs font-semibold text-destructive">Error</p>
                <p class="text-xs text-destructive/80">{error}</p>
            </div>
        {:else}
            <p class="text-xs text-muted-foreground">No details available</p>
        {/if}
    </Tooltip.Content>
</Tooltip.Root>
