<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { cn } from "$lib/utils";
    import type { ConnectionInfo } from "$lib/commands/types";
    import { IconCheck, IconX, IconEdit } from "@tabler/icons-svelte";

    interface Props {
        result: ConnectionInfo;
        onClose: () => void;
        className?: string;
    }

    let { result, onClose, className = "" }: Props = $props();

    const isSuccess = $derived(result.connected);
</script>

<div
    class={cn(
        "absolute bottom-full mb-4 left-1/2 -translate-x-1/2 w-80 z-50",
        className,
    )}
    transition:fly={{ y: 8, duration: 300, opacity: 0 }}
>
    <!-- Tooltip Container -->
    <div
        class="bg-[--theme-bg-secondary] border border-[--theme-border-default] rounded-xl shadow-[0_8px_30px_rgb(0,0,0,0.3)] p-5 relative ring-1 ring-white/5"
    >
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
            <span
                class={cn(
                    "font-bold uppercase tracking-widest text-[10px] px-2 py-0.5 rounded-full",
                    isSuccess
                        ? "text-green-400 bg-green-500/10 border border-green-500/20"
                        : "text-red-400 bg-red-500/10 border border-red-500/20",
                )}
            >
                {isSuccess ? "Succeeded" : "Failed"}
            </span>

            {#if result.response_time_ms !== undefined}
                <div
                    class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-[--theme-bg-tertiary] border border-[--theme-border-subtle]"
                >
                    <span
                        class={cn(
                            "text-[10px] font-bold",
                            result.response_time_ms < 50
                                ? "text-green-500"
                                : result.response_time_ms < 200
                                  ? "text-yellow-500"
                                  : "text-red-500",
                        )}>{result.response_time_ms} ms</span
                    >
                </div>
            {/if}
        </div>

        <!-- Content -->
        <div class="space-y-3 font-mono text-xs text-[--theme-fg-secondary]">
            <div class="flex items-center justify-between">
                <span class="text-[--theme-fg-tertiary]">Driver</span>
                <div class="flex items-center gap-1.5">
                    <span class="text-[--theme-fg-primary] font-medium"
                        >SQLite Native</span
                    >
                    {#if result.version}
                        <span class="text-[10px] opacity-60"
                            >v{result.version}</span
                        >
                    {/if}
                </div>
            </div>

            {#if result.error}
                <div
                    class="mt-4 p-3 rounded-lg bg-red-500/5 border border-red-500/20 text-red-400 text-[11px] leading-relaxed"
                >
                    <div class="flex gap-2">
                        <IconX size={14} class="shrink-0 mt-0.5" />
                        <span>{result.error}</span>
                    </div>
                </div>
            {/if}
        </div>

        <!-- Arrow -->
        <div
            class="absolute -bottom-1.5 left-1/2 -translate-x-1/2 w-3 h-3 bg-[--theme-bg-secondary] border-r border-b border-[--theme-border-default] rotate-45"
        ></div>
    </div>
</div>

<style>
    /* Ensure the popover doesn't overflow the window if possible */
    :global(.popover-container) {
        pointer-events: none;
    }
    :global(.popover-container > *) {
        pointer-events: auto;
    }
</style>
