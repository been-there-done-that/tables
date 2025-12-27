<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { cn } from "$lib/utils";
    import type { ConnectionInfo } from "$lib/commands/types";
    import { IconCheck, IconX, IconEdit } from "@tabler/icons-svelte";
    import { onMount } from "svelte";

    interface Props {
        result: ConnectionInfo;
        onClose: () => void;
        className?: string;
        driverName?: string;
    }

    let {
        result,
        onClose,
        className = "",
        driverName = "Unknown",
    }: Props = $props();

    const isSuccess = $derived(result.connected);
    let popoverRef: HTMLDivElement;

    onMount(() => {
        const handleOutsideClick = (event: MouseEvent) => {
            if (popoverRef && !popoverRef.contains(event.target as Node)) {
                onClose();
            }
        };

        window.addEventListener("mousedown", handleOutsideClick);
        return () => {
            window.removeEventListener("mousedown", handleOutsideClick);
        };
    });
</script>

<div
    bind:this={popoverRef}
    class={cn(
        "absolute bottom-full mb-3 left-1/2 -translate-x-1/2 w-80 z-50 origin-bottom filter drop-shadow-xl",
        className,
    )}
    transition:fly={{ y: 4, duration: 200, opacity: 0 }}
>
    <!-- Card Container -->
    <div
        class={cn(
            "border rounded-md relative z-10",
            isSuccess
                ? "bg-[#ecfdf5] border-green-200"
                : "bg-[#fef2f2] border-red-200",
        )}
    >
        <div class="p-4">
            <!-- Row 1: Icon + Latency -->
            <div class="flex items-center justify-between h-full">
                <!-- Left: Checkmark -->
                <div
                    class={cn(
                        "flex items-center justify-center w-8 h-8 rounded-full ring-4 shadow-sm",
                        isSuccess
                            ? "bg-green-500 text-white ring-green-500/10"
                            : "bg-red-500 text-white ring-red-500/10",
                    )}
                >
                    {#if isSuccess}
                        <IconCheck size={18} stroke={3} />
                    {:else}
                        <IconX size={18} stroke={3} />
                    {/if}
                </div>

                <!-- Right: Latency -->
                {#if isSuccess}
                    <div class="flex items-center justify-center h-full">
                        <span
                            class="text-[10px] uppercase tracking-wider text-green-800/60 font-bold mr-2"
                        >
                            Latency:
                        </span>
                        <span
                            class={cn(
                                "text-2xl font-mono font-bold leading-none",
                                (result.response_time_ms ?? 0) < 100
                                    ? "text-green-700"
                                    : "text-red-600",
                            )}>{result.response_time_ms}</span
                        >
                        <span class="text-xs text-green-800/60 font-medium"
                            >ms</span
                        >
                    </div>
                {:else}
                    <span
                        class="text-xs font-bold text-red-600 uppercase tracking-wider"
                        >Failed</span
                    >
                {/if}
                <div
                    class={cn(
                        "text-xs font-medium flex items-center gap-1",
                        isSuccess ? "text-green-900" : "text-red-900",
                    )}
                >
                    <span class="opacity-70">Driver:</span>
                    <span>{driverName}</span>
                </div>
            </div>

            <!-- Divider -->
            <div
                class={cn(
                    "h-px w-full my-3",
                    isSuccess ? "bg-green-200" : "bg-red-200",
                )}
            ></div>

            <!-- Row 2: Version | Driver -->
            <div
                class={cn(
                    "text-xs font-medium flex items-center gap-1",
                    isSuccess ? "text-green-900" : "text-red-900",
                )}
            >
                {#if isSuccess}
                    <div class="flex items-center gap-1 w-full justify-center">
                        <span class="opacity-70">Version:</span>
                        <span>{result.version || "Unknown"}</span>
                    </div>
                {:else}
                    <span class="opacity-90"
                        >{result.error || "Unknown Error"}</span
                    >
                {/if}
            </div>
        </div>
    </div>

    <!-- Seamless Arrow -->
    <div
        class={cn(
            "absolute -bottom-[8px] left-1/2 -translate-x-1/2 size-4 bg-red-500 border-r border-b rotate-45 z-20",
            isSuccess
                ? "bg-[#ecfdf5] border-green-200"
                : "bg-[#fef2f2] border-red-200",
        )}
    ></div>
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
