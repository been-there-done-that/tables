<script lang="ts">
    import { fly, fade } from "svelte/transition";
    import { backOut } from "svelte/easing";
    import {
        IconCheck,
        IconX,
        IconAlertCircle,
        IconInfoCircle,
        IconX as IconClose,
    } from "@tabler/icons-svelte";
    import type { NotificationItem as NotificationItemType } from "$lib/utils/notification.svelte";
    import { notifications } from "$lib/utils/notification.svelte";

    interface Props {
        item: NotificationItemType;
    }

    let { item }: Props = $props();

    const icons = {
        success: IconCheck,
        error: IconX,
        warning: IconAlertCircle,
        info: IconInfoCircle,
    };

    const colors = {
        success: "text-green-500 bg-green-500/20 border-green-500/30",
        error: "text-red-500 bg-red-500/20 border-red-500/30",
        warning: "text-amber-500 bg-amber-500/20 border-amber-500/30",
        info: "text-blue-500 bg-blue-500/20 border-blue-500/30",
    };

    const Icon = $derived(icons[item.type]);
</script>

<div
    class="relative flex items-center w-full max-w-sm p-4 overflow-hidden border rounded-lg shadow-xl pointer-events-auto bg-[--theme-bg-secondary] border-[--theme-border-default] group"
    in:fly={{ y: -100, duration: 400, easing: backOut }}
    out:fade={{ duration: 200 }}
>
    <!-- Accent line - thicker for better visibility -->
    <div
        class="absolute top-0 left-0 w-1.5 h-full {item.type === 'success'
            ? 'bg-green-500'
            : item.type === 'error'
              ? 'bg-red-500'
              : item.type === 'warning'
                ? 'bg-amber-500'
                : 'bg-blue-500'}"
    ></div>

    <div class="flex items-start grow min-w-0">
        <div class="shrink-0 mr-3 {colors[item.type]} p-2 rounded-lg border">
            <Icon size={20} stroke={2.5} />
        </div>

        <div class="grow min-w-0 py-0.5">
            {#if item.component}
                {@render item.component()}
            {:else}
                <p
                    class="text-sm font-semibold text-[--theme-fg-primary] leading-relaxed wrap-break-word"
                >
                    {item.message}
                </p>
            {/if}
        </div>
    </div>

    {#if item.dismissible}
        <button
            class="shrink-0 ml-4 p-1 rounded-md text-[--theme-fg-tertiary] hover:text-[--theme-fg-primary] hover:bg-[--theme-bg-tertiary] transition-colors focus:outline-none"
            onclick={() => notifications.dismiss(item.id)}
            aria-label="Close notification"
        >
            <IconClose size={16} stroke={2} />
        </button>
    {/if}
</div>

<style>
    /* Add a subtle glow based on type */
    div {
        backdrop-filter: blur(8px);
    }
</style>
